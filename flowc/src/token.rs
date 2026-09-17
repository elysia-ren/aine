//! Token definitions for the Aine language.
//!
//! Follows the Aine language design (V5.8) core keyword list (design doc s54):
//! fn let mut const / if else for in while return break continue match /
//! struct enum type trait impl where / pub use mod / move / unsafe extern /
//! try catch / ui render. Plus: go (for go{} / go!{}), defer, import
//! (used in the canonical example, design doc s69), @state/@global decorators,
//! #[...] attributes, f"..." format strings, and the '?' try operator.

use std::fmt;

/// A byte range in the source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

/// 1-based source location (line, column in characters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loc {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Token kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ---- keywords (design doc s54) ----
    KwFn,
    KwLet,
    KwMut,
    KwConst,
    KwIf,
    KwElse,
    KwFor,
    KwIn,
    KwWhile,
    KwReturn,
    KwBreak,
    KwContinue,
    KwMatch,
    KwStruct,
    KwEnum,
    KwType,
    KwTrait,
    KwImpl,
    KwWhere,
    KwPub,
    KwUse,
    KwMod,
    KwModule,
    KwExport,
    KwVar,
    KwImport,
    KwMove,
    KwUnsafe,
    KwExtern,
    KwTry,
    KwCatch,
    KwUi,
    KwRender,
    KwGo,
    KwDefer,
    KwTrue,
    KwFalse,

    // ---- literals ----
    Ident,
    Int,
    Float,
    Str,    // "..." (String literal)
    FmtStr, // f"..." (format string literal)

    // ---- punctuation ----
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    ColonColon,
    Dot,
    DotDot,
    Arrow,    // ->
    FatArrow, // =>
    At,       // @ (decorator: @state / @global)
    Hash,     // # (attribute start: #[...])
    Question, // ? (try propagation)
    Bang,     // ! (go!{}) and boolean not

    // ---- operators ----
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    AndAnd,
    OrOr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Shl,
    Shr,

    Eof,
}

impl TokenKind {
    /// Human-readable name for diagnostics (deterministic, stable).
    pub fn describe(self) -> &'static str {
        use TokenKind::*;
        match self {
            KwFn => "keyword 'fn'",
            KwLet => "keyword 'let'",
            KwMut => "keyword 'mut'",
            KwConst => "keyword 'const'",
            KwIf => "keyword 'if'",
            KwElse => "keyword 'else'",
            KwFor => "keyword 'for'",
            KwIn => "keyword 'in'",
            KwWhile => "keyword 'while'",
            KwReturn => "keyword 'return'",
            KwBreak => "keyword 'break'",
            KwContinue => "keyword 'continue'",
            KwMatch => "keyword 'match'",
            KwStruct => "keyword 'struct'",
            KwEnum => "keyword 'enum'",
            KwType => "keyword 'type'",
            KwTrait => "keyword 'trait'",
            KwImpl => "keyword 'impl'",
            KwWhere => "keyword 'where'",
            KwPub => "keyword 'pub'",
            KwUse => "keyword 'use'",
            KwModule => "module",
            KwExport => "export",
            KwVar => "var",
            KwMod => "keyword 'mod'",
            KwImport => "keyword 'import'",
            KwMove => "keyword 'move'",
            KwUnsafe => "keyword 'unsafe'",
            KwExtern => "keyword 'extern'",
            KwTry => "keyword 'try'",
            KwCatch => "keyword 'catch'",
            KwUi => "keyword 'ui'",
            KwRender => "keyword 'render'",
            KwGo => "keyword 'go'",
            KwDefer => "keyword 'defer'",
            KwTrue => "keyword 'true'",
            KwFalse => "keyword 'false'",
            Ident => "identifier",
            Int => "integer literal",
            Float => "float literal",
            Str => "string literal",
            FmtStr => "format string literal",
            LParen => "'('",
            RParen => "')'",
            LBrace => "'{'",
            RBrace => "'}'",
            LBracket => "'['",
            RBracket => "']'",
            Comma => "','",
            Semi => "';'",
            Colon => "':'",
            ColonColon => "'::'",
            Dot => "'.'",
            DotDot => "'..'",
            Arrow => "'->'",
            FatArrow => "'=>'",
            At => "'@'",
            Hash => "'#'",
            Question => "'?'",
            Bang => "'!'",
            Plus => "'+'",
            Minus => "'-'",
            Star => "'*'",
            Slash => "'/'",
            Percent => "'%'",
            Eq => "'='",
            EqEq => "'=='",
            NotEq => "'!='",
            Lt => "'<'",
            Gt => "'>'",
            Le => "'<='",
            Ge => "'>='",
            AndAnd => "'&&'",
            OrOr => "'||'",
            PlusEq => "'+='",
            MinusEq => "'-='",
            StarEq => "'*='",
            SlashEq => "'/='",
            PercentEq => "'%='",
            Ampersand => "'&'",
            Pipe => "'|'",
            Caret => "'^'",
            Tilde => "'~'",
            Shl => "'<<'",
            Shr => "'>>'",
            Eof => "end of file",
        }
    }
}

/// Parsed literal value attached to literal tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    None,
    Int(i64),
    Float(f64),
    Str(String),
}

/// A single token with its location and source text.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub loc: Loc,
    /// Raw source text of the token (used by parser/IDE).
    pub text: String,
    /// Parsed value for literal tokens (None for others).
    pub value: TokenValue,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, loc: Loc, text: String, value: TokenValue) -> Self {
        Token { kind, span, loc, text, value }
    }
}

/// Map an identifier string to a keyword kind, if any.
pub fn keyword_from_ident(ident: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match ident {
        "fn" => KwFn,
        "let" => KwLet,
        "mut" => KwMut,
        "const" => KwConst,
        "if" => KwIf,
        "else" => KwElse,
        "for" => KwFor,
        "in" => KwIn,
        "while" => KwWhile,
        "return" => KwReturn,
        "break" => KwBreak,
        "continue" => KwContinue,
        "match" => KwMatch,
        "struct" => KwStruct,
        "enum" => KwEnum,
        "type" => KwType,
        "interface" => KwTrait,
        "implement" => KwImpl,
        "where" => KwWhere,


        "module" => KwModule,
        "var" => KwVar,
        "export" => KwExport,
        "import" => KwImport,
        "move" => KwMove,
        "unsafe" => KwUnsafe,
        "extern" => KwExtern,
        "try" => KwTry,
        "catch" => KwCatch,
        "ui" => KwUi,
        "render" => KwRender,
        "go" => KwGo,
        "defer" => KwDefer,
        "true" => KwTrue,
        "false" => KwFalse,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_table_covers_core_syntax() {
        for k in [
            "fn", "let", "mut", "const", "if", "else", "for", "in", "while", "return",
            "break", "continue", "match", "struct", "enum", "type", "interface", "implement",
            "where", "export", "import", "module", "var", "move", "unsafe", "extern", "try", "catch",
            "ui", "render", "go", "defer", "true", "false",
        ] {
            assert!(keyword_from_ident(k).is_some(), "missing keyword {k}");
        }
    }

    #[test]
    fn non_keywords_are_none() {
        assert_eq!(keyword_from_ident("name"), None);
        assert_eq!(keyword_from_ident("_"), None);
        assert_eq!(keyword_from_ident("String"), None);
    }
}
