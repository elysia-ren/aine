//! Lexer for the Aine language.
//!
//! Produces a token stream with source spans and locations.
//! Diagnostics follow design doc s57 (deterministic, structured, explainable)
//! and use the F1xxx code range (see diagnostics::codes).

use crate::diagnostics::{codes, Diagnostic, DiagnosticSink, Severity};
use crate::token::{keyword_from_ident, Loc, Span, Token, TokenKind, TokenValue};

/// Result of lexing: tokens (including trailing Eof) plus diagnostics.
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: DiagnosticSink,
}

/// The lexer.
pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize, // byte offset
    line: u32,
    col: u32, // 1-based char column on the current line
    sink: DiagnosticSink,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            sink: DiagnosticSink::new(),
        }
    }

    /// Lex the whole source. Always ends with an Eof token.
    pub fn lex(mut self) -> Lexed {
        // skip a leading UTF-8 BOM if present
        if self.src.starts_with('\u{feff}') {
            self.pos = '\u{feff}'.len_utf8();
            self.col = 1;
        }
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia();
            let loc = Loc { line: self.line, col: self.col };
            let start = self.pos;
            let kind = self.next_token_kind();
            let end = self.pos;
            let text = self.src[start..end].to_string();
            let token = match kind {
                None => {
                    // EOF reached
                    tokens.push(Token::new(TokenKind::Eof, Span::new(start, start), loc, String::new(), TokenValue::None));
                    break;
                }
                Some(TokenKind::Str) => {
                    let value = TokenValue::Str(self.decode_string(start, end));
                    Token::new(TokenKind::Str, Span::new(start, end), loc, text, value)
                }
                Some(TokenKind::FmtStr) => {
                    let inner_start = start + 1; // skip 'f'
                    let value = TokenValue::Str(self.decode_string(inner_start, end));
                    Token::new(TokenKind::FmtStr, Span::new(start, end), loc, text, value)
                }
                Some(TokenKind::Int) => {
                    let value = self.parse_int(start, end);
                    Token::new(TokenKind::Int, Span::new(start, end), loc, text, value)
                }
                Some(TokenKind::Float) => {
                    let raw = &self.src[start..end];
                    let v: f64 = raw.parse().unwrap_or(0.0);
                    Token::new(TokenKind::Float, Span::new(start, end), loc, text, TokenValue::Float(v))
                }
                Some(kind) => {
                    Token::new(kind, Span::new(start, end), loc, text, TokenValue::None)
                }
            };
            tokens.push(token);
        }
        Lexed { tokens, diagnostics: self.sink }
    }

    // ---- helpers ----

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    /// Width in bytes of the UTF-8 char whose lead byte is given.
    fn char_width(lead: u8) -> usize {
        if lead < 0x80 {
            1
        } else if lead >= 0xF0 {
            4
        } else if lead >= 0xE0 {
            3
        } else if lead >= 0xC0 {
            2
        } else {
            1
        }
    }

    /// Advance one full UTF-8 character (col counts chars, not bytes).
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        let width = Self::char_width(b);
        self.pos += width;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    /// Skip whitespace and comments.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\r') => {
                    self.bump();
                }
                Some(b'\n') => {
                    self.bump();
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    // line comment
                    while let Some(b) = self.peek() {
                        if b == b'\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some(b'/') if self.peek2() == Some(b'*') => {
                    // block comment (nested)
                    self.bump(); // '/'
                    self.bump(); // '*'
                    let (line, col) = (self.line, self.col);
                    let start = self.pos;
                    let mut depth = 1usize;
                    while depth > 0 {
                        match (self.peek(), self.peek2()) {
                            (Some(b'/'), Some(b'*')) => {
                                self.bump();
                                self.bump();
                                depth += 1;
                            }
                            (Some(b'*'), Some(b'/')) => {
                                self.bump();
                                self.bump();
                                depth -= 1;
                            }
                            (Some(_), _) => {
                                self.bump();
                            }
                            (None, _) => {
                                self.sink.push(
                                    Diagnostic::new(
                                        codes::UNTERMINATED_BLOCK_COMMENT,
                                        Severity::Error,
                                        "unterminated block comment",
                                        "块注释没有闭合",
                                        line,
                                        col,
                                    )
                                    .with_cause("注释以 /* 开始，但未找到匹配的 */")
                                    .with_suggestion("在注释末尾补上 */")
                                    .with_span(start, self.pos),
                                );
                                return;
                            }
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn is_ident_start(b: u8) -> bool {
        b.is_ascii_alphabetic() || b == b'_'
    }

    fn is_ident_continue(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    /// Scan the next token kind (advancing pos), or None at EOF.
    fn next_token_kind(&mut self) -> Option<TokenKind> {
        use TokenKind::*;
        let b = self.peek()?;

        // f"..." format string: identifier 'f' directly followed by '"'
        // (must be checked before generic identifiers)
        if b == b'f' && self.peek2() == Some(b'"') {
            self.bump(); // 'f'
            self.scan_fmt_string_body();
            return Some(FmtStr);
        }

        // identifiers / keywords
        if Self::is_ident_start(b) {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if Self::is_ident_continue(c) {
                    self.bump();
                } else {
                    break;
                }
            }
            let ident = &self.src[start..self.pos];
            return Some(keyword_from_ident(ident).unwrap_or(Ident));
        }

        // numbers
        if b.is_ascii_digit() {
            return Some(self.scan_number());
        }

        // string literal
        if b == b'"' {
            self.scan_string_body();
            return Some(Str);
        }

        // punctuation / operators
        match b {
            b'(' => { self.bump(); Some(LParen) }
            b')' => { self.bump(); Some(RParen) }
            b'{' => { self.bump(); Some(LBrace) }
            b'}' => { self.bump(); Some(RBrace) }
            b'[' => { self.bump(); Some(LBracket) }
            b']' => { self.bump(); Some(RBracket) }
            b',' => { self.bump(); Some(Comma) }
            b';' => { self.bump(); Some(Semi) }
            b'@' => { self.bump(); Some(At) }
            b'#' => {
                // G2.0 ①：'#' 已无语言用途（属性统一 @），词法报错
                self.bump();
                let (line, col) = (self.line, self.col);
                self.sink.push(
                    crate::diagnostics::Diagnostic::new(
                        crate::diagnostics::codes::INVALID_CHARACTER,
                        crate::diagnostics::Severity::Error,
                        "invalid character",
                        "无法识别的字符 '#'（属性已统一为 @ 形态）",
                        line,
                        col,
                    )
                    .with_suggestion("使用 @name 或 @name(args) 形式书写属性")
                    .with_span(self.pos.saturating_sub(1), self.pos),
                );
                Some(Hash)
            }
            b'?' => { self.bump(); Some(Question) }
            b'~' => { self.bump(); Some(Tilde) }
            b'^' => { self.bump(); Some(Caret) }
            b'&' => {
                self.bump();
                if self.peek() == Some(b'&') { self.bump(); Some(AndAnd) } else { Some(Ampersand) }
            }
            b'|' => {
                self.bump();
                if self.peek() == Some(b'|') { self.bump(); Some(OrOr) } else { Some(Pipe) }
            }
            b'=' => {
                self.bump();
                match self.peek() {
                    Some(b'=') => { self.bump(); Some(EqEq) }
                    Some(b'>') => { self.bump(); Some(FatArrow) }
                    _ => Some(Eq),
                }
            }
            b'!' => {
                self.bump();
                if self.peek() == Some(b'=') { self.bump(); Some(NotEq) } else { Some(Bang) }
            }
            b'<' => {
                self.bump();
                match self.peek() {
                    Some(b'=') => { self.bump(); Some(Le) }
                    Some(b'<') => { self.bump(); Some(Shl) }
                    _ => Some(Lt),
                }
            }
            b'>' => {
                self.bump();
                match self.peek() {
                    Some(b'=') => { self.bump(); Some(Ge) }
                    Some(b'>') => { self.bump(); Some(Shr) }
                    _ => Some(Gt),
                }
            }
            b'+' => {
                self.bump();
                if self.peek() == Some(b'=') { self.bump(); Some(PlusEq) } else { Some(Plus) }
            }
            b'-' => {
                self.bump();
                match self.peek() {
                    Some(b'=') => { self.bump(); Some(MinusEq) }
                    Some(b'>') => { self.bump(); Some(Arrow) }
                    _ => Some(Minus),
                }
            }
            b'*' => {
                self.bump();
                if self.peek() == Some(b'=') { self.bump(); Some(StarEq) } else { Some(Star) }
            }
            b'/' => {
                self.bump();
                if self.peek() == Some(b'=') { self.bump(); Some(SlashEq) } else { Some(Slash) }
            }
            b'%' => {
                self.bump();
                if self.peek() == Some(b'=') { self.bump(); Some(PercentEq) } else { Some(Percent) }
            }
            b':' => {
                self.bump();
                if self.peek() == Some(b':') { self.bump(); Some(ColonColon) } else { Some(Colon) }
            }
            b'.' => {
                self.bump();
                if self.peek() == Some(b'.') { self.bump(); Some(DotDot) } else { Some(Dot) }
            }
            other => {
                let (line, col) = (self.line, self.col);
                let start = self.pos;
                self.bump();
                self.sink.push(
                    Diagnostic::new(
                        codes::INVALID_CHARACTER,
                        Severity::Error,
                        "invalid character",
                        &format!("无法识别的字符 '{}'", char::from(other)),
                        line,
                        col,
                    )
                    .with_suggestion("删除该字符，或使用正确的 Aine 语法")
                    .with_span(start, self.pos),
                );
                // skip the bad char and continue (skip trivia first)
                self.skip_trivia();
                return self.next_token_kind();
            }
        }
    }

    /// Scan a numeric literal: decimal int/float, hex, binary, octal.
    fn scan_number(&mut self) -> TokenKind {
        let start = self.pos;
        // 0x / 0b / 0o prefixes
        if self.peek() == Some(b'0') {
            match self.peek2() {
                Some(b'x') | Some(b'X') => {
                    self.bump();
                    self.bump();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_hexdigit() || c == b'_' {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    self.check_radix_body(start, "0x");
                    return TokenKind::Int;
                }
                Some(b'b') | Some(b'B') => {
                    self.bump();
                    self.bump();
                    while let Some(c) = self.peek() {
                        if c == b'0' || c == b'1' || c == b'_' {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    self.check_radix_body(start, "0b");
                    return TokenKind::Int;
                }
                Some(b'o') | Some(b'O') => {
                    self.bump();
                    self.bump();
                    while let Some(c) = self.peek() {
                        if (b'0'..=b'7').contains(&c) || c == b'_' {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                    self.check_radix_body(start, "0o");
                    return TokenKind::Int;
                }
                _ => {}
            }
        }
        // decimal digits
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'_' {
                self.bump();
            } else {
                break;
            }
        }
        // float: '.' followed by digit (not '..'), or exponent
        let mut is_float = false;
        if self.peek() == Some(b'.') && self.peek2().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            is_float = true;
            self.bump(); // '.'
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == b'_' {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        if self.peek() == Some(b'e') || self.peek() == Some(b'E') {
            // exponent: e[+-]?digits
            let save = (self.pos, self.line, self.col);
            self.bump();
            if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                self.bump();
            }
            let mut has_digit = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    has_digit = true;
                    self.bump();
                } else {
                    break;
                }
            }
            if has_digit {
                is_float = true;
            } else {
                // not an exponent after all; restore
                (self.pos, self.line, self.col) = save;
            }
        }
        if is_float {
            TokenKind::Float
        } else {
            TokenKind::Int
        }
    }

    /// Emit INVALID_NUMBER if a radix-prefixed literal has an empty body.
    fn check_radix_body(&mut self, start: usize, prefix: &str) {
        let body = &self.src[start + prefix.len()..self.pos];
        if body.is_empty() || body.chars().all(|c| c == '_') {
            let (line, col) = (self.line, self.col);
            self.sink.push(
                Diagnostic::new(
                    codes::INVALID_NUMBER,
                    Severity::Error,
                    "invalid number literal",
                    &format!("无效的数字字面量 '{}'", &self.src[start..self.pos]),
                    line,
                    col,
                )
                .with_suggestion("补齐数字位，例如 0xFF")
                .with_span(start, self.pos),
            );
        }
    }

    /// Scan a string body after the opening quote has been consumed.
    /// On unterminated string, emit F1001 and stop.
    fn scan_string_body(&mut self) {
        if self.peek() == Some(b'"') {
            self.bump(); // opening quote
        }
        loop {
            match self.peek() {
                Some(b'"') => {
                    self.bump();
                    return;
                }
                Some(b'\\') => {
                    self.bump();
                    if self.peek().is_some() {
                        self.bump();
                    }
                }
                Some(b'\n') => {
                    let (line, col) = (self.line, self.col);
                    let start = self.pos.saturating_sub(1);
                    self.sink.push(
                        Diagnostic::new(
                            codes::UNTERMINATED_STRING,
                            Severity::Error,
                            "unterminated string literal",
                            "字符串没有闭合",
                            line,
                            col,
                        )
                        .with_cause("字符串跨行且没有遇到闭合引号")
                        .with_suggestion("在字符串末尾补上闭合引号")
                        .with_quick_fix("自动补全: 在行尾添加闭合引号")
                        .with_span(start, self.pos),
                    );
                    return;
                }
                Some(_) => {
                    self.bump();
                }
                None => {
                    let (line, col) = (self.line, self.col);
                    let start = self.pos.saturating_sub(1);
                    self.sink.push(
                        Diagnostic::new(
                            codes::UNTERMINATED_STRING,
                            Severity::Error,
                            "unterminated string literal",
                            "字符串没有闭合",
                            line,
                            col,
                        )
                        .with_suggestion("在字符串末尾补上闭合引号")
                        .with_span(start, self.pos),
                    );
                    return;
                }
            }
        }
    }

    /// f-string 专用扫描：插值 `{...}` 内允许嵌套字符串（含嵌套引号）。
    /// 普通字符串仍走 scan_string_body（引号即终止）。
    fn scan_fmt_string_body(&mut self) {
        if self.peek() == Some(b'"') {
            self.bump(); // opening quote
        }
        let mut brace_depth: usize = 0;
        loop {
            match self.peek() {
                Some(b'"') => {
                    if brace_depth == 0 {
                        self.bump();
                        return;
                    }
                    // 插值内的嵌套字符串：消费到它的闭合引号（支持转义）
                    self.bump();
                    loop {
                        match self.peek() {
                            Some(b'\\') => {
                                self.bump();
                                if self.peek().is_some() {
                                    self.bump();
                                }
                            }
                            Some(b'"') => {
                                self.bump();
                                break;
                            }
                            Some(b'\n') | None => break, // 未闭合嵌套串：交回外层统一报错
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                Some(b'{') => {
                    brace_depth += 1;
                    self.bump();
                }
                Some(b'}') => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                    self.bump();
                }
                Some(b'\\') => {
                    self.bump();
                    if self.peek().is_some() {
                        self.bump();
                    }
                }
                Some(b'\n') => {
                    let (line, col) = (self.line, self.col);
                    let start = self.pos.saturating_sub(1);
                    self.sink.push(
                        Diagnostic::new(
                            codes::UNTERMINATED_STRING,
                            Severity::Error,
                            "unterminated string literal",
                            "字符串没有闭合",
                            line,
                            col,
                        )
                        .with_cause("字符串跨行且没有遇到闭合引号")
                        .with_suggestion("在字符串末尾补上闭合引号")
                        .with_quick_fix("自动补全: 在行尾添加闭合引号")
                        .with_span(start, self.pos),
                    );
                    return;
                }
                Some(_) => {
                    self.bump();
                }
                None => {
                    let (line, col) = (self.line, self.col);
                    let start = self.pos.saturating_sub(1);
                    self.sink.push(
                        Diagnostic::new(
                            codes::UNTERMINATED_STRING,
                            Severity::Error,
                            "unterminated string literal",
                            "字符串没有闭合",
                            line,
                            col,
                        )
                        .with_suggestion("在字符串末尾补上闭合引号")
                        .with_span(start, self.pos),
                    );
                    return;
                }
            }
        }
    }

    /// Decode a string literal body (between the quotes) into its value.
    fn decode_string(&self, start: usize, end: usize) -> String {
        let raw = &self.src[start..end];
        let mut chars = raw.chars();
        // strip leading quote
        if chars.clone().next() == Some('"') {
            chars.next();
        }
        let mut body: Vec<char> = chars.collect();
        if body.last() == Some(&'"') {
            body.pop();
        }
        let mut s = String::new();
        let mut i = 0;
        while i < body.len() {
            let c = body[i];
            if c == '\\' && i + 1 < body.len() {
                let e = body[i + 1];
                match e {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '0' => s.push('\0'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    '\'' => s.push('\''),
                    'x' => {
                        // \xHH
                        if i + 3 < body.len() {
                            let hi = body[i + 2].to_digit(16);
                            let lo = body[i + 3].to_digit(16);
                            if let (Some(h), Some(l)) = (hi, lo) {
                                let v = (h * 16 + l) as u8;
                                if v.is_ascii() {
                                    s.push(v as char);
                                }
                                i += 4;
                                continue;
                            }
                        }
                        s.push('\\');
                        s.push('x');
                        i += 2;
                        continue;
                    }
                    other => {
                        s.push('\\');
                        s.push(other);
                    }
                }
                i += 2;
            } else {
                s.push(c);
                i += 1;
            }
        }
        s
    }

    /// Parse an integer literal (handles prefixes and underscores).
    /// Out-of-range values fall back to Int(0) with an INVALID_NUMBER diagnostic.
    fn parse_int(&mut self, start: usize, end: usize) -> TokenValue {
        let text = &self.src[start..end];
        let clean: String = text.chars().filter(|c| *c != '_').collect();
        let value = if let Some(hex) = clean.strip_prefix("0x").or_else(|| clean.strip_prefix("0X")) {
            i64::from_str_radix(hex, 16)
        } else if let Some(bin) = clean.strip_prefix("0b").or_else(|| clean.strip_prefix("0B")) {
            i64::from_str_radix(bin, 2)
        } else if let Some(oct) = clean.strip_prefix("0o").or_else(|| clean.strip_prefix("0O")) {
            i64::from_str_radix(oct, 8)
        } else {
            clean.parse::<i64>()
        };
        match value {
            Ok(v) => TokenValue::Int(v),
            Err(_) => {
                // Empty radix bodies (e.g. "0x") are already diagnosed by
                // check_radix_body; avoid a duplicate diagnostic here.
                let radix_prefix = clean.starts_with("0x")
                    || clean.starts_with("0X")
                    || clean.starts_with("0b")
                    || clean.starts_with("0B")
                    || clean.starts_with("0o")
                    || clean.starts_with("0O");
                let empty_radix = radix_prefix && clean.len() >= 2 && clean[2..].is_empty();
                if !empty_radix {
                    let (line, col) = (self.line, self.col);
                    self.sink.push(
                        Diagnostic::new(
                            codes::INVALID_NUMBER,
                            Severity::Error,
                            "integer literal out of range",
                            &format!("整数超出范围: '{}'", text),
                            line,
                            col,
                        )
                        .with_suggestion("使用更小的整数，或改用 i64 可表示的范围")
                        .with_span(start, end),
                    );
                }
                TokenValue::Int(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind::*;

    fn lex_all(src: &str) -> (Vec<TokenKind>, Vec<Diagnostic>) {
        let lexed = Lexer::new(src).lex();
        let kinds = lexed.tokens.iter().map(|t| t.kind).collect();
        (kinds, lexed.diagnostics.diagnostics)
    }

    #[test]
    fn lexes_basic_program() {
        let (kinds, diags) = lex_all("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        assert_eq!(
            kinds,
            vec![
                KwFn, Ident, LParen, Ident, Colon, Ident, Comma, Ident, Colon, Ident,
                RParen, Arrow, Ident, LBrace, Ident, Plus, Ident, RBrace, Eof
            ]
        );
    }

    #[test]
    fn lexes_keywords_and_literals() {
        let (kinds, diags) = lex_all("var count = 0x1F");
        assert!(diags.is_empty());
        assert_eq!(kinds, vec![KwVar, Ident, Eq, Int, Eof]);
    }

    #[test]
    fn lexes_string_and_fmt_string() {
        let (kinds, diags) = lex_all("let s = \"hi\"\nlet f = f\"{count}\"");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(kinds, vec![KwLet, Ident, Eq, Str, KwLet, Ident, Eq, FmtStr, Eof]);
    }

    #[test]
    fn lexes_float_and_range() {
        let (kinds, diags) = lex_all("for i in 0..10 { let x = 1.5 }");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![
                KwFor, Ident, KwIn, Int, DotDot, Int, LBrace, KwLet, Ident, Eq, Float, RBrace, Eof
            ]
        );
    }

    #[test]
    fn lexes_operators() {
        let (kinds, diags) = lex_all("a <= b && c != d ? 1 : 2");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![
                Ident, Le, Ident, AndAnd, Ident, NotEq, Ident, Question, Int, Colon, Int, Eof
            ]
        );
    }

    #[test]
    fn skips_comments() {
        let (kinds, diags) = lex_all("// line comment\n/* block\ncomment */ let x = 1");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(kinds, vec![KwLet, Ident, Eq, Int, Eof]);
    }

    #[test]
    fn nested_block_comments() {
        let (kinds, diags) = lex_all("/* outer /* inner */ still outer */ let y = 2");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(kinds, vec![KwLet, Ident, Eq, Int, Eof]);
    }

    #[test]
    fn lexes_ui_and_decorators() {
        let (kinds, diags) = lex_all("@state\nvar count = 0\nui Counter { }");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![At, Ident, KwVar, Ident, Eq, Int, KwUi, Ident, LBrace, RBrace, Eof]
        );
    }

    #[test]
    fn lexes_go_and_go_bang() {
        let (kinds, diags) = lex_all("let t = go { work() }\ngo! { fire() }");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            kinds,
            vec![
                KwLet, Ident, Eq, KwGo, LBrace, Ident, LParen, RParen, RBrace,
                KwGo, Bang, LBrace, Ident, LParen, RParen, RBrace, Eof
            ]
        );
    }

    #[test]
    fn tracks_line_and_col() {
        let lexed = Lexer::new("let a = 1\nlet b = 2").lex();
        let toks = &lexed.tokens;
        // find 'b'
        let b_tok = toks.iter().find(|t| t.kind == Ident && t.text == "b").unwrap();
        assert_eq!(b_tok.loc.line, 2);
        assert_eq!(b_tok.loc.col, 5);
    }

    #[test]
    fn unterminated_string_reports_error() {
        let lexed = Lexer::new("let s = \"abc").lex();
        assert!(lexed.diagnostics.has_errors());
        let d = &lexed.diagnostics.diagnostics[0];
        assert_eq!(d.code, codes::UNTERMINATED_STRING);
    }

    #[test]
    fn unterminated_block_comment_reports_error() {
        let lexed = Lexer::new("/* never closed").lex();
        assert!(lexed.diagnostics.has_errors());
        assert_eq!(lexed.diagnostics.diagnostics[0].code, codes::UNTERMINATED_BLOCK_COMMENT);
    }

    #[test]
    fn invalid_char_reports_error() {
        let lexed = Lexer::new("let $ = 1").lex();
        assert!(lexed.diagnostics.has_errors());
        assert_eq!(lexed.diagnostics.diagnostics[0].code, codes::INVALID_CHARACTER);
    }

    #[test]
    fn string_escapes_decode() {
        let lexed = Lexer::new("let s = \"a\\tb\\n\\\\c\"").lex();
        let s_tok = lexed.tokens.iter().find(|t| t.kind == Str).unwrap();
        assert_eq!(s_tok.value, TokenValue::Str("a\tb\n\\c".to_string()));
    }

    #[test]
    fn hex_escape_decodes() {
        let lexed = Lexer::new("let s = \"\\x41\"").lex();
        let s_tok = lexed.tokens.iter().find(|t| t.kind == Str).unwrap();
        assert_eq!(s_tok.value, TokenValue::Str("A".to_string()));
    }

    #[test]
    fn invalid_number_prefix_reports_error() {
        let lexed = Lexer::new("let x = 0x").lex();
        assert!(lexed.diagnostics.has_errors());
        assert_eq!(lexed.diagnostics.diagnostics[0].code, codes::INVALID_NUMBER);
    }

    #[test]
    fn int_overflow_reports_error() {
        let lexed = Lexer::new("let x = 99999999999999999999").lex();
        assert!(lexed.diagnostics.has_errors());
        assert_eq!(lexed.diagnostics.diagnostics[0].code, codes::INVALID_NUMBER);
    }

    #[test]
    fn skips_utf8_bom() {
        let lexed = Lexer::new("\u{feff}let x = 1").lex();
        assert!(!lexed.diagnostics.has_errors(), "{:?}", lexed.diagnostics.diagnostics);
        let first = lexed.tokens.iter().find(|t| t.kind != TokenKind::Eof).unwrap();
        assert_eq!(first.text, "let");
    }

    #[test]
    fn multibyte_chars_in_strings() {
        let lexed = Lexer::new("let s = \"你好，世界\"").lex();
        assert!(!lexed.diagnostics.has_errors(), "{:?}", lexed.diagnostics.diagnostics);
        let s_tok = lexed.tokens.iter().find(|t| t.kind == Str).unwrap();
        assert_eq!(s_tok.value, TokenValue::Str("你好，世界".to_string()));
    }

    #[test]
    fn unterminated_string_with_multibyte_renders_without_panic() {
        let src = "let s = \"未闭合的字符串";
        let lexed = Lexer::new(src).lex();
        assert!(lexed.diagnostics.has_errors());
        let d = &lexed.diagnostics.diagnostics[0];
        let rendered = crate::diagnostics::render(d, src, "test.aine");
        assert!(rendered.contains("F1001"));
    }

    #[test]
    fn duplicate_diagnostics_suppressed_for_empty_radix() {
        let lexed = Lexer::new("let x = 0x").lex();
        let count = lexed
            .diagnostics
            .diagnostics
            .iter()
            .filter(|d| d.code == codes::INVALID_NUMBER)
            .count();
        assert_eq!(count, 1, "empty radix body must yield exactly one diagnostic");
    }
}
