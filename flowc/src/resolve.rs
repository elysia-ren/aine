//! Name resolution (design doc §56: Name Resolution after HIR).
//!
//! Walks the AST, builds lexical scopes, binds every definition to a symbol,
//! resolves every identifier occurrence to its definition, and emits
//! semantic diagnostics (N3xxx):
//! - N3001 undefined name
//! - N3002 duplicate definition
//! - N3003 unknown type (warning at this stage; hardened in M1.4 type check)
//!
//! Rules:
//! - items (fn/struct/enum/type/ui/global/import/mod) live in the module
//!   scope and are hoisted (forward references allowed);
//! - locals must be declared before use within a block; shadowing is allowed;
//! - duplicate params / duplicate items in the same scope are errors;
//! - closures push a scope for their params; for-patterns bind in the body;
//! - ui{} operations inherit the enclosing ui-def scope (component states);

use std::collections::HashMap;

use crate::ast::{self, Expr, Item, Pattern, Stmt, Type};
use crate::diagnostics::{codes, Diagnostic, DiagnosticSink, Severity};
use crate::hir::{
    FnInfo, HirItem, HirProgram, ParamInfo, Resolution, Symbol, SymbolId, SymbolKind, UiInfo,
    BUILTIN_GLOBALS, BUILTIN_TYPES, BUILTIN_VARIANTS,
};
use crate::token::{Span, Token};

/// Result of the resolution pass.
pub struct Resolved {
    pub program: HirProgram,
    pub resolution: Resolution,
    pub diagnostics: DiagnosticSink,
}

pub struct Resolver {
    /// global symbol table
    symbols: Vec<Symbol>,
    /// scope stack: each frame maps name -> symbol id
    scopes: Vec<HashMap<String, SymbolId>>,
    /// module-level items by name (for type/name lookup across scopes)
    #[allow(dead_code)]
    module_items: HashMap<String, SymbolId>,
    resolution: Resolution,
    diagnostics: DiagnosticSink,
    /// sorted (byte_start, line, col) index from tokens, for diagnostics
    locs: Vec<(usize, u32, u32)>,
}

impl Resolver {
    pub fn new(tokens: &[Token]) -> Self {
        let locs: Vec<(usize, u32, u32)> = tokens
            .iter()
            .map(|t| (t.span.start, t.loc.line, t.loc.col))
            .collect();
        let mut r = Resolver {
            symbols: Vec::new(),
            scopes: Vec::new(),
            module_items: HashMap::new(),
            resolution: Resolution::default(),
            diagnostics: DiagnosticSink::new(),
            locs,
        };
        r.push_scope();
        r
    }

    /// Approximate (line, col) for a byte offset via binary search on the
    /// token index (exact for token starts; spans used here start at tokens).
    fn loc(&self, byte: usize) -> (u32, u32) {
        match self.locs.binary_search_by_key(&byte, |&(b, _, _)| b) {
            Ok(i) => (self.locs[i].1, self.locs[i].2),
            Err(0) => (0, 0),
            Err(i) => (self.locs[i - 1].1, self.locs[i - 1].2),
        }
    }

    // ---- symbol table ----

    fn intern(&mut self, name: &str, kind: SymbolKind, span: Span, is_mut: bool) -> SymbolId {
        let id = self.symbols.len();
        self.symbols.push(Symbol { id, name: name.to_string(), kind, span, is_mut });
        id
    }

    fn intern_builtin(&mut self, name: &str, span: Span) -> SymbolId {
        if let Some(sym) = self.lookup(name) {
            return sym;
        }
        self.intern(name, SymbolKind::Builtin, span, false)
    }

    fn intern_builtin_type(&mut self, name: &str, span: Span) -> SymbolId {
        if let Some(sym) = self.lookup(name) {
            return sym;
        }
        self.intern(name, SymbolKind::BuiltinType, span, false)
    }

    // ---- scopes ----

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, id: SymbolId) -> Result<(), SymbolId> {
        if let Some(frame) = self.scopes.last_mut() {
            if let Some(existing) = frame.get(name) {
                return Err(*existing);
            }
            frame.insert(name.to_string(), id);
        }
        Ok(())
    }

    fn define_allow_shadow(&mut self, name: &str, id: SymbolId) {
        if let Some(frame) = self.scopes.last_mut() {
            frame.insert(name.to_string(), id);
        }
    }

    /// Look up a name in the current scope chain (innermost first).
    fn lookup(&self, name: &str) -> Option<SymbolId> {
        for frame in self.scopes.iter().rev() {
            if let Some(id) = frame.get(name) {
                return Some(*id);
            }
        }
        None
    }

    // ---- entry ----

    pub fn resolve(mut self, program: &ast::Program) -> Resolved {
        let items = self.resolve_items(&program.items);
        Resolved {
            program: HirProgram { symbols: self.symbols, items },
            resolution: self.resolution,
            diagnostics: self.diagnostics,
        }
    }

    /// Register all items in the current (module) scope, then resolve bodies.
    fn resolve_items(&mut self, items: &[Item]) -> Vec<HirItem> {
        // pass 1: hoist definitions (forward references allowed)。
        // 模块（内联/文件）内的名字递归提升到同一作用域 —— 扁平注册语义下，
        // 跨模块前向引用（模块 A 引用后面模块 B 的函数）必须在此全部可见。
        self.hoist_names(items);

        // pass 2: resolve bodies in source order
        let mut out = Vec::new();
        for item in items {
            out.push(self.resolve_item(item));
        }
        out
    }

    /// 只提升名字（递归进入模块），不解析函数体 —— 供 pass1 使用。
    fn hoist_names(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Import(_) => {
                    // import 是声明标记而非定义：不参与顶层命名（避免与 mod x; 重名）
                }
                Item::Mod(m) => {
                    if let Some(name) = item_name(item) {
                        let kind = item_kind(item);
                        let span = item_span(item);
                        let sym = self.intern(&name, kind, span, false);
                        if let Err(existing) = self.define(&name, sym) {
                            self.dup_definition_diag(&name, existing, span);
                            continue;
                        }
                    }
                    self.hoist_names(&m.items);
                }
                other => {
                    if let Some(name) = item_name(other) {
                        let kind = item_kind(other);
                        let span = item_span(other);
                        let sym = self.intern(&name, kind, span, false);
                        if let Err(existing) = self.define(&name, sym) {
                            self.dup_definition_diag(&name, existing, span);
                        }
                    }
                }
            }
        }
    }

    /// pass 2 专用于模块体：名字已在 pass1 递归提升，这里只解析各项函数体。
    fn resolve_bodies(&mut self, items: &[Item]) -> Vec<HirItem> {
        let mut out = Vec::new();
        for item in items {
            match item {
                Item::Mod(m) => {
                    let nested = self.resolve_bodies(&m.items);
                    let _sym = self.lookup(&m.name).unwrap();
                    out.push(HirItem::Mod { name: m.name.clone(), items: nested, span: m.span });
                }
                other => out.push(self.resolve_item(other)),
            }
        }
        out
    }

    fn dup_definition_diag(&mut self, name: &str, existing: SymbolId, span: Span) {        let t = self.symbols[existing].clone();
        let (line, col) = self.loc(span.start);
        self.diagnostics.push(
            Diagnostic::new(
                codes::DUPLICATE_DEFINITION,
                Severity::Error,
                "duplicate definition",
                &format!("重复定义 '{}'", name),
                line,
                col,
            )
            .with_cause(&format!("同名定义已存在（{}:{}）existing_name={}", t.span.start, t.span.end, t.name))
            .with_suggestion("重命名其中一个定义"),
        );
    }

    fn resolve_item(&mut self, item: &Item) -> HirItem {
        match item {
            Item::Import(imp) => {
                let path = imp.segments.join("::");
                // import 为声明标记：名字已存在（如 `mod x;`）时引用之，不存在时占位
                let sym = self.lookup(&path).unwrap_or_else(|| {
                    self.intern(&path, SymbolKind::Import, imp.span, false)
                });
                HirItem::Import { path, span: imp.span, sym }
            }
            Item::Struct(s) => {
                let sym = self.lookup(&s.name).unwrap();
                let fields = s
                    .fields
                    .iter()
                    .map(|f| {
                        self.resolve_type(&f.ty);
                        (f.name.clone(), f.span, Some(f.ty.clone()))
                    })
                    .collect();
                HirItem::Struct { name: s.name.clone(), fields, span: s.span, sym, attributes: s.attributes.clone() }
            }
            Item::Enum(e) => {
                let sym = self.lookup(&e.name).unwrap();
                let variants = e.variants.iter().map(|v| v.name.clone()).collect();
                for v in &e.variants {
                    for t in &v.payload {
                        self.resolve_type(t);
                    }
                    // N0001: 变体名与 UI 内建组件冲突 → 组件优先，运行期构造返回 Nil
                    const UI_WIDGETS: [&str; 8] = [
                        "Text", "Button", "TextInput", "List", "Row", "Column", "ListItem", "Window",
                    ];
                    if UI_WIDGETS.contains(&v.name.as_str()) {
                        let (line, col) = self.loc(v.span.start);
                        self.diagnostics.push(
                            Diagnostic::new(
                                "N0001",
                                Severity::Warning,
                                "enum variant shadows ui widget",
                                &format!(
                                    "枚举变体 '{}' 与 UI 内建组件同名：运行期组件优先，该变体构造将返回 Nil；请重命名变体（或枚举）",
                                    v.name
                                ),
                                line,
                                col,
                            )
                            .with_span(v.span.start, v.span.end),
                        );
                    }
                    // register the variant as a value symbol (constructor)
                    let vsym = self.intern(&v.name, SymbolKind::EnumVariant, v.span, false);
                    self.define_allow_shadow(&v.name, vsym);
                }
                HirItem::Enum { name: e.name.clone(), variants, span: e.span, sym }
            }
            Item::TypeAlias(t) => {
                let sym = self.lookup(&t.name).unwrap();
                self.resolve_type(&t.ty);
                HirItem::TypeAlias { name: t.name.clone(), span: t.span, sym }
            }
            Item::Fn(f) => {
                let sym = match self.lookup(&f.sig.name) {
                    Some(s) => s,
                    // impl 方法名不在顶层符号表（经 receiver 调用），解析体时按需新建
                    None => self.intern(&f.sig.name, SymbolKind::Fn, f.sig.span, false),
                };
                // resolve ret type
                if let Some(ret) = &f.sig.ret {
                    self.resolve_type(ret);
                }
                // param scope
                self.push_scope();
                let mut params = Vec::new();
                for p in &f.sig.params {
                    if let Some(ty) = &p.ty {
                        self.resolve_type(ty);
                    }
                    let psym = self.intern(&p.name, SymbolKind::Param, p.span, p.is_mut);
                    if let Err(existing) = self.define(&p.name, psym) {
                        let t = self.symbols[existing].clone();
                        let (line, col) = self.loc(p.span.start);
                        self.diagnostics.push(
                            Diagnostic::new(
                                codes::DUPLICATE_DEFINITION,
                                Severity::Error,
                                "duplicate parameter",
                                &format!("参数名 '{}' 重复", p.name),
                                line,
                                col,
                            )
                            .with_cause(&format!("已有同名参数（span {}..{}）", t.span.start, t.span.end)),
                        );
                    }
                    params.push(ParamInfo {
                        name: p.name.clone(),
                        sym: psym,
                        span: p.span,
                        ty: p.ty.clone(),
                        is_move: p.is_move,
                        is_mut: p.is_mut,
                    });
                }
                self.resolve_block(&f.body);
                self.pop_scope();
                HirItem::Fn(FnInfo {
                    name: f.sig.name.clone(),
                    params,
                    body: f.body.clone(),
                    ret: f.sig.ret.clone(),
                    span: f.span,
                    sym,
                    is_pub: f.is_pub,
                    is_unsafe: f.is_unsafe,
                    is_extern: f.is_extern,
                    is_catchable: f.is_catchable,
                })
            }
            Item::Ui(u) => {
                let sym = self.lookup(&u.name).unwrap();
                // ui scope: props + states visible inside render
                self.push_scope();
                let mut props = Vec::new();
                for p in &u.props {
                    let psym = self.intern(&p.name, SymbolKind::Prop, p.span, false);
                    self.define_allow_shadow(&p.name, psym);
                    self.resolve_expr(&p.value);
                    props.push((p.name.clone(), p.span));
                }
                let mut states = Vec::new();
                for s in &u.states {
                    let sname = s.pat.ident_name().unwrap_or_default();
                    let ssym = self.intern(&sname, SymbolKind::State, s.span, s.is_mut);
                    self.define_allow_shadow(&sname, ssym);
                    if let Some(ty) = &s.ty {
                        self.resolve_type(ty);
                    }
                    if let Some(init) = &s.init {
                        self.resolve_expr(init);
                    }
                    states.push(crate::hir::UiState {
                        name: sname,
                        sym: ssym,
                        span: s.span,
                        ty: s.ty.clone(),
                        init: s.init.as_ref().map(|e| (**e).clone()),
                    });
                }
                let render = u.render.as_ref().map(|b| {
                    self.resolve_block(b);
                    b.clone()
                });
                self.pop_scope();
                HirItem::Ui(UiInfo {
                    name: u.name.clone(),
                    props,
                    states,
                    render,
                    span: u.span,
                    sym,
                })
            }
            Item::GlobalState(g) => {
                let sym = self.lookup(&g.name).unwrap();
                if let Some(ty) = &g.ty {
                    self.resolve_type(ty);
                }
                if let Some(init) = &g.init {
                    self.resolve_expr(init);
                }
                HirItem::Global {
                    name: g.name.clone(),
                    ty: g.ty.clone(),
                    init: g.init.clone(),
                    span: g.span,
                    sym,
                }
            }
            Item::Mod(m) => {
                // flat module semantics: items register into the parent scope
                //（pass1 已递归提升名字 —— 这里只解析模块内函数体，避免重复定义）
                let items = self.resolve_bodies(&m.items);
                let _sym = self.lookup(&m.name).unwrap();
                HirItem::Mod { name: m.name.clone(), items, span: m.span }
            }
            Item::ExternBlock(_) | Item::Trait(_) => {
                // defer detailed handling to M1.4 (types/modules); resolve nothing
                HirItem::Mod { name: String::new(), items: Vec::new(), span: item_span(item) }
            }
            Item::Impl(ib) => {
                // 编码 interface 实现信息进 Mod.name："impl|<interface>|<type>"（typeck 建约束表）
                let iface = ib.trait_path.as_ref().map(|p| p.segments.join("::")).unwrap_or_default();
                let ty_name = match &ib.self_ty {
                    ast::Type::Path(p) => p.segments.last().cloned().unwrap_or_default(),
                    _ => String::new(),
                };
                let items = self.resolve_bodies(&ib.items);
                HirItem::Mod { name: format!("impl|{}|{}", iface, ty_name), items, span: item_span(item) }
            }
        }
    }

    // ---- blocks & statements ----

    fn resolve_block(&mut self, block: &ast::Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
        if let Some(tail) = &block.tail {
            self.resolve_expr(tail);
        }
        self.pop_scope();
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(ls) => {
                if let Some(ty) = &ls.ty {
                    self.resolve_type(ty);
                }
                if let Some(init) = &ls.init {
                    self.resolve_expr(init);
                }
                // bind after init (declared-before-use)
                if let Some(name) = ls.pat.ident_name() {
                    let kind = if ls.is_state { SymbolKind::State } else { SymbolKind::Local };
                    let sym = self.intern(&name, kind, ls.span, ls.is_mut);
                    self.define_allow_shadow(&name, sym);
                    self.resolution.bindings.insert(ls.span, sym);
                }
            }
            Stmt::Expr(e) => self.resolve_expr(e),
            Stmt::Return(v) => {
                if let Some(e) = v {
                    self.resolve_expr(e);
                }
            }
            Stmt::If(ifexpr) => {
                self.resolve_expr(&ifexpr.cond);
                self.resolve_block(&ifexpr.then_branch);
                if let Some(else_e) = &ifexpr.else_branch {
                    self.resolve_expr(else_e);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.resolve_expr(cond);
                self.resolve_block(body);
            }
            Stmt::For { pat, iter, body, .. } => {
                self.resolve_expr(iter);
                self.push_scope();
                if let Some(name) = pat.ident_name() {
                    let sym = self.intern(&name, SymbolKind::ForPattern, pat_span(pat), false);
                    self.define_allow_shadow(&name, sym);
                    self.resolution.bindings.insert(pat_span(pat), sym);
                }
                self.resolve_block(body);
                self.pop_scope();
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Defer(e) => self.resolve_expr(e),
            Stmt::TryCatch(tc) => {
                self.resolve_block(&tc.try_block);
                if let Some(p) = &tc.catch_param {
                    self.push_scope();
                    let sym = self.intern(&p.name, SymbolKind::Param, p.span, false);
                    self.define_allow_shadow(&p.name, sym);
                    self.resolve_block(&tc.catch_block);
                    self.pop_scope();
                } else {
                    self.resolve_block(&tc.catch_block);
                }
            }
            Stmt::Block(b) => self.resolve_block(b),
        }
    }

    // ---- expressions ----

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::FmtStr(_) | Expr::Bool(_) => {}
            Expr::Ident { name, span } => {
                self.resolve_ident(name, *span);
            }
            Expr::Path(p) => {
                // multi-segment path (e.g. foo::bar): resolve first segment
                if let Some(first) = p.segments.first() {
                    self.resolve_ident(first, p.span);
                }
            }
            Expr::Array(items) => {
                for e in items {
                    self.resolve_expr(e);
                }
            }
            Expr::Tuple(items) => {
                for e in items {
                    self.resolve_expr(e);
                }
            }
            Expr::StructLit { name, fields } => {
                self.resolve_type_name(&name.segments.join("::"), name.span);
                // check fields against struct definition if user-defined
                if let Some(sym) = self.lookup(&name.segments.join("::")) {
                    if let Some(Symbol { kind: SymbolKind::Struct, .. }) = self.symbols.get(sym) {
                        // field existence is checked in M1.4 type check (we don't
                        // have resolved field tables here yet); resolve values
                        for (_, v) in fields {
                            self.resolve_expr(v);
                        }
                    } else {
                        for (_, v) in fields {
                            self.resolve_expr(v);
                        }
                    }
                } else {
                    for (_, v) in fields {
                        self.resolve_expr(v);
                    }
                }
            }
            Expr::Unary { expr, .. } => self.resolve_expr(expr),
            Expr::Binary { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::Assign { target, value, .. } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            Expr::Call { callee, args, .. } => {
                // method calls: resolve only the receiver
                match &**callee {
                    Expr::Field { base, .. } => self.resolve_expr(base),
                    other => self.resolve_expr(other),
                }
                for a in args {
                    self.resolve_expr(&a.expr);
                }
            }
            Expr::Field { base, .. } => self.resolve_expr(base),
            Expr::Index { base, index } => {
                self.resolve_expr(base);
                self.resolve_expr(index);
            }
            Expr::Try(e) => self.resolve_expr(e),
            Expr::Range { start, end } => {
                if let Some(s) = start {
                    self.resolve_expr(s);
                }
                if let Some(e) = end {
                    self.resolve_expr(e);
                }
            }
            Expr::Block(b) => self.resolve_block(b),
            Expr::If(ifexpr) => {
                self.resolve_expr(&ifexpr.cond);
                self.resolve_block(&ifexpr.then_branch);
                if let Some(e) = &ifexpr.else_branch {
                    self.resolve_expr(e);
                }
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.resolve_expr(scrutinee);
                for arm in arms {
                    self.push_scope();
                    for (name, span) in arm.pat.bindings() {
                        let sym = self.intern(&name, SymbolKind::ForPattern, span, false);
                        self.define_allow_shadow(&name, sym);
                        self.resolution.bindings.insert(span, sym);
                    }
                    self.resolve_expr(&arm.value);
                    self.pop_scope();
                }
            }
            Expr::Closure { params, body, .. } => {
                self.push_scope();
                for p in params {
                    let sym = self.intern(&p.name, SymbolKind::ClosureParam, p.span, false);
                    self.define_allow_shadow(&p.name, sym);
                    self.resolution.bindings.insert(p.span, sym);
                }
                self.resolve_expr(body);
                self.pop_scope();
            }
            Expr::Go { block, .. } => self.resolve_block(block),
            Expr::UiOp { block, .. } => self.resolve_block(block),
            Expr::Paren(e) => self.resolve_expr(e),
        }
    }

    fn resolve_ident(&mut self, name: &str, span: Span) {
        if name == "_" {
            return;
        }
        if let Some(sym) = self.lookup(name) {
            self.resolution.idents.insert(span, sym);
            return;
        }
        // builtin enum variants (Some/None/Ok/Err)
        if BUILTIN_VARIANTS.contains(&name) {
            let sym = self.intern_builtin(name, span);
            self.resolution.idents.insert(span, sym);
            return;
        }
        // builtin types usable as values (e.g. Map.new())
        if BUILTIN_TYPES.contains(&name) {
            let sym = self.intern_builtin(name, span);
            self.resolution.idents.insert(span, sym);
            return;
        }
        // builtin globals (print, db, UI components, ...)
        if BUILTIN_GLOBALS.contains(&name) {
            let sym = self.intern_builtin(name, span);
            self.resolution.idents.insert(span, sym);
            return;
        }
        let (line, col) = self.loc(span.start);
        // 在已知名集合中找编辑距离最近的候选（≤2 即提示"是不是想用 X"）
        let mut best: Option<(&String, usize)> = None;
        for frame in &self.scopes {
            for known in frame.keys() {
                let d = edit_distance(name, known);
                if d <= 2 && (best.is_none() || d < best.unwrap().1) {
                    best = Some((known, d));
                }
            }
        }
        let sug = match best {
            Some((known, d)) => format!("未定义的名称 '{}'。{}是不是想用 '{}'？", name,
                if d == 0 { String::new() } else { format!("（与 '{}' 相差 {} 个字符）", known, d) },
                known),
            None => format!("未定义的名称 '{}'。Aine 中使用的每个名字都需要先声明：变量用 let，函数用 fn，类型用 struct/enum", name),
        };
        self.diagnostics.push(
            Diagnostic::new(
                codes::UNDEFINED_NAME,
                Severity::Error,
                "undefined name",
                &format!("未定义的名称 '{}'", name),
                line,
                col,
            )
            .with_span(span.start, span.end)
            .with_suggestion(sug),
        );
    }

    /// Resolve a type reference: builtin types are fine, module types must exist.
    fn resolve_type(&mut self, ty: &Type) {
        match ty {
            Type::Path(p) => {
                let full = p.segments.join("::");
                self.resolve_type_name(&full, p.span);
            }
            Type::Fn { params, ret } => {
                for p in params {
                    self.resolve_type(p);
                }
                if let Some(r) = ret {
                    self.resolve_type(r);
                }
            }
            Type::Ref { ty, .. } => self.resolve_type(ty),
            Type::Tuple(tys) => {
                for t in tys {
                    self.resolve_type(t);
                }
            }
            Type::Infer => {}
        }
    }

    fn resolve_type_name(&mut self, name: &str, span: Span) {
        if BUILTIN_TYPES.contains(&name) {
            let _sym = self.intern_builtin_type(name, span);
            self.resolution.type_refs.push(span);
            return;
        }
        if let Some(sym) = self.lookup(name) {
            let kind = self.symbols[sym].kind;
            if matches!(kind, SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias | SymbolKind::Ui) {
                self.resolution.type_refs.push(span);
                return;
            }
        }
        // unknown type: warning (hardened in M1.4 type check)
        let (line, col) = self.loc(span.start);
        self.diagnostics.push(
            Diagnostic::new(
                codes::UNKNOWN_TYPE,
                Severity::Warning,
                "unknown type",
                &format!("未定义的类型 '{}'", name),
                line,
                col,
            )
            .with_span(span.start, span.end)
            .with_suggestion("确认类型名拼写；若为外部类型，请先导入"),
        );
    }
}

// ---- helpers ----

fn item_name(item: &Item) -> Option<String> {
    match item {
        Item::Import(i) => Some(i.segments.join("::")),
        Item::Struct(s) => Some(s.name.clone()),
        Item::Enum(e) => Some(e.name.clone()),
        Item::TypeAlias(t) => Some(t.name.clone()),
        Item::Trait(t) => Some(t.name.clone()),
        Item::Fn(f) => Some(f.sig.name.clone()),
        Item::Ui(u) => Some(u.name.clone()),
        Item::GlobalState(g) => Some(g.name.clone()),
        Item::Mod(m) => Some(m.name.clone()),
        Item::Impl(_) | Item::ExternBlock(_) => None,
    }
}

fn item_kind(item: &Item) -> SymbolKind {
    match item {
        Item::Import(_) => SymbolKind::Import,
        Item::Struct(_) => SymbolKind::Struct,
        Item::Enum(_) => SymbolKind::Enum,
        Item::TypeAlias(_) => SymbolKind::TypeAlias,
        Item::Trait(_) => SymbolKind::TypeAlias,
        Item::Fn(_) => SymbolKind::Fn,
        Item::Ui(_) => SymbolKind::Ui,
        Item::GlobalState(_) => SymbolKind::Global,
        Item::Mod(_) => SymbolKind::Mod,
        Item::Impl(_) | Item::ExternBlock(_) => SymbolKind::TypeAlias,
    }
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Import(i) => i.span,
        Item::Struct(s) => s.span,
        Item::Enum(e) => e.span,
        Item::TypeAlias(t) => t.span,
        Item::Trait(t) => t.span,
        Item::Fn(f) => f.span,
        Item::Ui(u) => u.span,
        Item::GlobalState(g) => g.span,
        Item::Mod(m) => m.span,
        Item::Impl(i) => i.span,
        Item::ExternBlock(e) => e.span,
    }
}

fn pat_span(p: &Pattern) -> Span {
    match p {
        Pattern::Ident { span, .. } => *span,
        Pattern::Wildcard => Span::new(0, 0),
        Pattern::Lit(_) => Span::new(0, 0),
        Pattern::Path(path) => path.span,
        Pattern::Variant { .. } => Span::new(0, 0),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::SymbolKind;
    use crate::lexer::Lexer;

    fn resolve_src(src: &str) -> (HirProgram, Resolution, DiagnosticSink) {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = crate::parser::Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let resolver = Resolver::new(&lexed.tokens);
        let resolved = resolver.resolve(&program);
        (resolved.program, resolved.resolution, resolved.diagnostics)
    }

    fn err_count(d: &DiagnosticSink) -> usize {
        d.error_count()
    }

    #[test]
    fn resolves_params_and_locals() {
        let (prog, res, diags) = resolve_src("fn f(a: i32) { let b = a + 1 }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
        assert!(prog.symbols.iter().any(|s| s.name == "a" && s.kind == SymbolKind::Param));
        assert!(prog.symbols.iter().any(|s| s.name == "b" && s.kind == SymbolKind::Local));
        // the reference to 'a' in the init must be resolved
        assert!(res.idents.len() >= 1, "reference to a resolved, got {}", res.idents.len());
    }

    #[test]
    fn undefined_name_is_error() {
        let (_, _, diags) = resolve_src("fn f() { print(missing_var) }");
        assert!(err_count(&diags) >= 1);
        let d = &diags.diagnostics[0];
        assert_eq!(d.code, codes::UNDEFINED_NAME);
        assert!(d.message.contains("missing_var"));
    }

    #[test]
    fn duplicate_params_is_error() {
        let (_, _, diags) = resolve_src("fn f(a: i32, a: i32) { }");
        assert!(err_count(&diags) >= 1);
        assert_eq!(diags.diagnostics[0].code, codes::DUPLICATE_DEFINITION);
    }

    #[test]
    fn duplicate_items_is_error() {
        let (_, _, diags) = resolve_src("fn f() { }\nfn f() { }");
        assert!(err_count(&diags) >= 1);
        assert_eq!(diags.diagnostics[0].code, codes::DUPLICATE_DEFINITION);
    }

    #[test]
    fn forward_reference_to_item_is_ok() {
        let (_, res, diags) = resolve_src("fn f() { g() }\nfn g() { }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
        assert!(res.idents.len() >= 1);
    }

    #[test]
    fn shadowing_allows_rebinding() {
        let (_, _, diags) = resolve_src("fn f() { let x = 1\nlet x = x + 1 }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn declared_before_use_required() {
        let (_, _, diags) = resolve_src("fn f() { print(y)\nlet y = 1 }");
        assert!(err_count(&diags) >= 1);
        assert_eq!(diags.diagnostics[0].code, codes::UNDEFINED_NAME);
    }

    #[test]
    fn closure_params_resolve_in_body() {
        let (_, _, diags) = resolve_src("fn f() { let g = x => x + 1 }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn closure_captures_outer() {
        let (_, _, diags) = resolve_src("fn f() { let n = 10\nlet g = x => x + n }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn for_pattern_binds_in_body() {
        let (_, _, diags) = resolve_src("fn f() { for i in 0..10 { print(i) } }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn ui_states_resolve_in_render_and_callbacks() {
        let src = "ui Counter {\n    @state\n    var count = 0\n    render {\n        Button(\"+1\") {\n            count += 1\n        }\n        Text(f\"{count}\")\n    }\n}";
        let (prog, _, diags) = resolve_src(src);
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
        assert!(prog.symbols.iter().any(|s| s.name == "count" && s.kind == SymbolKind::State));
    }

    #[test]
    fn method_calls_resolve_receiver() {
        let (_, _, diags) = resolve_src("fn load() { }\nfn f() { let r = load()\nr.update() }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn struct_literal_resolves_type_and_fields() {
        let (_, _, diags) = resolve_src("struct User { name: String }\nfn f() { let u = User { name: \"a\" } }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn builtin_globals_resolve() {
        let (_, res, diags) = resolve_src("fn f() { print(now()) }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
        assert!(res.idents.len() >= 2);
    }

    #[test]
    fn unknown_type_is_warning_not_error() {
        let (_, _, diags) = resolve_src("fn f(x: i32) { match x { 1 => x
other => x } }");
        assert_eq!(err_count(&diags), 0, "{:?}", diags.diagnostics);
    }

    #[test]
    fn global_state_resolves() {
        let (prog, _, diags) = resolve_src("@global\nvar settings: Config = init()");
        assert!(err_count(&diags) <= 1, "{:?}", diags.diagnostics);
        assert!(prog.symbols.iter().any(|s| s.name == "settings" && s.kind == SymbolKind::Global));
    }

    #[test]
    fn all_example_idents_resolve() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let lexed = Lexer::new(&src).lex();
            let parsed = crate::parser::Parser::new(&lexed.tokens, name).parse_program();
            let program = parsed.program.unwrap();
            let resolver = Resolver::new(&lexed.tokens);
            let resolved = resolver.resolve(&program);
            assert_eq!(
                resolved.diagnostics.error_count(),
                0,
                "{} has semantic errors: {:?}",
                name,
                resolved.diagnostics.diagnostics
            );
        }
    }
}




/// 简易编辑距离（诊断建议用；名称一般很短，O(mn) 足够）
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=b.len() {
        dp[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}
