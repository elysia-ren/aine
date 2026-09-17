//! aine — Aine 语言编译器核心库。
//!
//! M1 里程碑：核心语言（Phase 0）。
//! 当前实现：Lexer（词法分析）+ 结构化诊断（设计稿 §57）。

pub mod ast;
pub mod bench;
#[cfg(test)]
mod conformance_tests;
pub mod debugger;
pub mod diagnostics;
pub mod fmt;
pub mod hir;
pub mod http;
pub mod interp;
pub mod json;
#[cfg(test)]
mod interp_tests;
pub mod lexer;
pub mod lsp;
pub mod parser;
pub mod profile;
#[cfg(test)]
mod parser_tests;
pub mod resolve;
pub mod token;
pub mod typeck;
pub mod valueal;
pub mod veil;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 编译单个源文件：词法分析。
/// 返回 token 流与诊断；调用方根据诊断决定是否继续后续阶段。
pub fn lex_source(source: &str) -> lexer::Lexed {
    lexer::Lexer::new(source).lex()
}

/// 编译单个源文件：词法 + 语法分析。
/// 词法错误时返回 None（不进入语法分析）。
pub fn parse_source(source: &str, file: &str) -> Option<parser::Parsed> {
    let lexed = lexer::Lexer::new(source).lex();
    if lexed.diagnostics.has_errors() {
        return None;
    }
    Some(parser::Parser::new(&lexed.tokens, file).parse_program())
}

/// 完整前端：词法 → 语法 → HIR + 名称解析。
/// 任一词法/语法错误时返回 None。
pub struct FrontendOutput {
    pub tokens: Vec<token::Token>,
    pub program: ast::Program,
    pub resolved: resolve::Resolved,
}

pub fn frontend(source: &str, file: &str) -> Option<FrontendOutput> {
    let lexed = lexer::Lexer::new(source).lex();
    if lexed.diagnostics.has_errors() {
        return None;
    }
    let tokens = lexed.tokens;
    let parsed = parser::Parser::new(&tokens, file).parse_program();
    if parsed.diagnostics.has_errors() {
        return None;
    }
    let program = parsed.program?;
    let resolver = resolve::Resolver::new(&tokens);
    let resolved = resolver.resolve(&program);
    Some(FrontendOutput { tokens, program, resolved })
}
/// 带重试的文件读取：Windows 下杀软/索引器可能短暂持有锁（共享冲突），
/// 造成偶发读取失败或半截内容——重试消除整类"间歇性"伪故障。
fn read_to_string_retry(path: &Path) -> Result<String, String> {
    let mut last: Option<std::io::Error> = None;
    for attempt in 0..5 {
        match std::fs::read_to_string(path) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last = Some(e);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
                }
            }
        }
    }
    Err(format!("{}", last.unwrap()))
}

/// 解析文件级模块：把 `mod name;`（is_file）替换为 name.aine 的内容
/// （相对当前文件所在目录，递归加载，环路检测）。
/// 语义为自举期扁平注册：模块内顶层项以原名进入全局命名空间。
pub fn resolve_file_modules(program: &mut ast::Program, base_dir: &Path) -> Result<(), String> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    resolve_items(&mut program.items, base_dir, &mut visited)
}

/// 读取源文件并把 `mod name;` 行文本级替换为 name.aine 全文（递归、防环）。
/// 返回虚拟合并源：所有文件在同一坐标系中，span 天然不碰撞，
/// 诊断行号对应合并视图。`mod x { }` 内联形式不受影响。
pub fn read_source_with_modules(path: &Path) -> Result<String, String> {
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let root = read_to_string_retry(path)
        .map_err(|e| format!("无法读取 {}：{}", path.display(), e))?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    inline_module_text(&root, &dir, &mut visited)
}

fn inline_module_text(src: &str, dir: &Path, visited: &mut HashSet<PathBuf>) -> Result<String, String> {
    let mut out: Vec<String> = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        let name = match trimmed.strip_prefix("module ").or_else(|| trimmed.strip_prefix("mod ")).and_then(|s| s.strip_suffix(';')) {
            Some(n) if !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_') => n,
            _ => {
                out.push(line.to_string());
                continue;
            }
        };
        // 搜索路径：同目录 → 同目录../stdlib → CWD stdlib
        let mut mod_path = dir.join(format!("{}.aine", name));
        if !mod_path.exists() {
            let alt = dir.join("..").join("stdlib").join(format!("{}.aine", name));
            if alt.exists() {
                mod_path = alt;
            } else {
                let cwd_std = Path::new("stdlib").join(format!("{}.aine", name));
                if cwd_std.exists() {
                    mod_path = cwd_std;
                }
            }
        }
        let canonical = mod_path.canonicalize().unwrap_or_else(|_| mod_path.clone());
        if !visited.insert(canonical.clone()) {
            return Err(format!("模块环路：'{}' 被重复加载（{}）", name, canonical.display()));
        }
        let text = read_to_string_retry(&mod_path)
            .map_err(|e| format!("无法加载模块 '{}'（{}）：{}", name, mod_path.display(), e))?;
        let sub_dir = mod_path.parent().unwrap_or(dir).to_path_buf();
        let inlined = inline_module_text(&text, &sub_dir, visited)?;
        out.push(inlined);
    }
    Ok(out.join("
"))
}

fn resolve_items(items: &mut Vec<ast::Item>, dir: &Path, visited: &mut HashSet<PathBuf>) -> Result<(), String> {
    let mut expanded: Vec<ast::Item> = Vec::with_capacity(items.len());
    for item in items.drain(..) {
        match item {
            ast::Item::Mod(mut m) if m.is_file => {
                let path = dir.join(format!("{}.aine", m.name));
                let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
                if !visited.insert(canonical.clone()) {
                    return Err(format!("模块环路：'{}' 被重复加载（{}）", m.name, canonical.display()));
                }
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| format!("无法加载模块 '{}'（{}）：{}", m.name, path.display(), e))?;
                let sub = parse_source(&source, &path.to_string_lossy())
                    .ok_or_else(|| format!("模块 '{}' 词法错误", m.name))?
                    .program
                    .ok_or_else(|| format!("模块 '{}' 语法错误", m.name))?;
                let sub_dir = path.parent().unwrap_or(dir).to_path_buf();
                let mut sub_items = sub.items;
                resolve_items(&mut sub_items, &sub_dir, visited)?;
                m.items = sub_items;
                expanded.push(ast::Item::Mod(m));
            }
            ast::Item::Mod(mut m) => {
                resolve_items(&mut m.items, dir, visited)?;
                expanded.push(ast::Item::Mod(m));
            }
            other => expanded.push(other),
        }
    }
    *items = expanded;
    Ok(())
}
