//! Recursive-descent parser for the Aine language (design doc §55/§56).
//!
//! Grammar characteristics:
//! - statements are terminated by newline (soft terminator); leading postfix
//!   operators (. ( [ ?) continue the previous expression across lines;
//! - blocks yield a value: the last statement, if an expression, is the tail;
//! - trailing block/closure after a call: Button("...") { }, List(records) |r| {};
//! - struct literal vs block-call disambiguation: ident { field: ... } is a
//!   struct literal, ident { stmts } is a component call with body.

use crate::ast::*;
use crate::diagnostics::{codes, Diagnostic, DiagnosticSink, Severity};
use crate::token::{Span, Token, TokenKind, TokenValue};

/// Result of parsing.
pub struct Parsed {
    pub program: Option<Program>,
    pub diagnostics: DiagnosticSink,
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    sink: DiagnosticSink,
    /// split-off second `>` when a `>>` (Shr) token closes two nested generics
    pending_gt: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], _file: &'a str) -> Self {
        Parser { tokens, pos: 0, sink: DiagnosticSink::new(), pending_gt: None }
    }

    pub fn parse_program(mut self) -> Parsed {
        let start = self.cur().span.start;
        let mut items = Vec::new();
        loop {
            if self.at(TokenKind::Eof) {
                break;
            }
            match self.parse_item() {
                Some(item) => items.push(item),
                None => {
                    if self.at(TokenKind::Eof) {
                        break;
                    }
                    self.recover_item();
                }
            }
        }
        let end = self.cur().span.end;
        let program = Program { items, span: Span::new(start, end) };
        Parsed { program: Some(program), diagnostics: self.sink }
    }

    // ---- token helpers ----

    fn cur(&self) -> &Token {
        if let Some(p) = &self.pending_gt {
            return p;
        }
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)]
    }

    fn kind(&self) -> TokenKind {
        self.cur().kind
    }

    fn at(&self, k: TokenKind) -> bool {
        self.kind() == k
    }

    fn advance(&mut self) -> Token {
        if let Some(p) = self.pending_gt.take() {
            return p;
        }
        let t = self.cur().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    /// Expect one closing `>` of a generic argument list. A `>>` (Shr) token
    /// closes TWO nested levels: consume it as the outer `>` and keep the
    /// inner `>` pending so the next expect_gt consumes it.
    fn expect_gt(&mut self) -> Result<Token, ()> {
        if self.at(TokenKind::Gt) {
            return Ok(self.advance());
        }
        if self.at(TokenKind::Shr) {
            let t = self.advance();
            self.pending_gt = Some(Token {
                kind: TokenKind::Gt,
                span: Span::new(t.span.start + 1, t.span.end),
                loc: t.loc,
                text: ">".to_string(),
                value: TokenValue::None,
            });
            return Ok(Token {
                kind: TokenKind::Gt,
                span: Span::new(t.span.start, t.span.start + 1),
                loc: t.loc,
                text: ">".to_string(),
                value: TokenValue::None,
            });
        }
        self.err_expected("'>'");
        Err(())
    }

    /// True if the current token starts a new line relative to the previous token.
    fn on_new_line(&self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.cur().loc.line > self.tokens[self.pos - 1].loc.line
    }

    fn eat(&mut self, k: TokenKind) -> Option<Token> {
        if self.at(k) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn expect(&mut self, k: TokenKind, what: &str) -> Result<Token, ()> {
        if self.at(k) {
            Ok(self.advance())
        } else {
            self.err_expected(what);
            Err(())
        }
    }

    fn err_expected(&mut self, what: &str) {
        let t = self.cur().clone();
        let mut d = Diagnostic::new(
            codes::EXPECTED,
            Severity::Error,
            "expected token",
            &format!("期望 {}，但遇到了 {}", what, t.kind.describe()),
            t.loc.line,
            t.loc.col,
        );
        d = d.with_span(t.span.start, t.span.end);
        if !t.text.is_empty() {
            d = d.with_suggestion(&format!("检查此处是否应为 {}", what));
        }
        self.sink.push(d);
    }

    fn err_unexpected(&mut self, msg: &str) {
        let t = self.cur().clone();
        let d = Diagnostic::new(
            codes::UNEXPECTED_TOKEN,
            Severity::Error,
            "unexpected token",
            msg,
            t.loc.line,
            t.loc.col,
        )
        .with_span(t.span.start, t.span.end);
        self.sink.push(d);
    }

    /// Skip tokens until a statement/item boundary (heuristic recovery).
    fn recover_item(&mut self) {
        while !self.at(TokenKind::Eof) {
            match self.kind() {
                TokenKind::KwFn
                | TokenKind::KwStruct
                | TokenKind::KwEnum
                | TokenKind::KwType
                | TokenKind::KwTrait
                | TokenKind::KwImpl
                | TokenKind::KwMod
                | TokenKind::KwImport
                | TokenKind::KwUse
                | TokenKind::KwUi
                | TokenKind::At
                | TokenKind::KwPub
                | TokenKind::KwUnsafe
                | TokenKind::KwExtern => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn recover_stmt(&mut self) {
        let start = self.pos;
        while !self.at(TokenKind::Eof) {
            match self.kind() {
                TokenKind::KwLet
                | TokenKind::KwFn
                | TokenKind::KwIf
                | TokenKind::KwWhile
                | TokenKind::KwFor
                | TokenKind::KwReturn
                | TokenKind::KwBreak
                | TokenKind::KwContinue
                | TokenKind::KwDefer
                | TokenKind::KwMatch
                | TokenKind::KwTry
                | TokenKind::KwUi
                | TokenKind::KwGo
                | TokenKind::RBrace => {
                    if self.pos == start {
                        // boundary token with no progress: consume one to guarantee
                        // forward movement (prevents infinite loops)
                        self.advance();
                    }
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Is this token usable as an identifier-like name (ident or keyword)?
    fn is_name_token(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind(),
            Ident | KwFn | KwLet | KwMut | KwConst | KwIf | KwElse | KwFor | KwIn | KwWhile
                | KwReturn | KwBreak | KwContinue | KwMatch | KwStruct | KwEnum | KwType
                | KwTrait | KwImpl | KwWhere | KwPub | KwUse | KwMod | KwImport | KwMove
                | KwUnsafe | KwExtern | KwTry | KwCatch | KwUi | KwRender | KwGo | KwDefer
                | KwTrue | KwFalse
        )
    }

    // ---- items ----

    fn parse_item(&mut self) -> Option<Item> {
        use TokenKind::*;
        let start = self.cur().span.start;

        // attributes #[...] — collect, then continue parsing the item
        let mut attributes: Vec<String> = Vec::new();
        loop {
            if self.at(At) && !self.at_item_decorator() {
                // G2.0 ② 定稿：属性唯一形态 @name / @name(args)（#[...] 已移除）
                // @global / @state 引导项语法（GlobalDef/StateDecl），非属性
                if let Some(attr) = self.parse_at_attribute() {
                    attributes.push(attr);
                }
            } else {
                break;
            }
        }

        let mut is_pub = false;
        let mut is_unsafe = false;
        let mut is_extern = false;
        let mut abi: Option<String> = None;

        loop {
            if self.at(KwPub) || self.at(KwExport) {
                self.advance();
                is_pub = true;
            } else if self.at(KwUnsafe) {
                self.advance();
                is_unsafe = true;
            } else if self.at(KwExtern) {
                self.advance();
                is_extern = true;
                if let Some(t) = self.eat(TokenKind::Str) {
                    abi = Some(match &t.value {
                        crate::token::TokenValue::Str(s) => s.clone(),
                        _ => t.text.clone(),
                    });
                }
            } else {
                break;
            }
        }

        match self.kind() {
            KwImport => Some(Item::Import(self.parse_import(start))),
            KwStruct => Some(Item::Struct(self.parse_struct(start, &attributes))),
            KwEnum => Some(Item::Enum(self.parse_enum(start))),
            KwType => Some(Item::TypeAlias(self.parse_type_alias(start))),
            KwTrait => Some(Item::Trait(self.parse_trait(start))),
            KwImpl => Some(Item::Impl(self.parse_impl(start))),
            KwModule => Some(Item::Mod(self.parse_mod(start))),
            KwFn => Some(Item::Fn(self.parse_fn(start, is_pub, is_unsafe, is_extern, abi, &attributes))),
            KwUi => Some(Item::Ui(self.parse_ui(start))),
            At => {
                // @global — module-level global state (design doc §44, DDR D8)
                if let Some(item) = self.parse_global_state(start) {
                    Some(item)
                } else {
                    None
                }
            }
            RBrace if self.pos > 0 => {
                // stray closing brace — let the enclosing scope handle it
                None
            }
            _ => {
                let t = self.cur().clone();
                self.err_unexpected(&format!(
                    "顶层只能出现项目（import/struct/enum/fn/ui 等），遇到了 {}",
                    t.kind.describe()
                ));
                None
            }
        }
    }

    /// `@` 装饰器属性（G2.0 ②）：`@test`、`@display(a, b)` —— 与 #[...] 存同一 attributes 表
    /// 项级装饰器前瞻：@global let / @state let（属项语法，非属性）
    fn at_item_decorator(&self) -> bool {
        let t1 = self.tokens.get(self.pos + 1);
        let t2 = self.tokens.get(self.pos + 2);
        match (t1, t2) {
            (Some(a), Some(b)) => {
                (a.text == "global" || a.text == "state")
                    && (b.kind == TokenKind::KwLet || b.kind == TokenKind::KwVar)
            }
            _ => false,
        }
    }

    fn parse_at_attribute(&mut self) -> Option<String> {
        use TokenKind::*;
        self.expect(At, "'@'").ok()?;
        let name = self.parse_path_token();
        let mut args = Vec::new();
        if self.eat(LParen).is_some() {
            while !self.at(RParen) && !self.at(Eof) {
                if self.at(Comma) {
                    self.advance();
                    continue;
                }
                if let Some(seg) = self.parse_ident_like() {
                    args.push(seg);
                } else {
                    self.advance();
                }
            }
            let _ = self.expect(RParen, "')'");
        }
        if args.is_empty() {
            Some(name)
        } else {
            Some(format!("{} ({})", name, args.join(", ")))
        }
    }

    fn parse_ident_like(&mut self) -> Option<String> {
        if self.is_name_token() {
            let t = self.advance();
            Some(t.text.clone())
        } else {
            None
        }
    }

    fn parse_path_token(&mut self) -> String {
        let first = self.parse_ident_like().unwrap_or_default();
        let mut path = first;
        while self.at(TokenKind::ColonColon) {
            self.advance();
            if let Some(seg) = self.parse_ident_like() {
                path = format!("{}::{}", path, seg);
            }
        }
        path
    }

    fn parse_import(&mut self, start: usize) -> ImportItem {
        self.advance(); // import / use
        let mut segments = Vec::new();
        if let Some(first) = self.parse_ident_like() {
            segments.push(first);
        }
        while self.at(TokenKind::ColonColon) {
            self.advance();
            if let Some(seg) = self.parse_ident_like() {
                segments.push(seg);
            }
        }
        let end = self.cur().span.end;
        ImportItem { segments, span: Span::new(start, end) }
    }

    fn parse_generic_params(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if self.eat(TokenKind::Lt).is_some() {
            while !self.at(TokenKind::Gt) && !self.at(TokenKind::Eof) {
                if self.eat(TokenKind::Comma).is_some() {
                    continue;
                }
                if let Some(name) = self.parse_ident_like() {
                    params.push(name);
                } else {
                    self.advance();
                }
            }
            let _ = self.expect(TokenKind::Gt, "'>'");
        }
        params
    }

    fn parse_struct(&mut self, start: usize, attributes: &[String]) -> StructDef {
        self.advance(); // struct
        let name = self.parse_ident_like().unwrap_or_default();
        let generics = self.parse_generic_params();
        let mut fields = Vec::new();
        if self.eat(TokenKind::LBrace).is_some() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                if self.eat(TokenKind::Comma).is_some() {
                    continue;
                }
                let fstart = self.cur().span.start;
                let fname = match self.parse_ident_like() {
                    Some(n) => n,
                    None => {
                        self.recover_stmt();
                        continue;
                    }
                };
                let fty = if self.eat(TokenKind::Colon).is_some() {
                    match self.parse_type() {
                        Some(t) => t,
                        None => Type::Infer,
                    }
                } else {
                    Type::Infer
                };
                let fend = self.cur().span.end;
                fields.push(StructField { name: fname, ty: fty, span: Span::new(fstart, fend) });
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
        }
        let end = self.cur().span.end;
        StructDef { name, generics, fields, span: Span::new(start, end), attributes: attributes.to_vec() }
    }

    fn parse_enum(&mut self, start: usize) -> EnumDef {
        self.advance(); // enum
        let name = self.parse_ident_like().unwrap_or_default();
        let generics = self.parse_generic_params();
        let mut variants = Vec::new();
        if self.eat(TokenKind::LBrace).is_some() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                if self.eat(TokenKind::Comma).is_some() {
                    continue;
                }
                let vstart = self.cur().span.start;
                let vname = match self.parse_ident_like() {
                    Some(n) => n,
                    None => {
                        self.recover_stmt();
                        continue;
                    }
                };
                let mut payload = Vec::new();
                if self.eat(TokenKind::LParen).is_some() {
                    while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
                        if self.eat(TokenKind::Comma).is_some() {
                            continue;
                        }
                        match self.parse_type() {
                            Some(t) => payload.push(t),
                            None => {
                                self.advance();
                            }
                        }
                    }
                    let _ = self.expect(TokenKind::RParen, "')'");
                }
                let vend = self.cur().span.end;
                variants.push(EnumVariant { name: vname, payload, span: Span::new(vstart, vend) });
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
        }
        let end = self.cur().span.end;
        EnumDef { name, generics, variants, span: Span::new(start, end) }
    }

    fn parse_type_alias(&mut self, start: usize) -> TypeAlias {
        self.advance(); // type
        let name = self.parse_ident_like().unwrap_or_default();
        let _ = self.expect(TokenKind::Eq, "'='");
        let ty = self.parse_type().unwrap_or(Type::Infer);
        let end = self.cur().span.end;
        TypeAlias { name, ty, span: Span::new(start, end) }
    }

    fn parse_trait(&mut self, start: usize) -> TraitDef {
        self.advance(); // trait
        let name = self.parse_ident_like().unwrap_or_default();
        let generics = self.parse_generic_params();
        let mut methods = Vec::new();
        if self.eat(TokenKind::LBrace).is_some() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::KwFn) {
                    methods.push(self.parse_fn_sig());
                } else if self.at(TokenKind::At) {
                    self.parse_at_attribute();
                } else {
                    self.recover_stmt();
                }
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
        }
        let end = self.cur().span.end;
        TraitDef { name, generics, methods, span: Span::new(start, end) }
    }

    fn parse_impl(&mut self, start: usize) -> ImplBlock {
        self.advance(); // impl
        //   impl Trait for Type { }    — trait impl
        //   impl Type { }              — inherent impl
        let first = self.parse_path();
        let (trait_path, self_ty) = if self.at(TokenKind::KwFor) {
            self.advance();
            let st = self.parse_type().unwrap_or(Type::Infer);
            (Some(first), st)
        } else {
            (None, Type::Path(first))
        };
        let mut items = Vec::new();
        if self.eat(TokenKind::LBrace).is_some() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                match self.parse_item() {
                    Some(it) => items.push(it),
                    None => self.recover_stmt(),
                }
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
        }
        let end = self.cur().span.end;
        ImplBlock { trait_path, self_ty, items, span: Span::new(start, end) }
    }

    fn parse_mod(&mut self, start: usize) -> ModDef {
        self.advance(); // mod
        let name = self.parse_ident_like().unwrap_or_default();
        let mut items = Vec::new();
        if self.eat(TokenKind::LBrace).is_some() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                match self.parse_item() {
                    Some(it) => items.push(it),
                    None => self.recover_item(),
                }
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
            let end = self.cur().span.end;
            return ModDef { name, items, is_file: false, span: Span::new(start, end) };
        }
        // 文件模块形态：`mod name;` —— 内容由模块加载器读入
        let _ = self.expect(TokenKind::Semi, "';'");
        let end = self.cur().span.end;
        ModDef { name, items, is_file: true, span: Span::new(start, end) }
    }

    fn parse_fn_sig(&mut self) -> FnSig {
        let start = self.cur().span.start;
        self.advance(); // fn
        let name = self.parse_ident_like().unwrap_or_default();
        let generics = self.parse_generic_params();
        let params = self.parse_params();
        let ret = if self.eat(TokenKind::Arrow).is_some() {
            self.parse_type()
        } else {
            None
        };
        let end = self.cur().span.end;
        FnSig { name, generics, params, ret, span: Span::new(start, end) }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.expect(TokenKind::LParen, "'('").is_err() {
            return params;
        }
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            let pstart = self.cur().span.start;
            let is_move = self.eat(TokenKind::KwMove).is_some();
            let is_mut = self.eat(TokenKind::KwMut).is_some();
            let name = match self.parse_ident_like() {
                Some(n) => n,
                None => {
                    self.recover_stmt();
                    continue;
                }
            };
            let ty = if self.eat(TokenKind::Colon).is_some() {
                self.parse_type()
            } else {
                None
            };
            let pend = self.cur().span.end;
            params.push(Param { name, ty, is_move, is_mut, span: Span::new(pstart, pend) });
        }
        let _ = self.expect(TokenKind::RParen, "')'");
        params
    }

    fn parse_fn(
        &mut self,
        start: usize,
        is_pub: bool,
        is_unsafe: bool,
        is_extern: bool,
        abi: Option<String>,
        attributes: &[String],
    ) -> FnDef {
        let _ = abi;
        let is_catchable = attributes.iter().any(|a| a.starts_with("catchable"));
        let sig = self.parse_fn_sig();
        // extern 声明可以没有函数体（FFI：符号由外部 C 提供）
        let body = if is_extern && !self.at(TokenKind::LBrace) {
            Block { stmts: Vec::new(), tail: None, span: self.cur().span }
        } else {
            self.parse_block()
        };
        let end = self.cur().span.end;
        FnDef {
            sig,
            body,
            is_pub,
            is_unsafe,
            is_extern,
            is_catchable,
            attributes: attributes.to_vec(),
            span: Span::new(start, end),
        }
    }

    fn parse_ui(&mut self, start: usize) -> UiDef {
        self.advance(); // ui
        let name = self.parse_ident_like().unwrap_or_default();
        let mut props = Vec::new();
        let mut states = Vec::new();
        let mut render = None;
        if self.expect(TokenKind::LBrace, "'{'").is_ok() {
            while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                if self.at(TokenKind::At) {
                    // @state (lexed as '@' + identifier)
                    self.advance(); // '@'
                    let _decorator = self.parse_ident_like(); // 'state'/'global'
                    if self.at(TokenKind::KwLet) {
                        if let Some(stmt) = self.parse_let_stmt(true) {
                            if let Stmt::Let(ls) = stmt {
                                states.push(ls);
                            }
                        }
                    } else if self.at(TokenKind::KwVar) {
                        // G2.0 ①：@state var = 可变 UI 状态
                        if let Some(stmt) = self.parse_let_stmt_mut() {
                            if let Stmt::Let(mut ls) = stmt {
                                ls.is_state = true;
                                states.push(ls);
                            }
                        }
                    } else {
                        self.err_expected("关键字 'var'");
                        self.recover_stmt();
                    }
                } else if self.at(TokenKind::KwRender) {
                    self.advance();
                    render = Some(self.parse_block());
                } else if self.is_name_token() && self.peek(1).kind == TokenKind::Eq {
                    let pstart = self.cur().span.start;
                    let pname = self.advance().text.clone();
                    self.advance(); // '='
                    if let Some(value) = self.parse_expr(0) {
                        let pend = self.cur().span.end;
                        props.push(UiProp { name: pname, value, span: Span::new(pstart, pend) });
                    } else {
                        self.recover_stmt();
                    }
                } else {
                    self.recover_stmt();
                }
            }
            let _ = self.expect(TokenKind::RBrace, "'}'");
        }
        let end = self.cur().span.end;
        UiDef { name, props, states, render, span: Span::new(start, end) }
    }

    fn parse_global_state(&mut self, start: usize) -> Option<Item> {
        // @global let [mut] name [: Type] [= init]
        self.advance(); // @
        let _ = self.parse_ident_like(); // 'global' (validated later)
        // G2.0 ①：@global var name（可变）/ @global let name（不可变）
        let is_mut = if self.at(TokenKind::KwVar) {
            self.advance();
            true
        } else if self.at(TokenKind::KwLet) {
            self.advance();
            false
        } else {
            self.err_unexpected("期望 @global 后跟 var 或 let 声明");
            return None;
        };
        let name = self.parse_ident_like()?;
        let ty = if self.eat(TokenKind::Colon).is_some() {
            self.parse_type()
        } else {
            None
        };
        let init = if self.eat(TokenKind::Eq).is_some() {
            self.parse_expr(0)
        } else {
            None
        };
        let end = self.cur().span.end;
        let _ = is_mut;
        Some(Item::GlobalState(GlobalStateDef { name, ty, init, span: Span::new(start, end) }))
    }

    // ---- types ----

    fn parse_type(&mut self) -> Option<Type> {
        use TokenKind::*;
        match self.kind() {
            Ampersand => {
                self.advance();
                let mutable = self.eat(KwMut).is_some();
                let ty = self.parse_type().unwrap_or(Type::Infer);
                Some(Type::Ref { ty: Box::new(ty), mutable })
            }
            LParen => {
                self.advance();
                let mut tys = Vec::new();
                while !self.at(RParen) && !self.at(Eof) {
                    if self.eat(Comma).is_some() {
                        continue;
                    }
                    match self.parse_type() {
                        Some(t) => tys.push(t),
                        None => {
                            self.advance();
                        }
                    }
                }
                let _ = self.expect(RParen, "')'");
                Some(Type::Tuple(tys))
            }
            Ident | KwTrue | KwFalse => {
                let p = self.parse_path_with_generics();
                Some(Type::Path(p))
            }
            KwFn => {
                // fn(A, B) -> R：统一函数类型（开发者永不区分捕获/非捕获）
                self.advance(); // fn
                let _ = self.expect(LParen, "'('");
                let mut params = Vec::new();
                while !self.at(RParen) && !self.at(Eof) {
                    if self.eat(Comma).is_some() {
                        continue;
                    }
                    match self.parse_type() {
                        Some(t) => params.push(t),
                        None => {
                            self.advance();
                        }
                    }
                }
                let _ = self.expect(RParen, "')'");
                let ret = if self.eat(Arrow).is_some() {
                    Some(Box::new(self.parse_type().unwrap_or(Type::Infer)))
                } else {
                    None
                };
                Some(Type::Fn { params, ret })
            }
            _ => {
                self.err_expected("类型");
                None
            }
        }
    }

    fn parse_path(&mut self) -> Path {
        let start = self.cur().span.start;
        let mut segments = Vec::new();
        if let Some(s) = self.parse_ident_like() {
            segments.push(s);
        }
        while self.at(TokenKind::ColonColon) {
            self.advance();
            if let Some(s) = self.parse_ident_like() {
                segments.push(s);
            }
        }
        let end = self.cur().span.end;
        Path { segments, generics: Vec::new(), span: Span::new(start, end) }
    }

    fn parse_path_with_generics(&mut self) -> Path {
        let start = self.cur().span.start;
        let mut segments = Vec::new();
        let mut generics = Vec::new();
        if let Some(s) = self.parse_ident_like() {
            segments.push(s);
        }
        while self.at(TokenKind::ColonColon) {
            self.advance();
            if let Some(s) = self.parse_ident_like() {
                segments.push(s);
            }
        }
        if self.at(TokenKind::Lt) {
            self.advance();
            let mut args = Vec::new();
            while !self.at(TokenKind::Gt) && !self.at(TokenKind::Shr) && !self.at(TokenKind::Eof) {
                if self.eat(TokenKind::Comma).is_some() {
                    continue;
                }
                match self.parse_type() {
                    Some(t) => args.push(t),
                    None => {
                        self.advance();
                    }
                }
            }
            let _ = self.expect_gt();
            generics.push(args);
        }
        let end = self.cur().span.end;
        Path { segments, generics, span: Span::new(start, end) }
    }

    // ---- statements ----

    fn parse_block(&mut self) -> Block {
        let start = self.cur().span.start;
        let mut stmts = Vec::new();
        let mut tail = None;
        if self.expect(TokenKind::LBrace, "'{'").is_err() {
            return Block { stmts, tail, span: Span::new(start, start) };
        }
        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if self.on_new_line() {
                // fine — newline separates statements
            }
            match self.parse_stmt() {
                Some(Stmt::Expr(e)) if self.at(TokenKind::RBrace) => {
                    // last expression becomes the block's tail value
                    tail = Some(Box::new(*e));
                }
                Some(s) => stmts.push(s),
                None => {
                    if self.at(TokenKind::RBrace) {
                        break;
                    }
                    self.recover_stmt();
                }
            }
        }
        let _ = self.expect(TokenKind::RBrace, "'}'");
        let end = self.cur().span.end;
        Block { stmts, tail, span: Span::new(start, end) }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        use TokenKind::*;
        match self.kind() {
            KwLet => self.parse_let_stmt(false),
            KwVar => self.parse_let_stmt_mut(),
            KwReturn => {
                self.advance();
                // G2.0 ⑥：同行/续行表达式起始 → 返回值；下一行以语句关键字或块结束 → 空 return
                let value = if self.on_new_line() && !self.starts_return_value() {
                    None
                } else {
                    self.parse_expr(0)
                };
                Some(Stmt::Return(value.map(Box::new)))
            }
            KwIf => Some(Stmt::If(Box::new(self.parse_if_expr()))),
            KwWhile => {
                let start = self.cur().span.start;
                self.advance();
                let cond = Box::new(self.parse_expr_no_trailing(0).unwrap_or(Expr::Bool(false)));
                let body = self.parse_block();
                let end = self.cur().span.end;
                Some(Stmt::While { cond, body, span: Span::new(start, end) })
            }
            KwFor => {
                let start = self.cur().span.start;
                self.advance();
                let pat = self.parse_pattern().unwrap_or(Pattern::Wildcard);
                let _ = self.expect(KwIn, "关键字 'in'");
                let iter = Box::new(self.parse_expr_no_trailing(0).unwrap_or(Expr::Array(Vec::new())));
                let body = self.parse_block();
                let end = self.cur().span.end;
                Some(Stmt::For { pat, iter, body, span: Span::new(start, end) })
            }
            KwBreak => {
                let t = self.advance();
                Some(Stmt::Break(t.span))
            }
            KwContinue => {
                let t = self.advance();
                Some(Stmt::Continue(t.span))
            }
            KwDefer => {
                let span = self.cur().span;
                self.advance();
                let expr = self.parse_expr(0).unwrap_or(Expr::Ident { name: String::new(), span });
                Some(Stmt::Defer(Box::new(expr)))
            }
            KwTry => Some(Stmt::TryCatch(self.parse_try_catch())),
            _ => {
                // expression statement（含 go/ui/match 前缀表达式）
                match self.parse_expr(0) {
                    Some(e) => Some(Stmt::Expr(Box::new(e))),
                    None => {
                        self.recover_stmt();
                        None
                    }
                }
            }
        }
    }

    /// Does the current token continue the previous expression across a newline?
    /// 第 pos+n 个 token 是否为指定类型
    fn peek_is(&self, n: usize, k: TokenKind) -> bool {
        self.tokens.get(self.pos + n).map(|t| t.kind == k).unwrap_or(false)
    }

    /// G2.0 ③：`(` 开头的一组是否为闭包参数表（配对 `)` 后紧跟 `=>`）。
    /// 纯前向扫描无副作用，用于与括号表达式/元组消歧。
    fn paren_group_is_closure_params(&self) -> bool {
        let mut depth: usize = 0;
        let mut i = self.pos;
        while i < self.tokens.len() {
            let k = self.tokens[i].kind;
            match k {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return self.tokens.get(i + 1).map(|t| t.kind == TokenKind::FatArrow).unwrap_or(false);
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// G2.0 ⑥：return 换行后，下一行是否以表达式起始 token 开头（续读为返回值）
    fn starts_return_value(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind(),
            Int | Float | Str | FmtStr | KwTrue | KwFalse | Ident | LParen | LBracket | Minus | Bang
        )
    }

    fn starts_continuation(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind(),
            Dot | LParen | LBracket | Question | Pipe | OrOr | LBrace
                | Plus | Minus | Star | Slash | Percent | Eq | EqEq | NotEq | Lt | Gt | Le | Ge
                | AndAnd | Ampersand | Caret | Shl | Shr
                | PlusEq | MinusEq | StarEq | SlashEq | PercentEq | DotDot
        )
    }

    fn parse_let_stmt(&mut self, is_state: bool) -> Option<Stmt> {
        let start = self.cur().span.start;
        self.advance(); // let
        // G2.0 ①：可变绑定唯一形态 var（let mut 已移除）
        self.finish_let(start, false, is_state)
    }

    fn parse_let_stmt_mut(&mut self) -> Option<Stmt> {
        let start = self.cur().span.start;
        self.advance(); // var
        self.finish_let(start, true, false)
    }

    fn finish_let(&mut self, start: usize, is_mut: bool, is_state: bool) -> Option<Stmt> {
        let pat = self.parse_pattern().unwrap_or(Pattern::Wildcard);
        let ty = if self.eat(TokenKind::Colon).is_some() {
            self.parse_type()
        } else {
            None
        };
        let init = if self.eat(TokenKind::Eq).is_some() {
            self.parse_expr(0).map(Box::new)
        } else {
            None
        };
        // 语句 span 结束 = 上一个被消费 token 的 end（不是下一个 token 的 end，
        // 否则片段会吞进下一行，诊断位置跨行）
        let end = self.tokens[self.pos.saturating_sub(1)].span.end.max(start);
        Some(Stmt::Let(LetStmt { pat, ty, init, is_mut, is_state, span: Span::new(start, end) }))
    }

    fn parse_try_catch(&mut self) -> TryCatchStmt {
        let start = self.cur().span.start;
        self.advance(); // try
        let try_block = self.parse_block();
        let mut catch_param = None;
        let mut catch_block = Block { stmts: Vec::new(), tail: None, span: try_block.span };
        if self.eat(TokenKind::KwCatch).is_some() {
            if self.eat(TokenKind::LParen).is_some() {
                let pstart = self.cur().span.start;
                let name = self.parse_ident_like().unwrap_or_default();
                let ty = if self.eat(TokenKind::Colon).is_some() {
                    self.parse_type()
                } else {
                    None
                };
                let pend = self.cur().span.end;
                catch_param = Some(Param { name, ty, is_move: false, is_mut: false, span: Span::new(pstart, pend) });
                let _ = self.expect(TokenKind::RParen, "')'");
            }
            catch_block = self.parse_block();
        }
        let end = self.cur().span.end;
        TryCatchStmt { try_block, catch_param, catch_block, span: Span::new(start, end) }
    }

    /// Is the current token a statement-start keyword (for match arms)?
    fn at_statement_start(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind(),
            KwReturn | KwLet | KwIf | KwWhile | KwFor | KwBreak | KwContinue | KwDefer | KwTry
        )
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        use TokenKind::*;
        match self.kind() {
            Ident => {
                let t = self.advance();
                if t.text == "_" {
                    return Some(Pattern::Wildcard);
                }
                // variant pattern: Name(field, ...)
                if self.at(LParen) {
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.at(RParen) && !self.at(Eof) {
                        if self.eat(Comma).is_some() {
                            continue;
                        }
                        match self.parse_pattern() {
                            Some(p) => fields.push(p),
                            None => break,
                        }
                    }
                    let _ = self.expect(RParen, "')'");
                    return Some(Pattern::Variant { name: t.text, fields });
                }
                Some(Pattern::Ident { name: t.text, span: t.span })
            }
            Int | Float | Str | FmtStr | KwTrue | KwFalse => {
                let e = self.parse_expr_no_trailing(6)?; // literal; no operators / 尾块
                Some(Pattern::Lit(e))
            }
            Minus => {
                // 负数字面量模式：-1 => ...（lexer 产生 Minus + Int）
                self.advance();
                if self.kind() == Int {
                    let t = self.advance();
                    let v = t.text.parse::<i64>().ok()?;
                    Some(Pattern::Lit(Expr::Int(-v)))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // ---- expressions ----

    fn parse_if_expr(&mut self) -> IfExpr {
        let start = self.cur().span.start;
        self.advance(); // if
        let cond = Box::new(self.parse_expr_no_trailing(0).unwrap_or(Expr::Bool(false)));
        let then_branch = self.parse_block();
        let mut else_branch = None;
        if self.eat(TokenKind::KwElse).is_some() {
            if self.at(TokenKind::KwIf) {
                let nested = self.parse_if_expr();
                else_branch = Some(Box::new(Expr::If(Box::new(nested))));
            } else {
                let blk = self.parse_block();
                else_branch = Some(Box::new(Expr::Block(Box::new(blk))));
            }
        }
        let end = self.cur().span.end;
        IfExpr { cond, then_branch, else_branch, span: Span::new(start, end) }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Option<Expr> {
        self.parse_expr_mode(min_bp, true)
    }

    /// Parse an expression with trailing block/closure consumption disabled
    /// (used for if/while/for/match headers: the '{' belongs to the block).
    fn parse_expr_no_trailing(&mut self, min_bp: u8) -> Option<Expr> {
        self.parse_expr_mode(min_bp, false)
    }

    fn parse_expr_mode(&mut self, min_bp: u8, allow_trailing: bool) -> Option<Expr> {
        let lhs = match self.parse_prefix() {
            Some(e) => e,
            None => return None,
        };
        self.parse_expr_bp(lhs, min_bp, allow_trailing)
    }

    fn parse_expr_bp(&mut self, mut lhs: Expr, min_bp: u8, allow_trailing: bool) -> Option<Expr> {
        loop {
            // postfix operators bind tightest and continue across newlines
            loop {
                match self.kind() {
                    TokenKind::LParen => {
                        let args = self.parse_call_args();
                        let span = self.last_span(&lhs, &args);
                        lhs = Expr::Call { callee: Box::new(lhs), args, span, trailing: false };
                    }
                    TokenKind::Dot => {
                        self.advance();
                        if self.is_name_token() {
                            let field = self.advance().text.clone();
                            let span = self.cur().span;
                            lhs = Expr::Field { base: Box::new(lhs), field, span };
                        } else {
                            self.err_expected("字段名");
                        }
                    }
                    TokenKind::LBracket => {
                        self.advance();
                        let index = self.parse_expr(0).unwrap_or(Expr::Int(0));
                        let _ = self.expect(TokenKind::RBracket, "']'");
                        lhs = Expr::Index { base: Box::new(lhs), index: Box::new(index) };
                    }
                    TokenKind::Question => {
                        self.advance();
                        lhs = Expr::Try(Box::new(lhs));
                    }
                    TokenKind::Ident if allow_trailing
                        && self.peek_is(1, TokenKind::FatArrow)
                        && !self.on_new_line() => {
                        // trailing closure argument: List(records) |r| { ... }
                        // (NOT OrOr: a || b is logical-or in infix position)
                        let closure = self.parse_prefix()?;
                        if let Expr::Closure { .. } = closure {
                            // attach as an extra argument to the call
                            match &mut lhs {
                                Expr::Call { args, trailing, .. } => {
                                    args.push(CallArg {
                                        name: None,
                                        expr: closure,
                                        span: self.cur().span,
                                    });
                                    *trailing = true;
                                }
                                _ => {
                                    // List |r| {} without parens: wrap
                                    lhs = Expr::Call {
                                        callee: Box::new(lhs),
                                        args: vec![CallArg {
                                            name: None,
                                            expr: closure,
                                            span: self.cur().span,
                                        }],
                                        span: self.cur().span,
                                        trailing: true,
                                    };
                                }
                            }
                        }
                    }
                    TokenKind::LBrace if allow_trailing && self.is_struct_literal_start() => {
                        // struct literal: Record { id: 0, desc: "..." }
                        // (ident { field: ... } — field names followed by ':')
                        self.advance(); // '{'
                        let mut fields = Vec::new();
                        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
                            if self.eat(TokenKind::Comma).is_some() {
                                continue;
                            }
                            let fname = match self.parse_ident_like() {
                                Some(n) => n,
                                None => {
                                    self.recover_stmt();
                                    break;
                                }
                            };
                            let _ = self.expect(TokenKind::Colon, "':'");
                            match self.parse_expr(0) {
                                Some(e) => fields.push((fname, e)),
                                None => break,
                            }
                        }
                        let _ = self.expect(TokenKind::RBrace, "'}'");
                        let name = match lhs {
                            Expr::Ident { name, span } => Path {
                                segments: vec![name],
                                generics: Vec::new(),
                                span,
                            },
                            Expr::Path(p) => p,
                            Expr::Field { base, field, span } => {
                                // qualified type name: math.Point { ... }
                                let prefix = match &*base {
                                    Expr::Ident { name, .. } => name.clone(),
                                    Expr::Path(p) => p.segments.join("::"),
                                    _ => String::new(),
                                };
                                Path {
                                    segments: vec![prefix, field.clone()],
                                    generics: Vec::new(),
                                    span,
                                }
                            }
                            other => {
                                self.err_unexpected("结构体字面量的左侧必须是类型名");
                                return Some(other);
                            }
                        };
                        lhs = Expr::StructLit { name, fields };
                    }
                    TokenKind::LBrace if allow_trailing => {
                        // trailing block: Column { ... } / Button("x") { ... }
                        let block = self.parse_block();
                        let closure = Expr::Closure {
                            params: Vec::new(),
                            body: Box::new(Expr::Block(Box::new(block))),
                            span: self.cur().span,
                        };
                        match &mut lhs {
                            Expr::Call { args, trailing, .. } => {
                                args.push(CallArg { name: None, expr: closure, span: self.cur().span });
                                *trailing = true;
                            }
                            _ => {
                                lhs = Expr::Call {
                                    callee: Box::new(lhs),
                                    args: vec![CallArg { name: None, expr: closure, span: self.cur().span }],
                                    span: self.cur().span,
                                    trailing: true,
                                };
                            }
                        }
                    }
                    _ => break,
                }
            }

            // binary operators (low binding; continue across newlines)
            let (op, lbp, rbp) = match self.kind() {
                TokenKind::Eq => (BinOpL::Assign, 1, 1),
                TokenKind::PlusEq => (BinOpL::AssignPlus, 1, 1),
                TokenKind::MinusEq => (BinOpL::AssignMinus, 1, 1),
                TokenKind::StarEq => (BinOpL::AssignStar, 1, 1),
                TokenKind::SlashEq => (BinOpL::AssignSlash, 1, 1),
                TokenKind::PercentEq => (BinOpL::AssignPercent, 1, 1),
                TokenKind::DotDot => (BinOpL::Range, 2, 3),
                TokenKind::OrOr => (BinOpL::Or, 3, 4),
                TokenKind::AndAnd => (BinOpL::And, 4, 5),
                TokenKind::EqEq => (BinOpL::Eq, 5, 6),
                TokenKind::NotEq => (BinOpL::Ne, 5, 6),
                TokenKind::Pipe => (BinOpL::BitOr, 6, 7),
                TokenKind::Ampersand => (BinOpL::BitAnd, 7, 8),
                TokenKind::Lt => (BinOpL::Lt, 9, 10),
                TokenKind::Le => (BinOpL::Le, 9, 10),
                TokenKind::Gt => (BinOpL::Gt, 9, 10),
                TokenKind::Ge => (BinOpL::Ge, 9, 10),
                TokenKind::Caret => (BinOpL::BitXor, 11, 12),
                TokenKind::Shl => (BinOpL::Shl, 11, 12),
                TokenKind::Shr => (BinOpL::Shr, 11, 12),
                TokenKind::Plus => (BinOpL::Add, 13, 14),
                TokenKind::Minus => (BinOpL::Sub, 13, 14),
                TokenKind::Star => (BinOpL::Mul, 15, 16),
                TokenKind::Slash => (BinOpL::Div, 15, 16),
                TokenKind::Percent => (BinOpL::Rem, 15, 16),
                _ => break,
            };

            if lbp < min_bp {
                break;
            }
            self.advance();

            // range with missing lhs handled in prefix; here lhs is present
            if matches!(op, BinOpL::Range) {
                let rhs = if self.at(TokenKind::DotDot) {
                    // a.. — open end
                    None
                } else {
                    self.parse_expr_mode(rbp, allow_trailing)
                };
                lhs = Expr::Range { start: Some(Box::new(lhs)), end: rhs.map(Box::new) };
                continue;
            }

            if op.is_assign() {
                // right-assoc assignment
                let value = self.parse_expr_mode(rbp, allow_trailing);
                match value {
                    Some(v) => {
                        let assign_op = op.to_assign_op();
                        lhs = Expr::Assign {
                            op: assign_op,
                            target: Box::new(lhs),
                            value: Box::new(v),
                        };
                    }
                    None => {
                        self.err_expected("赋值表达式");
                    }
                }
                continue;
            }

            let rhs = match self.parse_expr_mode(rbp, allow_trailing) {
                Some(e) => e,
                None => {
                    self.err_expected("表达式");
                    break;
                }
            };
            lhs = Expr::Binary { op: op.to_binop(), lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Some(lhs)
    }

    fn last_span(&self, lhs: &Expr, args: &[CallArg]) -> Span {
        let start = match lhs {
            Expr::Ident { span, .. } => span.start,
            Expr::Path(p) => p.span.start,
            Expr::Field { span, .. } => span.start,
            _ => 0,
        };
        let end = args.last().map(|a| a.span.end).unwrap_or_else(|| self.cur().span.end);
        Span::new(start, end)
    }

    /// Struct literal detection: ident { field: ... } — the token after '{'
    /// is a name and the one after that is ':'.
    fn is_struct_literal_start(&self) -> bool {
        // current token is '{'; look at the two following tokens
        let n1 = self.peek(1).kind;
        let n2 = self.peek(2).kind;
        self.is_name_like(n1) && n2 == TokenKind::Colon
    }

    fn is_name_like(&self, k: TokenKind) -> bool {
        use TokenKind::*;
        matches!(
            k,
            Ident | KwFn | KwLet | KwMut | KwType | KwIf | KwElse | KwFor | KwIn | KwMatch
                | KwStruct | KwEnum | KwTrait | KwImpl | KwPub | KwUse | KwMod | KwImport
                | KwMove | KwTry | KwCatch | KwUi | KwRender | KwGo | KwDefer | KwTrue | KwFalse
        )
    }

    fn parse_call_args(&mut self) -> Vec<CallArg> {
        let mut args = Vec::new();
        if self.expect(TokenKind::LParen, "'('").is_err() {
            return args;
        }
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            let astart = self.cur().span.start;
            // named argument: name = expr
            let name = if self.is_name_token() && self.peek(1).kind == TokenKind::Eq {
                let n = self.advance().text.clone();
                self.advance(); // '='
                Some(n)
            } else {
                None
            };
            match self.parse_expr(0) {
                Some(e) => {
                    let aend = self.cur().span.end;
                    args.push(CallArg { name, expr: e, span: Span::new(astart, aend) });
                }
                None => {
                    self.err_expected("表达式");
                    self.recover_stmt();
                    break;
                }
            }
        }
        let _ = self.expect(TokenKind::RParen, "')'");
        args
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        use TokenKind::*;
        match self.kind() {
            Ident if self.peek_is(1, TokenKind::FatArrow) => {
                // G2.0 ③：单参无类型闭包 |x| expr
                self.parse_closure()
            }
            LParen if self.paren_group_is_closure_params() => {
                // G2.0 ③：|a, b| expr / |x: i32| expr / () => expr
                self.parse_closure()
            }
            Int => {
                let t = self.advance();
                let v = match &t.value {
                    crate::token::TokenValue::Int(i) => *i,
                    _ => 0,
                };
                Some(Expr::Int(v))
            }
            Float => {
                let t = self.advance();
                let v = match &t.value {
                    crate::token::TokenValue::Float(f) => *f,
                    _ => 0.0,
                };
                Some(Expr::Float(v))
            }
            Str => {
                let t = self.advance();
                let v = match &t.value {
                    crate::token::TokenValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                Some(Expr::Str(v))
            }
            FmtStr => {
                let t = self.advance();
                let v = match &t.value {
                    crate::token::TokenValue::Str(s) => s.clone(),
                    _ => String::new(),
                };
                Some(Expr::FmtStr(v))
            }
            KwTrue => {
                self.advance();
                Some(Expr::Bool(true))
            }
            KwFalse => {
                self.advance();
                Some(Expr::Bool(false))
            }
            Ident => {
                // single-segment path (or struct literal start)
                let t = self.advance();
                if t.text == "_" {
                    return Some(Expr::Ident { name: "_".to_string(), span: t.span });
                }
                Some(Expr::Ident { name: t.text, span: t.span })
            }
            LParen => {
                self.advance();
                let mut items = Vec::new();
                while !self.at(RParen) && !self.at(Eof) {
                    if self.eat(Comma).is_some() {
                        continue;
                    }
                    match self.parse_expr(0) {
                        Some(e) => items.push(e),
                        None => break,
                    }
                }
                let _ = self.expect(RParen, "')'");
                if items.len() == 1 {
                    Some(Expr::Paren(Box::new(items.remove(0))))
                } else {
                    Some(Expr::Tuple(items))
                }
            }
            LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.at(RBracket) && !self.at(Eof) {
                    if self.eat(Comma).is_some() {
                        continue;
                    }
                    match self.parse_expr(0) {
                        Some(e) => items.push(e),
                        None => break,
                    }
                }
                let _ = self.expect(RBracket, "']'");
                Some(Expr::Array(items))
            }
            LBrace => {
                let block = self.parse_block();
                Some(Expr::Block(Box::new(block)))
            }
            Minus => {
                self.advance();
                // 一元操作数只吸收比乘除更紧的后续(min_bp=14: 在 +/- 处停下):
                // -5 + 3 = (-5) + 3, -5 * 3 = (-5) * 3（语法 §2.5 UnaryExpr 层级）
                let e = self.parse_expr_mode(14, false)?;
                Some(Expr::Unary { op: UnOp::Neg, expr: Box::new(e) })
            }
            Bang => {
                self.advance();
                let e = self.parse_expr_mode(14, false)?;
                Some(Expr::Unary { op: UnOp::Not, expr: Box::new(e) })
            }
            Ampersand => {
                self.advance();
                let e = self.parse_expr_mode(14, false)?;
                Some(Expr::Unary { op: UnOp::Ref, expr: Box::new(e) })
            }
            Star => {
                self.advance();
                let e = self.parse_expr_mode(14, false)?;
                Some(Expr::Unary { op: UnOp::Deref, expr: Box::new(e) })
            }
            DotDot => {
                // ..end — range with open start
                self.advance();
                let end = if self.at(DotDot) || self.at(Eof) {
                    None
                } else {
                    self.parse_expr(3)
                };
                Some(Expr::Range { start: None, end: end.map(Box::new) })
            }
            KwGo => {
                let start = self.cur().span.start;
                self.advance();
                let detached = self.eat(Bang).is_some();
                let block = self.parse_block();
                let end = self.cur().span.end;
                Some(Expr::Go { block: Box::new(block), detached, span: Span::new(start, end) })
            }
            KwUi => {
                let start = self.cur().span.start;
                self.advance();
                let block = self.parse_block();
                let end = self.cur().span.end;
                Some(Expr::UiOp { block: Box::new(block), span: Span::new(start, end) })
            }
            KwIf => Some(Expr::If(Box::new(self.parse_if_expr()))),
            KwMatch => {
                let start = self.cur().span.start;
                self.advance();
                let scrutinee = self.parse_expr_no_trailing(0).unwrap_or(Expr::Bool(false));
                let mut arms = Vec::new();
                if self.expect(LBrace, "'{'").is_ok() {
                    while !self.at(RBrace) && !self.at(Eof) {
                        if self.eat(Comma).is_some() {
                            continue;
                        }
                        let astart = self.cur().span.start;
                        let pat = self.parse_pattern().unwrap_or(Pattern::Wildcard);
                        if self.at(LBrace) {
                            // G2.0 ⑤：块式臂 Pat { body }（与闭包 x => 无冲突）
                            let block = self.parse_block();
                            let aend = self.cur().span.end;
                            arms.push(MatchArm {
                                pat,
                                value: Expr::Block(Box::new(block)),
                                span: Span::new(astart, aend),
                            });
                            continue;
                        }
                        if self.expect(FatArrow, "'=>'").is_err() {
                            break;
                        }
                        // arm value: statement-start keywords (return/let/...)
                        // are wrapped into a block so match arms can return
                        let value = if self.at_statement_start() {
                            let mut stmts = Vec::new();
                            if let Some(s) = self.parse_stmt() {
                                stmts.push(s);
                            }
                            let sp = Span::new(self.cur().span.start, self.cur().span.end);
                            Expr::Block(Box::new(Block { stmts, tail: None, span: sp }))
                        } else {
                            match self.parse_expr(0) {
                                Some(v) => v,
                                None => break,
                            }
                        };
                        let aend = self.cur().span.end;
                        arms.push(MatchArm { pat, value, span: Span::new(astart, aend) });
                    }
                    let _ = self.expect(RBrace, "'}'");
                }
                let end = self.cur().span.end;
                Some(Expr::Match { scrutinee: Box::new(scrutinee), arms, span: Span::new(start, end) })
            }
            _ => {
                self.err_expected("表达式");
                None
            }
        }
    }

    fn parse_closure(&mut self) -> Option<Expr> {
        // G2.0 ③: 闭包唯一文法为箭头形态 `x => expr` / `(a, b) => expr` /
        // `(x: i32) => expr` / `() => expr`(竖线 |x| 形态从未实现, 见 docs/GRAMMAR.md)
        let start = self.cur().span.start;
        let mut params: Vec<Param> = Vec::new();
        if self.at(TokenKind::LParen) {
            self.advance();
            while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
                if self.eat(TokenKind::Comma).is_some() {
                    continue;
                }
                let pstart = self.cur().span.start;
                let name = match self.parse_ident_like() {
                    Some(n) => n,
                    None => break,
                };
                let ty = if self.eat(TokenKind::Colon).is_some() {
                    self.parse_type()
                } else {
                    None
                };
                let pend = self.cur().span.end;
                params.push(Param { name, ty, is_move: false, is_mut: false, span: Span::new(pstart, pend) });
            }
            let _ = self.expect(TokenKind::RParen, "')'");
        } else {
            let pstart = self.cur().span.start;
            let name = self.parse_ident_like()?;
            let pend = self.cur().span.end;
            params.push(Param { name, ty: None, is_move: false, is_mut: false, span: Span::new(pstart, pend) });
        }
        let _ = self.expect(TokenKind::FatArrow, "'=>'");
        let body = self.parse_expr(0)?;
        let end = self.cur().span.end;
        Some(Expr::Closure { params, body: Box::new(body), span: Span::new(start, end) })
    }
}


/// Parse a single expression from a string (used by f-string interpolation
/// at interpretation time). Returns None on lex/parse errors.
pub fn parse_expression_str(src: &str) -> Option<crate::ast::Expr> {
    let lexed = crate::lexer::Lexer::new(src).lex();
    if lexed.diagnostics.has_errors() {
        return None;
    }
    let mut p = Parser::new(&lexed.tokens, "<expr>");
    let e = p.parse_expr(0)?;
    if p.at(TokenKind::Eof) {
        Some(e)
    } else {
        None
    }
}

// BinOp extension helpers (kept out of ast.rs to avoid bloating it)
enum BinOpL {
    Assign,
    AssignPlus,
    AssignMinus,
    AssignStar,
    AssignSlash,
    AssignPercent,
    Range,
    Or,
    And,
    Eq,
    Ne,
    BitOr,
    BitAnd,
    Lt,
    Le,
    Gt,
    Ge,
    BitXor,
    Shl,
    Shr,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl BinOpL {
    fn is_assign(&self) -> bool {
        matches!(
            self,
            BinOpL::Assign
                | BinOpL::AssignPlus
                | BinOpL::AssignMinus
                | BinOpL::AssignStar
                | BinOpL::AssignSlash
                | BinOpL::AssignPercent
        )
    }
    fn to_assign_op(&self) -> AssignOp {
        match self {
            BinOpL::Assign => AssignOp::Assign,
            BinOpL::AssignPlus => AssignOp::AddAssign,
            BinOpL::AssignMinus => AssignOp::SubAssign,
            BinOpL::AssignStar => AssignOp::MulAssign,
            BinOpL::AssignSlash => AssignOp::DivAssign,
            BinOpL::AssignPercent => AssignOp::RemAssign,
            _ => AssignOp::Assign,
        }
    }
    fn to_binop(&self) -> BinOp {
        match self {
            BinOpL::Range => BinOp::Range,
            BinOpL::Or => BinOp::Or,
            BinOpL::And => BinOp::And,
            BinOpL::Eq => BinOp::Eq,
            BinOpL::Ne => BinOp::Ne,
            BinOpL::BitOr => BinOp::BitOr,
            BinOpL::BitAnd => BinOp::BitAnd,
            BinOpL::Lt => BinOp::Lt,
            BinOpL::Le => BinOp::Le,
            BinOpL::Gt => BinOp::Gt,
            BinOpL::Ge => BinOp::Ge,
            BinOpL::BitXor => BinOp::BitXor,
            BinOpL::Shl => BinOp::Shl,
            BinOpL::Shr => BinOp::Shr,
            BinOpL::Add => BinOp::Add,
            BinOpL::Sub => BinOp::Sub,
            BinOpL::Mul => BinOp::Mul,
            BinOpL::Div => BinOp::Div,
            BinOpL::Rem => BinOp::Rem,
            _ => BinOp::Add,
        }
    }
}
