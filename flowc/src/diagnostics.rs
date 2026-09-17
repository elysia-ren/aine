//! Deterministic, structured diagnostics (design doc s57).
//!
//! Every diagnostic carries:
//! - Error Code (stable, e.g. F1001)
//! - standard term + human explanation (人话解释)
//! - file/line/col + source snippet
//! - cause + suggestion + quick-fix hint

use std::fmt;

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// A structured diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Stable error code, e.g. "F1001".
    pub code: String,
    pub severity: Severity,
    /// Standard (technical) term.
    pub term: String,
    /// Human explanation (人话解释).
    pub message: String,
    /// Optional cause chain explanation.
    pub cause: Option<String>,
    /// Optional suggestion / remediation.
    pub suggestion: Option<String>,
    /// Optional quick-fix hint (IDE can turn this into an edit).
    pub quick_fix: Option<String>,
    /// Location in source (if known).
    pub line: u32,
    pub col: u32,
    /// Byte span in source (if known).
    pub span: Option<(usize, usize)>,
}

impl Diagnostic {
    pub fn new(
        code: &str,
        severity: Severity,
        term: &str,
        message: &str,
        line: u32,
        col: u32,
    ) -> Self {
        Diagnostic {
            code: code.to_string(),
            severity,
            term: term.to_string(),
            message: message.to_string(),
            cause: None,
            suggestion: None,
            quick_fix: None,
            line,
            col,
            span: None,
        }
    }

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_quick_fix(mut self, quick_fix: impl Into<String>) -> Self {
        self.quick_fix = Some(quick_fix.into());
        self
    }

    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.span = Some((start, end));
        self
    }
}

/// Sink that collects diagnostics (deterministic order: emitted order).
#[derive(Debug, Default)]
pub struct DiagnosticSink {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        DiagnosticSink { diagnostics: Vec::new() }
    }

    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }
}

/// Lexer error codes (F1xxx range).
pub mod codes {
    pub const UNTERMINATED_STRING: &str = "F1001";
    pub const UNTERMINATED_BLOCK_COMMENT: &str = "F1002";
    pub const INVALID_CHARACTER: &str = "F1003";
    pub const INVALID_NUMBER: &str = "F1004";
    pub const INVALID_ESCAPE: &str = "F1005";

    // Parser error codes (P2xxx range)
    pub const UNEXPECTED_TOKEN: &str = "P2001";
    pub const EXPECTED: &str = "P2002";

    // Semantic / name-resolution codes (N3xxx range)
    pub const UNDEFINED_NAME: &str = "N3001";
    pub const DUPLICATE_DEFINITION: &str = "N3002";
    pub const UNKNOWN_TYPE: &str = "N3003";

    // Type-check codes (T3xxx range; spec: Flow_Type_System.md)
    pub const LET_TYPE_MISMATCH: &str = "T3001";
    pub const BINOP_TYPE_MISMATCH: &str = "T3002";
    pub const NON_BOOL_CONDITION: &str = "T3003";
    pub const RETURN_TYPE_MISMATCH: &str = "T3004";
    pub const ASSIGN_TYPE_MISMATCH: &str = "T3005";
    pub const UNKNOWN_FIELD: &str = "T3006";
    pub const FIELD_TYPE_MISMATCH: &str = "T3007";
    pub const ARG_COUNT_MISMATCH: &str = "T3008";
    pub const DUPLICATE_FIELD: &str = "T3009";
    pub const ASSIGN_TO_IMMUTABLE: &str = "T3010";
    pub const BAD_TRY: &str = "T3011";
}

/// Render a diagnostic into a rustc-style text block for the CLI.
/// Follows design doc s57: file/line/col + source snippet + cause + suggestion.
pub fn render(diag: &Diagnostic, source: &str, file: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{}[{}]: {} ({})\n",
        diag.severity, diag.code, diag.message, diag.term
    ));
    out.push_str(&format!(" --> {}:{}:{}\n", file, diag.line, diag.col));

    // Source snippet: show the line, with a caret under the span.
    if let Some((start0, end0)) = diag.span {
        let start = floor_char_boundary(source, start0);
        let end = ceil_char_boundary(source, end0);
        let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        // 片段永远限制在 start 所在行内（跨行 span 只显示首行——
        // 避免 let/if 等"下一个 token end"式的宽 span 吞进后续行）
        let line_end = source[start.min(source.len())..]
            .find('\n')
            .map(|i| start + i)
            .unwrap_or(source.len());
        let end = end.clamp(line_start, line_end);
        let line_text: String = source[line_start..line_end].chars().collect();
        out.push_str(&format!("  |\n"));
        out.push_str(&format!("{} | {}\n", diag.line, line_text));
        // caret column: count chars before span start on this line
        let caret_col = source[line_start..start].chars().count();
        let width = source[start..end.min(source.len())].chars().count().max(1);
        let pad = " ".repeat(caret_col);
        let carets = "^".repeat(width);
        out.push_str(&format!("  | {}{}\n", pad, carets));
        out.push_str(&format!("  |\n"));
    }
    if let Some(cause) = &diag.cause {
        out.push_str(&format!("  = 原因: {}\n", cause));
    }
    if let Some(suggestion) = &diag.suggestion {
        out.push_str(&format!("  = 建议: {}\n", suggestion));
    }
    if let Some(qf) = &diag.quick_fix {
        out.push_str(&format!("  = Quick Fix: {}\n", qf));
    }
    out
}

/// Align a byte index down to the nearest UTF-8 char boundary.
fn floor_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Align a byte index up to the nearest UTF-8 char boundary.
fn ceil_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        return s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_fields_are_set() {
        let d = Diagnostic::new("F1001", Severity::Error, "unterminated string", "字符串没有闭合", 3, 15)
            .with_cause("读到行尾仍未遇到闭合引号")
            .with_suggestion("在字符串末尾补上闭合引号")
            .with_quick_fix("自动补全: 在行尾添加一个闭合引号");
        assert_eq!(d.code, "F1001");
        assert!(d.cause.is_some());
        assert!(d.suggestion.is_some());
        assert!(d.quick_fix.is_some());
    }

    #[test]
    fn sink_counts_errors() {
        let mut sink = DiagnosticSink::new();
        sink.push(Diagnostic::new("F1001", Severity::Error, "x", "m", 1, 1));
        sink.push(Diagnostic::new("F2001", Severity::Warning, "x", "m", 1, 1));
        assert!(sink.has_errors());
        assert_eq!(sink.error_count(), 1);
    }
}
