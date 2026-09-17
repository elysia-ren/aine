//! Type checking (M1.4), following Flow_Type_System.md (T1.0).
//!
//! Gradual typing: explicit errors for clear mistakes, Unknown fallback for
//! the unknown library surface (methods, builtins, extern) - no false errors.
//!
//! Runs after name resolution: walks the resolved HIR, keeps a SymbolId to
//! Ty environment, and produces a Span to Ty map plus diagnostics (T3xxx).

use std::collections::{HashMap, HashSet};

use crate::ast::{self, Expr, Pattern, Stmt, Type};
use crate::diagnostics::{codes, Diagnostic, DiagnosticSink, Severity};
use crate::hir::{HirItem, HirProgram, Resolution, Symbol, SymbolId, SymbolKind};
use crate::token::{Span, Token};

/// Result of the type-check pass.
pub struct TypeckResult {
    pub diagnostics: DiagnosticSink,
    /// type of each typed expression node, keyed by its source Span
    pub types: HashMap<Span, Ty>,
    /// final inferred type of each binding symbol (for valueal analysis)
    pub binding_types: HashMap<SymbolId, Ty>,
    /// struct name -> (field name, field type) — Send 推断等下游分析用
    pub structs: HashMap<String, Vec<(String, Ty)>>,
    /// @not_send 标记的类型（Concurrency Guide §3.3）
    pub not_send: HashSet<String>,
}

/// Aine's type model (Flow_Type_System.md section 1).
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    Usize,
    F32,
    F64,
    Bool,
    Char,
    /// owned String
    Str,
    Vec(Box<Ty>),
    /// Map<String, V>
    Map(Box<Ty>),
    OptionT(Box<Ty>),
    ResultT(Box<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    Ref(Box<Ty>, bool),
    /// user struct / enum / ui component by name
    Named(String),
    /// 统一函数类型 fn(A, B) -> R（捕获闭包同型，捕获即快照）
    Fn(Vec<Ty>, Option<Box<Ty>>),
    /// unit type
    Unit,
    /// unresolved / library surface
    Unknown,
    /// poisoned after a type error (suppress cascades)
    Error,
}
impl Ty {
    pub fn display(&self) -> String {
        match self {
            Ty::I32 => "i32".into(),
            Ty::I64 => "i64".into(),
            Ty::U8 => "u8".into(),
            Ty::U16 => "u16".into(),
            Ty::U32 => "u32".into(),
            Ty::U64 => "u64".into(),
            Ty::Usize => "usize".into(),
            Ty::F32 => "f32".into(),
            Ty::F64 => "f64".into(),
            Ty::Bool => "bool".into(),
            Ty::Char => "char".into(),
            Ty::Str => "String".into(),
            Ty::Vec(t) => format!("Vec<{}>", t.display()),
            Ty::Map(t) => format!("Map<String, {}>", t.display()),
            Ty::OptionT(t) => format!("Option<{}>", t.display()),
            Ty::ResultT(a, b) => format!("Result<{}, {}>", a.display(), b.display()),
            Ty::Fn(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| p.display()).collect();
                let rs = match ret {
                    Some(r) => r.display(),
                    None => "()".to_string(),
                };
                format!("fn({}) -> {}", ps.join(", "), rs)
            }
            Ty::Tuple(tys) => {
                if tys.is_empty() {
                    "()".into()
                } else {
                    let inner: Vec<String> = tys.iter().map(|t| t.display()).collect();
                    format!("({})", inner.join(", "))
                }
            }
            Ty::Ref(t, m) => {
                if *m {
                    format!("&mut {}", t.display())
                } else {
                    format!("&{}", t.display())
                }
            }
            Ty::Named(n) => n.clone(),
            Ty::Unit => "()".into(),
            Ty::Unknown => "?".into(),
            Ty::Error => "<error>".into(),
        }
    }

    fn is_numeric(&self) -> bool {
        matches!(
            self,
            Ty::I32 | Ty::I64 | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::Usize | Ty::F32 | Ty::F64
        )
    }

    fn is_integer(&self) -> bool {
        matches!(self, Ty::I32 | Ty::I64 | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::Usize)
    }

    /// A is assignable to B (Flow_Type_System.md section 4).
    fn assignable(a: &Ty, b: &Ty) -> bool {
        if matches!(a, Ty::Unknown | Ty::Error) || matches!(b, Ty::Unknown | Ty::Error) {
            return true;
        }
        if a == b {
            return true;
        }
        match (a, b) {
            // 统一函数类型：捕获/非捕获同型，签名宽松匹配（无 Fn/Options 三分法）
            (Ty::Fn(_, _), Ty::Fn(_, _)) => true,
            (Ty::Vec(x), Ty::Vec(y)) => Ty::assignable(x, y),
            (Ty::Map(x), Ty::Map(y)) => Ty::assignable(x, y),
            (Ty::OptionT(x), Ty::OptionT(y)) => Ty::assignable(x, y),
            (Ty::ResultT(a1, a2), Ty::ResultT(b1, b2)) => {
                Ty::assignable(a1, b1) && Ty::assignable(a2, b2)
            }
            (Ty::Tuple(a), Ty::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| Ty::assignable(x, y))
            }
            _ => false,
        }
    }
}

/// Function signature: param types + return type.
#[derive(Debug, Clone)]
struct FnSig {
    params: Vec<Ty>,
    ret: Ty,
    arity: usize,
}
/// The checker.
pub struct TypeChecker<'a> {
    hir: &'a HirProgram,
    resolution: &'a Resolution,
    /// struct name -> (field name -> Ty)
    structs: HashMap<String, Vec<(String, Ty)>>,
    /// enum name -> variants
    enums: HashMap<String, Vec<String>>,
    /// variant name -> enum type name
    enum_variants: HashMap<String, String>,
    /// interface name -> implementing types (from `implement I for T`)
    impls: HashMap<String, Vec<String>>,
    /// @not_send 标记的类型（Send 推断时视为非 Send）
    not_send: HashSet<String>,
    /// fn name -> signature
    fns: HashMap<String, FnSig>,
    /// SymbolId -> Ty environment (scope stack)
    env: Vec<HashMap<SymbolId, Ty>>,
    /// current function's declared return type
    cur_ret: Option<Ty>,
    types: HashMap<Span, Ty>,
    binding_types: HashMap<SymbolId, Ty>,
    diagnostics: DiagnosticSink,
    locs: Vec<(usize, u32, u32)>,
}

impl<'a> TypeChecker<'a> {
    pub fn new(hir: &'a HirProgram, resolution: &'a Resolution, tokens: &[Token]) -> Self {
        let locs: Vec<(usize, u32, u32)> = tokens
            .iter()
            .map(|t| (t.span.start, t.loc.line, t.loc.col))
            .collect();
        TypeChecker {
            hir,
            resolution,
            structs: HashMap::new(),
            enums: HashMap::new(),
            fns: HashMap::new(),
            env: Vec::new(),
            cur_ret: None,
            types: HashMap::new(),
            binding_types: HashMap::new(),
            diagnostics: DiagnosticSink::new(),
            locs,
            enum_variants: HashMap::new(),
            impls: HashMap::new(),
            not_send: HashSet::new(),
        }
    }

    pub fn check(mut self) -> TypeckResult {
        // collect type definitions & signatures
        self.collect(&self.hir.items);
        let structs = self.structs.clone();
        let not_send = self.not_send.clone();
        // check item bodies
        for item in &self.hir.items {
            self.check_item(item);
        }
        TypeckResult {
            diagnostics: self.diagnostics,
            types: self.types,
            binding_types: self.binding_types,
            structs,
            not_send,
        }
    }

    fn loc(&self, byte: usize) -> (u32, u32) {
        match self.locs.binary_search_by_key(&byte, |&(b, _, _)| b) {
            Ok(i) => (self.locs[i].1, self.locs[i].2),
            Err(0) => (0, 0),
            Err(i) => (self.locs[i - 1].1, self.locs[i - 1].2),
        }
    }

    fn err(&mut self, code: &str, msg: &str, span: Span) -> Diagnostic {
        let (line, col) = self.loc(span.start);
        let d = Diagnostic::new(code, Severity::Error, "type error", msg, line, col)
            .with_span(span.start, span.end);
        self.diagnostics.push(d.clone());
        d
    }

    /// 带建议的错误（用户导向：每条诊断都应告诉开发者"下一步做什么"）
    fn err_sug(&mut self, code: &str, msg: &str, span: Span, sug: String) -> Diagnostic {
        let (line, col) = self.loc(span.start);
        let d = Diagnostic::new(code, Severity::Error, "type error", msg, line, col)
            .with_span(span.start, span.end)
            .with_suggestion(sug);
        self.diagnostics.push(d);
        Diagnostic::new(code, Severity::Error, "type error", msg, line, col)
            .with_span(span.start, span.end)
            .with_suggestion(String::new())
    }

    // ---- type collection ----

    fn collect(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Struct { name, fields, attributes, .. } => {
                    let tys: Vec<(String, Ty)> = fields
                        .iter()
                        .map(|(n, _, ty)| (n.clone(), ty.as_ref().map(|t| self.type_of_type(t)).unwrap_or(Ty::Unknown)))
                        .collect();
                    self.structs.insert(name.clone(), tys);
                    if attributes.iter().any(|a| a == "not_send") {
                        self.not_send.insert(name.clone());
                    }
                }
                HirItem::Enum { name, variants, .. } => {
                    self.enums.insert(name.clone(), variants.clone());
                    for v in variants {
                        self.enum_variants.insert(v.clone(), name.clone());
                    }
                }
                HirItem::Fn(f) => {
                    let params: Vec<Ty> = f
                        .params
                        .iter()
                        .map(|p| p.ty.as_ref().map(|t| self.type_of_type(t)).unwrap_or(Ty::Unknown))
                        .collect();
                    let ret = match &f.ret {
                        Some(t) => self.type_of_type(t),
                        None => Ty::Unit,
                    };
                    let arity = params.len();
                    self.fns.insert(f.name.clone(), FnSig { params, ret, arity });
                }
                HirItem::Mod { name, items, .. } => {
                    if name.starts_with("impl|") {
                        // "impl|<interface>|<type>" → impls 表；方法体照常收集
                        let parts: Vec<&str> = name.split('|').collect();
                        if parts.len() >= 3 && !parts[1].is_empty() {
                            self.impls.entry(parts[1].to_string()).or_default().push(parts[2].to_string());
                        }
                    }
                    self.collect(items);
                }
                _ => {}
            }
        }
    }

    /// Resolve an ast::Type to Ty (Flow_Type_System.md section 1).
    fn type_of_type(&self, t: &Type) -> Ty {
        match t {
            Type::Path(p) => {
                let name = p.segments.join("::");
                let args: Vec<Ty> = p
                    .generics
                    .first()
                    .map(|gs| gs.iter().map(|g| self.type_of_type(g)).collect())
                    .unwrap_or_default();
                match name.as_str() {
                    "i32" => Ty::I32,
                    "i64" => Ty::I64,
                    "u8" => Ty::U8,
                    "u16" => Ty::U16,
                    "u32" => Ty::U32,
                    "u64" => Ty::U64,
                    "usize" => Ty::Usize,
                    "f32" => Ty::F32,
                    "f64" => Ty::F64,
                    "bool" => Ty::Bool,
                    "char" => Ty::Char,
                    "String" => Ty::Str,
                    "Vec" => Ty::Vec(Box::new(args.first().cloned().unwrap_or(Ty::Unknown))),
                    "Map" => Ty::Map(Box::new(args.get(1).cloned().unwrap_or(Ty::Unknown))),
                    "Option" => Ty::OptionT(Box::new(args.first().cloned().unwrap_or(Ty::Unknown))),
                    "Result" => {
                        let a = args.first().cloned().unwrap_or(Ty::Unknown);
                        let b = args.get(1).cloned().unwrap_or(Ty::Unknown);
                        Ty::ResultT(Box::new(a), Box::new(b))
                    }
                    "CStr" | "CString" | "CArray" => Ty::Unknown,
                    _ => Ty::Named(name),
                }
            }
            Type::Fn { params, ret } => {
                let ps: Vec<Ty> = params.iter().map(|t| self.type_of_type(t)).collect();
                let r = ret.as_ref().map(|r| Box::new(self.type_of_type(r)));
                Ty::Fn(ps, r)
            }
            Type::Ref { ty, mutable } => Ty::Ref(Box::new(self.type_of_type(ty)), *mutable),
            Type::Tuple(tys) => {
                let inner: Vec<Ty> = tys.iter().map(|t| self.type_of_type(t)).collect();
                if inner.is_empty() {
                    Ty::Unit
                } else {
                    Ty::Tuple(inner)
                }
            }
            Type::Infer => Ty::Unknown,
        }
    }

    // ---- items ----

    fn check_item(&mut self, item: &HirItem) {
        match item {
            HirItem::Fn(f) => self.check_fn(f),
            HirItem::Ui(u) => self.check_ui(u),
            HirItem::Global { init, .. } => {
                if let Some(e) = init {
                    self.infer_expr(e);
                }
            }
            HirItem::Mod { items, .. } => {
                for it in items {
                    self.check_item(it);
                }
            }
            _ => {}
        }
    }

    fn check_fn(&mut self, f: &crate::hir::FnInfo) {
        self.push_scope();
        // param types come from the collected signature
        let sig = self.fns.get(&f.name).cloned();
        for (i, p) in f.params.iter().enumerate() {
            let t = sig
                .as_ref()
                .and_then(|s| s.params.get(i))
                .cloned()
                .unwrap_or(Ty::Unknown);
            self.set_env(p.sym, t);
        }
        let declared = match &f.ret {
            Some(t) => Some(self.type_of_type(t)),
            None => None,
        };
        let saved_ret = self.cur_ret.take();
        self.cur_ret = declared.clone();
        self.check_block_inner(&f.body, true);
        self.cur_ret = saved_ret;
        self.pop_scope();
    }

    fn check_ui(&mut self, u: &crate::hir::UiInfo) {
        self.push_scope();
        // states: type from annotation or init
        for st in &u.states {
            let ann = st.ty.as_ref().map(|t| self.type_of_type(t));
            let init_ty = st.init.as_ref().map(|e| self.infer_expr(e)).unwrap_or(Ty::Unknown);
            let t = ann.clone().unwrap_or(init_ty);
            self.set_env(st.sym, t);
        }
        if let Some(render) = &u.render {
            self.check_block(render);
        }
        self.pop_scope();
    }
    // ---- environment ----

    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.env.pop();
    }

    fn set_env(&mut self, sym: SymbolId, ty: Ty) {
        if let Some(frame) = self.env.last_mut() {
            frame.insert(sym, ty.clone());
        }
        self.binding_types.insert(sym, ty);
    }

    fn env_type(&self, sym: SymbolId) -> Ty {
        for frame in self.env.iter().rev() {
            if let Some(t) = frame.get(&sym) {
                return t.clone();
            }
        }
        Ty::Unknown
    }

    fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.hir.symbols.get(id)
    }

    fn is_mut_binding(&self, id: SymbolId) -> bool {
        self.symbol(id).map(|s| s.is_mut).unwrap_or(true)
    }

    // ---- blocks & statements ----

    fn check_block(&mut self, block: &ast::Block) {
        self.check_block_inner(block, false);
    }

    /// Check a block; is_fn_body enables the tail-vs-return check (which
    /// must run while the block's bindings are still in scope).
    fn check_block_inner(&mut self, block: &ast::Block, is_fn_body: bool) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let tail_ty = if let Some(tail) = &block.tail {
            let t = self.infer_expr(tail);
            self.types.insert(block.span, t.clone());
            t
        } else {
            let t = Ty::Unit;
            self.types.insert(block.span, t.clone());
            t
        };
        // fn tail vs declared return — only at function-body level
        if is_fn_body {
            let cur_ret = self.cur_ret.clone();
            if let Some(ret) = &cur_ret {
                if let Some(tail) = &block.tail {
                    let t = self.infer_expr_preserve(tail);
                    if !Ty::assignable(&t, ret) {
                        self.err(
                            codes::RETURN_TYPE_MISMATCH,
                            &format!("函数体尾表达式类型不匹配：期望 {}，实际 {}", ret.display(), t.display()),
                            expr_span(tail),
                        );
                    }
                }
            }
        }
        self.pop_scope();
        let _ = tail_ty;
    }

    /// infer an expression again (type map only, no side effects)
    fn infer_expr_preserve(&mut self, expr: &Expr) -> Ty {
        self.infer_expr(expr)
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let(ls) => {
                let ann = ls.ty.as_ref().map(|t| self.type_of_type(t));
                let init_ty = ls.init.as_ref().map(|e| self.infer_expr(e)).unwrap_or(Ty::Unknown);
                let final_ty = ann.clone().unwrap_or(init_ty.clone());
                if let Some(sym) = self.resolution.bindings.get(&ls.span) {
                    self.set_env(*sym, final_ty.clone());
                }
                if let (Some(a), i) = (ann, init_ty) {
                    if !Ty::assignable(&i, &a) {
                        let sug = match (&a, &i) {
                            (Ty::Str, _) => "右侧是字符串。若需要文本请把标注改为 String；若需要数字请检查是否漏了转换".to_string(),
                            (Ty::I32 | Ty::I64, Ty::Str) => "右侧是字符串字面量，不能直接当数字用。若要转换请提供显式的转换方法".to_string(),
                            _ => format!("把标注改为 {}，或修改右侧表达式使其产出 {}", i.display(), a.display()),
                        };
                        self.err_sug(
                            codes::LET_TYPE_MISMATCH,
                            &format!("类型不匹配：标注为 {}，初始化值为 {}", a.display(), i.display()),
                            ls.span, sug,
                        );
                    }
                }
            }
            Stmt::Expr(e) => {
                self.infer_expr(e);
            }
            Stmt::Return(v) => {
                if let Some(e) = v {
                    let t = self.infer_expr(e);
                    if let Some(ret) = self.cur_ret.clone() {
                        if !Ty::assignable(&t, &ret) {
                            self.err_sug(
                                codes::RETURN_TYPE_MISMATCH,
                                &format!("返回值类型不匹配：期望 {}，实际 {}", ret.display(), t.display()),
                                expr_span(e),
                                format!("函数声明返回 {}。修改 return 表达式，或修改函数签名的返回类型", ret.display()),
                            );
                        }
                    }
                }
            }
            Stmt::If(ifexpr) => {
                self.check_if(ifexpr);
            }
            Stmt::While { cond, body, span } => {
                let t = self.infer_expr(cond);
                if !matches!(t, Ty::Bool | Ty::Unknown | Ty::Error) {
                    self.err_sug(
                        codes::NON_BOOL_CONDITION,
                        &format!("while 条件必须是 bool，实际为 {}", t.display()),
                        fallback_span(expr_span(cond), *span),
                        "if/while 的条件必须是布尔值。比较表达式（a > b）、bool 变量或逻辑组合（&& ||）都可以".to_string(),
                    );
                }
                self.check_block(body);
            }
            Stmt::For { pat, iter, body, .. } => {
                let it = self.infer_expr(iter);
                let elem = match &it {
                    Ty::Vec(e) => (**e).clone(),
                    _ => Ty::Unknown,
                };
                self.push_scope();
                if let Some(sym) = self.resolution.bindings.get(&pat_span(pat)) {
                    self.set_env(*sym, elem);
                }
                self.check_block(body);
                self.pop_scope();
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Defer(e) => {
                self.infer_expr(e);
            }
            Stmt::TryCatch(tc) => {
                self.check_block(&tc.try_block);
                if let Some(p) = &tc.catch_param {
                    self.push_scope();
                    if let Some(sym) = self.resolution.bindings.get(&p.span) {
                        self.set_env(*sym, Ty::Unknown);
                    }
                    self.check_block(&tc.catch_block);
                    self.pop_scope();
                } else {
                    self.check_block(&tc.catch_block);
                }
            }
            Stmt::Block(b) => self.check_block(b),
        }
    }

    // ---- expressions ----

    fn infer_expr(&mut self, expr: &Expr) -> Ty {
        let ty = match expr {
            Expr::Int(_) => Ty::I32,
            Expr::Float(_) => Ty::F64,
            Expr::Str(_) | Expr::FmtStr(_) => Ty::Str,
            Expr::Bool(_) => Ty::Bool,
            Expr::Ident { span, .. } => self.infer_ident(*span),
            Expr::Path(_) => Ty::Unknown,
            Expr::Array(items) => {
                let elem = items
                    .iter()
                    .map(|e| self.infer_expr(e))
                    .find(|t| !matches!(t, Ty::Unknown))
                    .unwrap_or(Ty::Unknown);
                Ty::Vec(Box::new(elem))
            }
            Expr::Tuple(items) => {
                let tys: Vec<Ty> = items.iter().map(|e| self.infer_expr(e)).collect();
                if tys.is_empty() {
                    Ty::Unit
                } else {
                    Ty::Tuple(tys)
                }
            }
            Expr::StructLit { name, fields } => self.check_struct_lit(name, fields),
            Expr::Unary { op, expr } => {
                let t = self.infer_expr(expr);
                match op {
                    crate::ast::UnOp::Neg => {
                        if !matches!(t, Ty::Unknown | Ty::Error) && !t.is_numeric() {
                            self.err(
                                codes::BINOP_TYPE_MISMATCH,
                                &format!("一元负号要求数值，实际为 {}", t.display()),
                                expr_span(expr),
                            );
                            Ty::Error
                        } else {
                            t
                        }
                    }
                    crate::ast::UnOp::Not => {
                        if matches!(t, Ty::Bool | Ty::Unknown | Ty::Error) {
                            Ty::Bool
                        } else {
                            self.err(
                                codes::BINOP_TYPE_MISMATCH,
                                &format!("逻辑非要求 bool，实际为 {}", t.display()),
                                expr_span(expr),
                            );
                            Ty::Error
                        }
                    }
                    crate::ast::UnOp::Ref => Ty::Ref(Box::new(t), false),
                    crate::ast::UnOp::Deref => match t {
                        Ty::Ref(inner, _) => *inner,
                        _ => Ty::Unknown,
                    },
                }
            }
            Expr::Binary { op, lhs, rhs } => self.check_binop(*op, lhs, rhs),
            Expr::Assign { op, target, value } => self.check_assign(*op, target, value),
            Expr::Call { callee, args, span, .. } => self.check_call(callee, args, *span),
            Expr::Field { base, field, span } => {
                let bt = self.infer_expr(base);
                let ft = match &bt {
                    Ty::Named(s) => self
                        .structs
                        .get(s)
                        .and_then(|fields| fields.iter().find(|(n, _)| n == field).map(|(_, t)| t.clone()))
                        .unwrap_or(Ty::Unknown),
                    _ => Ty::Unknown,
                };
                self.types.insert(*span, ft.clone());
                ft
            }
            Expr::Index { base, index } => {
                self.infer_expr(index);
                match self.infer_expr(base) {
                    Ty::Vec(e) => {
                        if matches!(&**index, Expr::Range { .. }) {
                            Ty::Vec(e.clone())
                        } else {
                            *e
                        }
                    }
                    _ => Ty::Unknown,
                }
            }
            Expr::Try(e) => {
                let t = self.infer_expr(e);
                match t {
                    Ty::OptionT(inner) => *inner,
                    Ty::ResultT(inner, _) => *inner,
                    Ty::Unknown => Ty::Unknown,
                    other => {
                        self.err(
                            codes::BAD_TRY,
                            &format!("问号运算符只能用于 Option/Result，实际为 {}", other.display()),
                            expr_span(e),
                        );
                        Ty::Error
                    }
                }
            }
            Expr::Range { .. } => Ty::Unknown,
            Expr::Block(b) => {
                self.check_block(b);
                self.types.get(&b.span).cloned().unwrap_or(Ty::Unknown)
            }
            Expr::If(ifexpr) => self.check_if_expr(ifexpr),
            Expr::Match { scrutinee, arms, span } => {
                self.infer_expr(scrutinee);
                let mut result = Ty::Unknown;
                for arm in arms {
                    self.push_scope();
                    for (_, span) in arm.pat.bindings() {
                        if let Some(sym) = self.resolution.bindings.get(&span) {
                            self.set_env(*sym, Ty::Unknown);
                        }
                    }
                    let t = self.infer_expr(&arm.value);
                    // arms that diverge (block ending in return) never produce
                    // a value — they must not poison the match result type
                    let diverges = matches!(&arm.value, Expr::Block(b) if b.tail.is_none() && !b.stmts.is_empty());
                    if !diverges {
                        if matches!(result, Ty::Unknown) {
                            result = t;
                        } else if !Ty::assignable(&t, &result) && !matches!(t, Ty::Unknown) {
                            result = Ty::Unknown;
                        }
                    }
                    self.pop_scope();
                }
                self.types.insert(*span, result.clone());
                result
            }
            Expr::Closure { body, .. } => {
                self.infer_expr(body);
                Ty::Unknown
            }
            Expr::Go { block, .. } => {
                self.check_block(block);
                Ty::Unknown
            }
            Expr::UiOp { block, .. } => {
                self.check_block(block);
                Ty::Unknown
            }
            Expr::Paren(e) => self.infer_expr(e),
        };
        // record type for span-bearing nodes
        match expr {
            Expr::Ident { span, .. } => {
                self.types.insert(*span, ty.clone());
            }
            Expr::Call { span, .. } => {
                self.types.insert(*span, ty.clone());
            }
            Expr::Path(p) => {
                self.types.insert(p.span, ty.clone());
            }
            Expr::StructLit { name, .. } => {
                self.types.insert(name.span, ty.clone());
            }
            _ => {}
        }
        ty
    }

    fn infer_ident(&mut self, span: Span) -> Ty {
        match self.resolution.get(&span) {
            Some(sym) => {
                let kind = self.symbol(sym).map(|s| s.kind).unwrap_or(SymbolKind::Local);
                match kind {
                    SymbolKind::Param
                    | SymbolKind::Local
                    | SymbolKind::State
                    | SymbolKind::Prop
                    | SymbolKind::ClosureParam
                    | SymbolKind::ForPattern
                    | SymbolKind::Global => self.env_type(sym),
                    _ => Ty::Unknown,
                }
            }
            None => Ty::Unknown,
        }
    }

    fn check_binop(&mut self, op: crate::ast::BinOp, lhs: &Expr, rhs: &Expr) -> Ty {
        use crate::ast::BinOp;
        let lt = self.infer_expr(lhs);
        let rt = self.infer_expr(rhs);
        let any_unknown =
            matches!(lt, Ty::Unknown | Ty::Error) || matches!(rt, Ty::Unknown | Ty::Error);
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                // string concatenation with +
                if matches!(op, BinOp::Add) {
                    if matches!(lt, Ty::Str) && matches!(rt, Ty::Str) {
                        return Ty::Str;
                    }
                }
                if !any_unknown && (!lt.is_numeric() || !rt.is_numeric()) {
                    self.err(
                        codes::BINOP_TYPE_MISMATCH,
                        &format!(
                            "算术运算要求数值：{} {} {}",
                            lt.display(),
                            op_name(op),
                            rt.display()
                        ),
                        expr_span(lhs),
                    );
                    return Ty::Error;
                }
                if matches!(lt, Ty::F64 | Ty::F32) || matches!(rt, Ty::F64 | Ty::F32) {
                    Ty::F64
                } else if lt.is_integer() && rt.is_integer() {
                    lt
                } else {
                    Ty::Unknown
                }
            }
            BinOp::Eq | BinOp::Ne => {
                // char 与 String 字面量可比较（字符与 1 字符字符串）
                let char_str_mix = matches!((&lt, &rt), (Ty::Char, Ty::Str) | (Ty::Str, Ty::Char));
                if !any_unknown && !char_str_mix && !Ty::assignable(&lt, &rt) && !Ty::assignable(&rt, &lt) {
                    self.err(
                        codes::BINOP_TYPE_MISMATCH,
                        &format!("无法比较不同类型：{} 和 {}", lt.display(), rt.display()),
                        expr_span(lhs),
                    );
                    return Ty::Error;
                }
                Ty::Bool
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let both_str = matches!(lt, Ty::Str) && matches!(rt, Ty::Str);
                let char_str_mix = matches!((&lt, &rt), (Ty::Char, Ty::Str) | (Ty::Str, Ty::Char));
                if !any_unknown && !both_str && !char_str_mix && (!lt.is_numeric() || !rt.is_numeric()) {
                    self.err(
                        codes::BINOP_TYPE_MISMATCH,
                        &format!("比较运算要求数值或同为 String：{} 和 {}", lt.display(), rt.display()),
                        expr_span(lhs),
                    );
                    return Ty::Error;
                }
                Ty::Bool
            }
            BinOp::And | BinOp::Or => {
                if !any_unknown && (lt != Ty::Bool || rt != Ty::Bool) {
                    self.err(
                        codes::BINOP_TYPE_MISMATCH,
                        &format!("逻辑运算要求 bool：{} 和 {}", lt.display(), rt.display()),
                        expr_span(lhs),
                    );
                    return Ty::Error;
                }
                Ty::Bool
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if !any_unknown && (!lt.is_integer() || !rt.is_integer()) {
                    self.err(
                        codes::BINOP_TYPE_MISMATCH,
                        &format!("位运算要求整数：{} 和 {}", lt.display(), rt.display()),
                        expr_span(lhs),
                    );
                    return Ty::Error;
                }
                lt
            }
            BinOp::Range => Ty::Unknown,
        }
    }

    fn check_assign(&mut self, op: crate::ast::AssignOp, target: &Expr, value: &Expr) -> Ty {
        match target {
            Expr::Ident { span, .. } => {
                let target_sym = self.resolution.get(span);
                if let Some(sym) = target_sym {
                    if !self.is_mut_binding(sym) {
                        let name = match target {
                            Expr::Ident { name, .. } => name.clone(),
                            _ => String::new(),
                        };
                        self.err(
                            codes::ASSIGN_TO_IMMUTABLE,
                            &format!("不能给不可变绑定 '{}' 赋值（需要 var）", name),
                            *span,
                        );
                    }
                }
            }
            // v[i] = x / m[k] = v：与整体赋值同一 mut 资格模型
            Expr::Index { base, .. } => {
                if let Expr::Ident { span, name, .. } = &**base {
                    if let Some(sym) = self.resolution.get(span) {
                        if !self.is_mut_binding(sym) {
                            self.err(
                                codes::ASSIGN_TO_IMMUTABLE,
                                &format!(
                                    "不能给不可变绑定 '{}' 的元素赋值（需要 var）",
                                    name
                                ),
                                *span,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
        let tt = self.infer_expr(target);
        let vt = self.infer_expr(value);
        if op == crate::ast::AssignOp::Assign {
            if !Ty::assignable(&vt, &tt) && !matches!(tt, Ty::Unknown | Ty::Error) {
                self.err(
                    codes::ASSIGN_TYPE_MISMATCH,
                    &format!("赋值类型不匹配：目标 {}，值 {}", tt.display(), vt.display()),
                    expr_span(value),
                );
            }
        } else if !matches!(tt, Ty::Unknown | Ty::Error) && !tt.is_numeric() {
            self.err(
                codes::ASSIGN_TYPE_MISMATCH,
                &format!("复合赋值要求数值目标，实际为 {}", tt.display()),
                expr_span(target),
            );
        }
        tt
    }

    /// interface 约束：期望 Named(I) 且 at 是 I 的实现类型
    fn assignable_via_interface(&self, at: &Ty, expected: &Ty) -> bool {
        match (at, expected) {
            (Ty::Named(actual), Ty::Named(iface)) => {
                self.impls.get(iface).map(|list| list.contains(actual)).unwrap_or(false)
            }
            _ => false,
        }
    }

    fn check_call(&mut self, callee: &Expr, args: &[crate::ast::CallArg], span: Span) -> Ty {
        if let Expr::Field { base, .. } = callee {
            self.infer_expr(base);
            for a in args {
                self.infer_expr(&a.expr);
            }
            return Ty::Unknown;
        }
        let callee_name = match callee {
            Expr::Ident { name, .. } => Some(name.clone()),
            _ => None,
        };
        if let Some(name) = callee_name {
            if let Some(enum_ty) = self.enum_variants.get(&name).cloned() {
                // user enum variant constructor
                for a in args {
                    self.infer_expr(&a.expr);
                }
                return Ty::Named(enum_ty);
            }
            if let Some(sig) = self.fns.get(&name).cloned() {
                if args.len() != sig.arity {
                    let sug = if args.len() < sig.arity {
                        format!("缺少 {} 个实参。{} 的参数是：{}",
                            sig.arity - args.len(), name,
                            sig.params.iter().map(|p| p.display()).collect::<Vec<_>>().join(", "))
                    } else {
                        format!("多传了 {} 个实参。{} 只接受：{}",
                            args.len() - sig.arity, name,
                            sig.params.iter().map(|p| p.display()).collect::<Vec<_>>().join(", "))
                    };
                    self.err_sug(
                        codes::ARG_COUNT_MISMATCH,
                        &format!(
                            "参数个数不匹配：{} 期望 {} 个参数，实际传了 {} 个",
                            name,
                            sig.arity,
                            args.len()
                        ),
                        span, sug,
                    );
                }
                for (a, expected) in args.iter().zip(sig.params.iter()) {
                    let at = self.infer_expr(&a.expr);
                    let ok = Ty::assignable(&at, expected)
                        || self.assignable_via_interface(&at, expected);
                    if !ok && !matches!(at, Ty::Unknown | Ty::Error) {
                        let sug = if matches!(expected, Ty::Str) && matches!(at, Ty::I32 | Ty::I64 | Ty::F64) {
                            "该参数需要 String。数字不能隐式转为字符串，请先转换".to_string()
                        } else {
                            format!("该参数需要 {}", expected.display())
                        };
                        self.err_sug(
                            codes::ASSIGN_TYPE_MISMATCH,
                            &format!(
                                "参数类型不匹配：期望 {}，实际 {}",
                                expected.display(),
                                at.display()
                            ),
                            a.span, sug,
                        );
                    }
                }
                return sig.ret;
            }
        }
        for a in args {
            self.infer_expr(&a.expr);
        }
        Ty::Unknown
    }

    fn check_struct_lit(&mut self, name: &crate::ast::Path, fields: &[(String, Expr)]) -> Ty {
        let type_name = name.segments.join("::");
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (fname, _) in fields {
            if !seen.insert(fname.clone()) {
                self.err(
                    codes::DUPLICATE_FIELD,
                    &format!("结构体字面量中字段 '{}' 重复", fname),
                    name.span,
                );
            }
        }
        let field_defs = self.structs.get(&type_name).cloned();
        match field_defs {
            Some(defs) => {
                for (fname, fexpr) in fields {
                    match defs.iter().find(|(n, _)| n == fname) {
                        Some((_, expected)) => {
                            let at = self.infer_expr(fexpr);
                            if !Ty::assignable(&at, expected)
                                && !matches!(at, Ty::Unknown | Ty::Error)
                            {
                                self.err(
                                    codes::FIELD_TYPE_MISMATCH,
                                    &format!(
                                        "字段 '{}' 类型不匹配：期望 {}，实际 {}",
                                        fname,
                                        expected.display(),
                                        at.display()
                                    ),
                                    name.span,
                                );
                            }
                        }
                        None => {
                            self.err(
                                codes::UNKNOWN_FIELD,
                                &format!("类型 '{}' 没有字段 '{}'", type_name, fname),
                                name.span,
                            );
                        }
                    }
                }
                Ty::Named(type_name)
            }
            None => {
                for (_, fexpr) in fields {
                    self.infer_expr(fexpr);
                }
                Ty::Named(type_name)
            }
        }
    }

    fn check_if(&mut self, ifexpr: &crate::ast::IfExpr) {
        let cond = self.infer_expr(&ifexpr.cond);
        if !matches!(cond, Ty::Bool | Ty::Unknown | Ty::Error) {
            self.err_sug(
                codes::NON_BOOL_CONDITION,
                &format!("if 条件必须是 bool，实际为 {}", cond.display()),
                fallback_span(expr_span(&ifexpr.cond), ifexpr.span),
                "if/while 的条件必须是布尔值。比较表达式（a > b）、bool 变量或逻辑组合（&& ||）都可以".to_string(),
            );
        }
        self.check_block(&ifexpr.then_branch);
        if let Some(e) = &ifexpr.else_branch {
            self.infer_expr(e);
        }
    }

    fn check_if_expr(&mut self, ifexpr: &crate::ast::IfExpr) -> Ty {
        self.check_if(ifexpr);
        Ty::Unknown
    }
}

// ---- helpers ----

fn op_name(op: crate::ast::BinOp) -> &'static str {
    use crate::ast::BinOp;
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        _ => "?",
    }
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::Ident { span, .. } => *span,
        Expr::Path(p) => p.span,
        Expr::Call { span, .. } => *span,
        Expr::Field { span, .. } => *span,
        Expr::Block(b) => b.span,
        Expr::Match { span, .. } => *span,
        Expr::Go { span, .. } => *span,
        Expr::UiOp { span, .. } => *span,
        Expr::Closure { span, .. } => *span,
        Expr::If(ifx) => ifx.span,
        Expr::Array(items) => span_of_first_last(items),
        Expr::Tuple(items) => span_of_first_last(items),
        Expr::StructLit { name, .. } => name.span,
        Expr::Index { base, index } => Span::new(expr_span(base).start, expr_span(index).end),
        Expr::Unary { expr, .. } => expr_span(expr),
        Expr::Binary { lhs, rhs, .. } => Span::new(expr_span(lhs).start, expr_span(rhs).end),
        Expr::Try(e2) => expr_span(e2),
        _ => Span::new(0, 0),
    }
}

/// 主 span 无效（0,0）时用备选 span
fn fallback_span(primary: Span, fallback: Span) -> Span {
    if primary.start == 0 && primary.end == 0 {
        fallback
    } else {
        primary
    }
}

/// 从表达式列表取首尾 span 的合成范围（Array/Tuple 用）
fn span_of_first_last(items: &[Expr]) -> Span {
    if items.is_empty() {
        return Span::new(0, 0);
    }
    let s = expr_span(&items[0]);
    let e = expr_span(&items[items.len() - 1]);
    Span::new(s.start.min(e.start), s.end.max(e.end))
}

/// 内部递归（返回 0 span 也不 panic）
#[allow(dead_code)]
fn expr_span2(e: &Expr) -> Span {
    let sp = expr_span(e);
    if sp.start == 0 && sp.end == 0 {
        Span::new(0, 1)
    } else {
        sp
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
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Resolver;

    fn typeck_src(src: &str) -> (TypeckResult, DiagnosticSink) {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let resolver = Resolver::new(&lexed.tokens);
        let resolved = resolver.resolve(&program);
        let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
        (checker.check(), resolved.diagnostics)
    }

    fn err_count(t: &TypeckResult) -> usize {
        t.diagnostics.error_count()
    }

    fn has_code(t: &TypeckResult, code: &str) -> bool {
        t.diagnostics.diagnostics.iter().any(|d| d.code == code)
    }

    #[test]
    fn let_annotation_mismatch() {
        let (t, _) = typeck_src("fn f() { let x: i32 = \"hello\" }");
        assert!(has_code(&t, codes::LET_TYPE_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn binop_mismatch() {
        let (t, _) = typeck_src("fn f() { let x = 1 + \"two\" }");
        assert!(has_code(&t, codes::BINOP_TYPE_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn non_bool_condition() {
        let (t, _) = typeck_src("fn f() { if 42 { } }");
        assert!(has_code(&t, codes::NON_BOOL_CONDITION), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn return_mismatch() {
        let (t, _) = typeck_src("fn f() -> i32 { let x: String = \"a\"\nx }");
        assert!(has_code(&t, codes::RETURN_TYPE_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn assign_type_mismatch() {
        let (t, _) = typeck_src("fn f() { var x: i32 = 1\nx = \"str\" }");
        assert!(has_code(&t, codes::ASSIGN_TYPE_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn assign_to_immutable() {
        let (t, _) = typeck_src("fn f() { let x = 1\nx = 2 }");
        assert!(has_code(&t, codes::ASSIGN_TO_IMMUTABLE), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn unknown_struct_field() {
        let (t, _) = typeck_src("struct User { name: String }\nfn f() { let u = User { age: 3 } }");
        assert!(has_code(&t, codes::UNKNOWN_FIELD), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn field_type_mismatch() {
        let (t, _) = typeck_src("struct User { name: String }\nfn f() { let u = User { name: 42 } }");
        assert!(has_code(&t, codes::FIELD_TYPE_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn arg_count_mismatch() {
        let (t, _) = typeck_src("fn g(a: i32, b: i32) { }\nfn f() { g(1) }");
        assert!(has_code(&t, codes::ARG_COUNT_MISMATCH), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn duplicate_struct_literal_fields() {
        let (t, _) = typeck_src("struct U { a: i32 }\nfn f() { let u = U { a: 1 a: 2 } }");
        assert!(has_code(&t, codes::DUPLICATE_FIELD), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn bad_try_usage() {
        let (t, _) = typeck_src("fn f() { let x = 42? }");
        assert!(has_code(&t, codes::BAD_TRY), "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn correct_program_passes() {
        let (t, _) = typeck_src(
            "fn add(a: i32, b: i32) -> i32 { a + b }\nfn main() { let r = add(1, 2)\nif r > 0 { print(r) } }",
        );
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn literal_types_inferred() {
        // literal values flow into binding types; references carry them in the map
        let (t, _) = typeck_src(
            "fn f() { let a = 1\nlet b = 1.5\nlet c = \"s\"\nlet d = true\nlet e = [1, 2, 3]\nlet x = a\nlet y = b\nlet z = c\nlet w = d\nlet v = e }",
        );
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
        assert!(t.types.values().any(|ty| *ty == Ty::I32));
        assert!(t.types.values().any(|ty| *ty == Ty::F64));
        assert!(t.types.values().any(|ty| *ty == Ty::Str));
        assert!(t.types.values().any(|ty| *ty == Ty::Bool));
        assert!(t.types.values().any(|ty| matches!(ty, Ty::Vec(_))));
    }

    #[test]
    fn call_return_type_used() {
        let (t, _) = typeck_src("fn g() -> i32 { 5 }\nfn f() { let x = g() }");
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
        assert!(t.types.values().any(|ty| *ty == Ty::I32));
    }

    #[test]
    fn struct_literal_typed() {
        let (t, _) = typeck_src("struct User { name: String }\nfn f() { let u = User { name: \"a\" } }");
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
        assert!(t.types.values().any(|ty| *ty == Ty::Named("User".into())));
    }

    #[test]
    fn unknown_method_condition_ok() {
        let (t, _) = typeck_src("fn f() { if data.valid() { } }");
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn vec_state_with_empty_init_ok() {
        let src = "ui Counter {\n    @state\n    var records: Vec<i32> = []\n    render {\n        Text(\"x\")\n    }\n}";
        let (t, _) = typeck_src(src);
        assert_eq!(err_count(&t), 0, "{:?}", t.diagnostics.diagnostics);
    }

    #[test]
    fn examples_typecheck_clean() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let lexed = Lexer::new(&src).lex();
            let parsed = Parser::new(&lexed.tokens, name).parse_program();
            let program = parsed.program.unwrap();
            let resolver = Resolver::new(&lexed.tokens);
            let resolved = resolver.resolve(&program);
            let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
            let t = checker.check();
            assert_eq!(err_count(&t), 0, "{} type errors: {:?}", name, t.diagnostics.diagnostics);
            assert!(
                !resolved.diagnostics.has_errors(),
                "{} semantic errors: {:?}",
                name,
                resolved.diagnostics.diagnostics
            );
        }
    }
}
