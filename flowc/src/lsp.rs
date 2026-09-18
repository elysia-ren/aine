//! Aine Language Server (M7 §5 DoD 5)：LSP over stdio（vscode/neovim 兼容）。
//!
//! 能力：全流水线诊断推送（lex/parse/resolve/typeck/valueal）、hover（类型/
//! 函数摘要）、completion（关键词 + 符号）、definition（符号跳转）。
//! 传输：Content-Length 头 + JSON-RPC 2.0 body（LSP 标准）。

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::{resolve::Resolver, resolve_file_modules};
use crate::typeck::TypeChecker;
use crate::valueal::ValueFlow;

/// 每行起始字节偏移（LSP 位置 ↔ 字节互转）
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(src: &str) -> Self {
        let mut starts = vec![0usize];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex { starts }
    }
    /// (0-based line, 0-based char) → byte offset
    fn to_offset(&self, line: u32, ch: u32) -> usize {
        let l = line as usize;
        let base = if l < self.starts.len() { self.starts[l] } else { *self.starts.last().unwrap() };
        base + ch as usize
    }
    /// byte offset → (0-based line, 0-based char)
    fn to_pos(&self, off: usize) -> (u32, u32) {
        let mut lo = 0usize;
        let mut hi = self.starts.len();
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if self.starts[mid] <= off {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo as u32, (off - self.starts[lo]) as u32)
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// 一次分析的结果（诊断 + 符号表 + 类型 + 摘要）
struct Analysis {
    diagnostics: Vec<(String, u32, u32, Option<(usize, usize)>, String, String)>,
    symbols: Vec<(String, String, usize, usize)>, // name, kind, start, end
    types: std::collections::HashMap<usize, String>,
    summaries: Vec<String>,
    idents: Vec<(usize, usize, usize)>, // span.start, span.end, symbol index
    bindings: Vec<(usize, usize, usize)>, // def span → symbol index
}

fn analyze(src: &str, path: &str) -> Analysis {
    let mut diags = Vec::new();
    let lexed = Lexer::new(src).lex();
    for d in &lexed.diagnostics.diagnostics {
        diags.push((
            d.code.clone(),
            d.line,
            d.col,
            d.span,
            d.term.clone(),
            d.message.clone(),
        ));
    }
    let mut symbols = Vec::new();
    let mut types = std::collections::HashMap::new();
    let mut summaries = Vec::new();
    let mut idents = Vec::new();
    let mut bindings = Vec::new();

    if lexed.diagnostics.has_errors() {
        return Analysis { diagnostics: diags, symbols, types, summaries, idents, bindings };
    }
    let mut program = match Parser::new(&lexed.tokens, path).parse_program() {
        p if p.diagnostics.has_errors() => {
            for d in &p.diagnostics.diagnostics {
                diags.push((
                    d.code.clone(),
                    d.line,
                    d.col,
                    d.span,
                    d.term.clone(),
                    d.message.clone(),
                ));
            }
            return Analysis { diagnostics: diags, symbols, types, summaries, idents, bindings };
        }
        p => p.program.unwrap(),
    };
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = resolve_file_modules(&mut program, &mod_dir) {
        diags.push(("M9001".into(), 1, 1, None, "module".into(), e));
        return Analysis { diagnostics: diags, symbols, types, summaries, idents, bindings };
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    for d in &resolved.diagnostics.diagnostics {
        diags.push((
            d.code.clone(),
            d.line,
            d.col,
            d.span,
            d.term.clone(),
            d.message.clone(),
        ));
    }
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    for d in &typeck.diagnostics.diagnostics {
        diags.push((
            d.code.clone(),
            d.line,
            d.col,
            d.span,
            d.term.clone(),
            d.message.clone(),
        ));
    }
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    for d in &vf.diagnostics.diagnostics {
        diags.push((
            d.code.clone(),
            d.line,
            d.col,
            d.span,
            d.term.clone(),
            d.message.clone(),
        ));
    }
    // 符号表
    for (i, s) in resolved.program.symbols.iter().enumerate() {
        symbols.push((s.name.clone(), format!("{:?}", s.kind), s.span.start, s.span.end));
        if let Some(t) = typeck.binding_types.get(&s.id) {
            types.insert(i, t.display());
        }
    }
    for (span, sid) in &resolved.resolution.idents {
        idents.push((span.start, span.end, *sid));
    }
    for (span, sid) in &resolved.resolution.bindings {
        bindings.push((span.start, span.end, *sid));
    }
    let mut names: Vec<&String> = vf.summaries.keys().collect();
    names.sort();
    for n in names {
        summaries.push(vf.summaries.get(n).unwrap().display());
    }
    Analysis { diagnostics: diags, symbols, types, summaries, idents, bindings }
}

/// 读一个 LSP 消息（Content-Length 头 + body）
fn read_message(reader: &mut impl BufRead) -> Option<String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None; // EOF
        }
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        if let Some(v) = t.strip_prefix("Content-Length:") {
            content_length = v.trim().parse::<usize>().ok();
        }
    }
    let n = content_length?;
    let mut body = vec![0u8; n];
    reader.read_exact(&mut body).ok()?;
    Some(String::from_utf8_lossy(&body).into_owned())
}

fn json_str(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

/// (id, method, params_json)
fn parse_request(body: &str) -> (Option<String>, String, String) {
    let id = extract_json(body, "\"id\":");
    let method = extract_json(body, "\"method\":");
    let params = body
        .find("\"params\":")
        .map(|i| body[i + "\"params\":".len()..].trim().to_string())
        .unwrap_or_else(|| "null".into());
    (id, method.unwrap_or_default(), params)
}

/// 提取 "key": value（value 到下一个逗号/右括号，处理字符串）
fn extract_json(body: &str, key: &str) -> Option<String> {
    let i = body.find(key)? + key.len();
    let rest = body[i..].trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = stripped.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    match n {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        other => out.push(other),
                    }
                }
            } else if c == '"' {
                return Some(out);
            } else {
                out.push(c);
            }
        }
        Some(out)
    } else {
        let end = rest.find(|c: char| c == ',' || c == '}' || c == ']').unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

fn lsp_position(pos: (u32, u32)) -> String {
    format!("{{\"line\":{},\"character\":{}}}", pos.0, pos.1)
}

/// IDE API: get hover info for byte offset in source. Returns None if no info.
pub fn ide_hover(src: &str, path: &str, byte_offset: usize) -> Option<String> {
    let a = analyze(src, path);
    let idx = LineIndex::new(src);
    // Try idents first, then bindings
    for (s, e, sid) in &a.idents {
        if byte_offset >= *s && byte_offset <= *e {
            let name = a.symbols.get(*sid).map(|x| x.0.clone()).unwrap_or_default();
            for sum in &a.summaries {
                if sum.starts_with(&name) && !name.is_empty() {
                    return Some(sum.clone());
                }
            }
            if let Some(t) = a.types.get(sid) {
                return Some(format!("{}: {}", name, t));
            }
        }
    }
    for (s, e, sid) in &a.bindings {
        if byte_offset >= *s && byte_offset <= *e {
            let name = a.symbols.get(*sid).map(|x| x.0.clone()).unwrap_or_default();
            if let Some(t) = a.types.get(sid) {
                return Some(format!("{}: {}", name, t));
            }
        }
    }
    None
}

/// IDE API: get definition location for byte offset. Returns (line, col) 0-based.
pub fn ide_definition(src: &str, path: &str, byte_offset: usize) -> Option<(usize, usize)> {
    let a = analyze(src, path);
    let idx = LineIndex::new(src);
    for (s, e, sid) in &a.idents {
        if byte_offset >= *s && byte_offset <= *e {
            if let Some(sym) = a.symbols.get(*sid) {
                if sym.1 != "Builtin" {
                    let (line, col) = idx.to_pos(sym.2);
                    return Some((line as usize, col as usize));
                }
            }
        }
    }
    None
}

/// IDE API: 全工程符号搜索（workspace symbol）：名称/类别/文件/行/列
pub fn ide_workspace_symbols(query: &str, dir: &std::path::Path) -> Vec<(String, String, String, usize, usize)> {
    let mut out: Vec<(String, String, String, usize, usize)> = Vec::new();
    let q = query.to_lowercase();
    collect_ws_symbols(dir, &q, &mut out, 0);
    out.sort_by(|a, b| a.2.cmp(&b.2));
    out.truncate(200);
    out
}

fn collect_ws_symbols(dir: &std::path::Path, q: &str, out: &mut Vec<(String, String, String, usize, usize)>, depth: usize) {
    if depth > 6 || out.len() >= 200 { return; }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dn = entry.file_name().to_string_lossy().to_string();
                if dn == "target" || dn == ".git" || dn == "node_modules" { continue; }
                collect_ws_symbols(&path, q, out, depth + 1);
            } else if path.to_string_lossy().ends_with(".aine") {
                if let Ok(src) = std::fs::read_to_string(&path) {
                    let rel = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    for (name, kind, line, col) in ide_symbols(&src, &path.to_string_lossy()) {
                        if q.is_empty() || name.to_lowercase().contains(&q) {
                            out.push((name, kind, rel.clone(), line, col));
                        }
                    }
                }
            }
        }
    }
}

/// IDE API: 文档符号大纲（Outline）：名称 / 类别 / 行 / 列（0 基行列）
pub fn ide_symbols(src: &str, path: &str) -> Vec<(String, String, usize, usize)> {
    let a = analyze(src, path);
    let idx = LineIndex::new(src);
    let mut out: Vec<(String, String, usize, usize)> = a
        .symbols
        .iter()
        .map(|(name, kind, start, _end)| {
            let (line, col) = idx.to_pos(*start);
            (name.clone(), kind.clone(), line as usize, col as usize)
        })
        .collect();
    out.sort_by_key(|x| (x.2, x.3));
    out
}

/// IDE API: find all references of symbol at byte offset. Returns (line, col) list 0-based.
pub fn ide_references(src: &str, path: &str, byte_offset: usize) -> Option<Vec<(usize, usize)>> {
    let a = analyze(src, path);
    let idx = LineIndex::new(src);
    // Find symbol id at offset
    let mut target: Option<usize> = None;
    for (s, e, sid) in &a.idents {
        if byte_offset >= *s && byte_offset <= *e {
            target = Some(*sid);
            break;
        }
    }
    let sid = target?;
    let mut out = Vec::new();
    for (s, _e, id) in &a.idents {
        if *id == sid {
            let (line, col) = idx.to_pos(*s);
            out.push((line as usize, col as usize));
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// IDE API: rename symbol at byte offset. Returns new source text, or None if no symbol.
pub fn ide_rename(src: &str, path: &str, byte_offset: usize, new_name: &str) -> Option<String> {
    let a = analyze(src, path);
    let mut target: Option<usize> = None;
    for (s, e, sid) in &a.idents {
        if byte_offset >= *s && byte_offset <= *e {
            if let Some(sym) = a.symbols.get(*sid) {
                if sym.1 == "Builtin" { return None; }
            }
            target = Some(*sid);
            break;
        }
    }
    let sid = target?;
    // Collect spans, replace from end to start to keep offsets valid
    let mut spans: Vec<(usize, usize)> = a.idents.iter()
        .filter(|(_, _, id)| *id == sid)
        .map(|(s, e, _)| (*s, *e))
        .collect();
    spans.sort_by(|x, y| y.0.cmp(&x.0));
    let mut text = src.to_string();
    for (s, e) in spans {
        if s < e && e <= text.len() && text.is_char_boundary(s) && text.is_char_boundary(e) {
            text.replace_range(s..e, new_name);
        }
    }
    Some(text)
}

pub fn run() -> ExitCode {
    let stdin = std::io::stdin();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    let mut open_docs: std::collections::HashMap<String, (String, LineIndex)> =
        std::collections::HashMap::new();

    loop {
        let Some(body) = read_message(&mut reader) else { break };
        let (id, method, params) = parse_request(&body);
        match method.as_str() {
            "initialize" => {
                let resp = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"capabilities\":{{\
                     \"textDocumentSync\":1,\
                     \"hoverProvider\":true,\
                     \"completionProvider\":{{\"triggerCharacters\":[\".\",\":\"]}},\
                     \"definitionProvider\":true\
                     }},\"serverInfo\":{{\"name\":\"aine-lsp\",\"version\":\"0.1.0\"}}}}}}",
                    id.as_deref().unwrap_or("null")
                );
                write_msg(&mut out, &resp);
            }
            "initialized" | "shutdown" => {
                if method == "shutdown" {
                    let resp =
                        format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}", id.as_deref().unwrap_or("null"));
                    write_msg(&mut out, &resp);
                }
            }
            "exit" => break,
            "textDocument/didOpen" | "textDocument/didChange" => {
                // 提取 uri 与全文
                let uri = extract_json(&params, "\"uri\":")
                    .or_else(|| extract_json(&params, "\"textDocument\":{\"uri\":"))
                    .unwrap_or_default();
                let text = extract_json(&params, "\"text\":");
                if let Some(t) = text {
                    let path = uri_to_path(&uri);
                    let idx = LineIndex::new(&t);
                    let analysis = analyze(&t, &path);
                    open_docs.insert(uri.clone(), (t, idx));
                    push_diagnostics(&mut out, &uri, &analysis);
                }
            }
            "textDocument/hover" => {
                if std::env::var("AINE_LSP_DEBUG").is_ok() {
                    eprintln!("HOVER uri_raw={:?} docs={}", params, open_docs.len());
                }
                let uri_h = extract_json(&params, "\"uri\":");
                if std::env::var("AINE_LSP_DEBUG").is_ok() {
                    eprintln!("HOVER uri_extracted={:?} keys={:?}", uri_h, open_docs.keys().collect::<Vec<_>>());
                }
                let uri = extract_json(&params, "\"uri\":")
                    .or_else(|| extract_json(&params, "\"textDocument\":{\"uri\":"))
                    .unwrap_or_default();
                let (line, ch) = extract_pos(&params);
                let result = if let Some((doc, idx)) = open_docs.get(&uri) {
                    let off = idx.to_offset(line, ch);
                    let analysis = analyze(doc, &uri_to_path(&uri));
                    hover_at(&analysis, off, idx, doc)
                } else {
                    "null".to_string()
                };
                let resp = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
                    id.as_deref().unwrap_or("null"),
                    result
                );
                write_msg(&mut out, &resp);
            }
            "textDocument/completion" => {
                let uri = extract_json(&params, "\"uri\":")
                    .or_else(|| extract_json(&params, "\"textDocument\":{\"uri\":"))
                    .unwrap_or_default();
                let result = if let Some((doc, _)) = open_docs.get(&uri) {
                    let analysis = analyze(doc, &uri_to_path(&uri));
                    completion_items(&analysis)
                } else {
                    "{\"isIncomplete\":false,\"items\":[]}".to_string()
                };
                let resp = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
                    id.as_deref().unwrap_or("null"),
                    result
                );
                write_msg(&mut out, &resp);
            }
            "textDocument/definition" => {
                let uri = extract_json(&params, "\"uri\":")
                    .or_else(|| extract_json(&params, "\"textDocument\":{\"uri\":"))
                    .unwrap_or_default();
                let (line, ch) = extract_pos(&params);
                let result = if let Some((doc, idx)) = open_docs.get(&uri) {
                    let off = idx.to_offset(line, ch);
                    let analysis = analyze(doc, &uri_to_path(&uri));
                    definition_at(&analysis, off, &uri, idx)
                } else {
                    "null".to_string()
                };
                let resp = format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
                    id.as_deref().unwrap_or("null"),
                    result
                );
                write_msg(&mut out, &resp);
            }
            _ => {
                // 未知请求：返回 null（保持协议活）
                if let Some(i) = id {
                    let resp = format!("{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":null}}", i);
                    write_msg(&mut out, &resp);
                }
            }
        }
        let _ = out.flush();
    }
    ExitCode::SUCCESS
}

fn write_msg(out: &mut impl Write, body: &str) {
    let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = out.flush();
}

fn uri_to_path(uri: &str) -> String {
    uri.strip_prefix("file:///")
        .map(|s| {
            // Windows: file:///E:/... → E:/...
            s.replace("%20", " ")
        })
        .unwrap_or_else(|| uri.to_string())
}

fn extract_pos(params: &str) -> (u32, u32) {
    let line = extract_json(params, "\"line\":")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    let ch = extract_json(params, "\"character\":")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    (line, ch)
}

fn push_diagnostics(out: &mut impl Write, uri: &str, a: &Analysis) {
    let mut items = Vec::new();
    for (code, line, col, span, term, msg) in &a.diagnostics {
        let sev = if code.starts_with('F') { 2 } else { 1 }; // F=warning, 其余 error
        let (start_pos, end_pos) = match span {
            Some((s, e)) => {
                // 用行号近似: line/col 是 1-based → 0-based
                (lsp_position((line.saturating_sub(1), col.saturating_sub(1))), lsp_position((line.saturating_sub(1), col.saturating_sub(1) + 2)))
            }
            None => (lsp_position((line.saturating_sub(1), col.saturating_sub(1))), lsp_position((line.saturating_sub(1), col.saturating_sub(1) + 2))),
        };
        items.push(format!(
            "{{\"range\":{{\"start\":{},\"end\":{}}},\"severity\":{},\"code\":{},\"source\":\"aine\",\"message\":{}}}",
            start_pos,
            end_pos,
            sev,
            json_str(code),
            json_str(&format!("[{}] {} — {}", code, term, msg))
        ));
    }
    let body = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":{},\"diagnostics\":[{}]}}}}",
        json_str(uri),
        items.join(",")
    );
    write_msg(out, &body);
}

fn hover_at(a: &Analysis, off: usize, idx: &LineIndex, src: &str) -> String {
    if std::env::var("AINE_LSP_DEBUG").is_ok() {
        eprintln!("HOVERAT off={} idents={:?} types_keys={:?}", off, a.idents.iter().take(8).collect::<Vec<_>>(), a.types.keys().take(5).collect::<Vec<_>>());
    }
    // 1) 命中 ident → 符号类型 / 函数摘要
    for (s, e, sid) in &a.idents {
        if off >= *s && off <= *e {
            let name = a.symbols.get(*sid).map(|x| x.0.clone()).unwrap_or_default();
            // 函数符号 → 摘要
            for sum in &a.summaries {
                if sum.starts_with(&name) && !name.is_empty() {
                    let md = format!("```text
{}
```", sum);
                    return format!(
                        "{{\"contents\":{{\"kind\":\"markdown\",\"value\":{}}},\"range\":{{\"start\":{},\"end\":{}}}}}",
                        json_str(&md),
                        lsp_position(idx.to_pos(*s)),
                        lsp_position(idx.to_pos(*e))
                    );
                }
            }
            // 普通绑定 → 类型
            if let Some(t) = a.types.get(sid) {
                let md = format!("```aine
{}: {}
```", name, t);
                return format!(
                    "{{\"contents\":{{\"kind\":\"markdown\",\"value\":{}}},\"range\":{{\"start\":{},\"end\":{}}}}}",
                    json_str(&md),
                    lsp_position(idx.to_pos(*s)),
                    lsp_position(idx.to_pos(*e))
                );
            }
        }
    }
    // 2) 定义处（let 绑定名等）→ 类型
    for (s, e, sid) in &a.bindings {
        if off >= *s && off <= *e {
            let name = a.symbols.get(*sid).map(|x| x.0.clone()).unwrap_or_default();
            if let Some(t) = a.types.get(sid) {
                let md = format!("```aine
{}: {}
```", name, t);
                return format!(
                    "{{\"contents\":{{\"kind\":\"markdown\",\"value\":{}}},\"range\":{{\"start\":{},\"end\":{}}}}}",
                    json_str(&md),
                    lsp_position(idx.to_pos(*s)),
                    lsp_position(idx.to_pos(*e))
                );
            }
        }
    }
    "null".to_string()
}

fn completion_items(a: &Analysis) -> String {
    let keywords = [
        "let", "var", "fn", "if", "else", "while", "for", "in", "return", "match", "struct",
        "enum", "interface", "implement", "mod", "import", "go", "go!", "ui", "true", "false",
        "None", "Some", "Ok", "Err", "print", "assert", "extern", "@state", "@global", "@test",
        "@not_send",
    ];
    let mut items = Vec::new();
    for k in keywords {
        items.push(format!(
            "{{\"label\":{},\"kind\":14,\"detail\":\"keyword\"}}",
            json_str(k)
        ));
    }
    for (name, kind, _, _) in &a.symbols {
        items.push(format!(
            "{{\"label\":{},\"kind\":6,\"detail\":{}}}",
            json_str(name),
            json_str(kind)
        ));
    }
    format!("{{\"isIncomplete\":false,\"items\":[{}]}}", items.join(","))
}

fn definition_at(a: &Analysis, off: usize, uri: &str, idx: &LineIndex) -> String {
    // ident → 定义符号 → 符号 span 位置
    for (s, e, sid) in &a.idents {
        if off >= *s && off <= *e {
            if let Some(sym) = a.symbols.get(*sid) {
                if sym.1 != "Builtin" {
                    let start = idx.to_pos(sym.2);
                    let end = idx.to_pos(sym.3);
                    return format!(
                        "[{{\"uri\":{},\"range\":{{\"start\":{},\"end\":{}}}}}]",
                        json_str(uri),
                        lsp_position(start),
                        lsp_position(end)
                    );
                }
            }
            let _ = (e, uri);
        }
    }
    "null".to_string()
}
