//! Formatter (M1.5): canonical pretty-printer for Aine source.
//!
//! Emits the AST back to canonical Aine source, following the Grammar
//! specification (Flow_Language_Grammar.md §1.1 whitespace, §3 soft newlines):
//! - 4-space indentation (design doc §69 style);
//! - one statement per line (newline-terminated);
//! - blocks braces on their own lines with indented bodies;
//! - expressions inline with minimal parentheses (re-derived from
//!   precedence, so formatting preserves semantics);
//! - trailing blocks/closures after calls (Button("x") { }, List(x) |r| { }).
//!
//! Known limitation: comments are not preserved (AST does not carry them).
//! Round-trip guarantee: format(x) re-parses with 0 errors and is idempotent.

use crate::ast::{self, Expr, Item, Pattern, Stmt, Type};

/// Operator precedence levels (must match parser.rs binding powers).
const P_ASSIGN: u8 = 1;
const P_RANGE: u8 = 2;
const P_OR: u8 = 3;
const P_AND: u8 = 4;
const P_EQ: u8 = 5;
const P_BITOR: u8 = 6;
const P_BITAND: u8 = 7;
const P_CMP: u8 = 9;
const P_BITXOR: u8 = 11;
const P_SHIFT: u8 = 11;
const P_ADD: u8 = 13;
const P_MUL: u8 = 15;
const P_UNARY: u8 = 17;
const P_POSTFIX: u8 = 19;
const P_PRIMARY: u8 = 20;

const INDENT: &str = "    ";

pub struct Formatter {
    out: String,
    depth: usize,
}

impl Formatter {
    pub fn new() -> Self {
        Formatter { out: String::new(), depth: 0 }
    }

    pub fn format_program(mut self, program: &ast::Program) -> String {
        for (i, item) in program.items.iter().enumerate() {
            if i > 0 {
                self.out.push('\n');
            }
            self.fmt_item(item);
        }
        self.out
    }

    // ---- helpers ----

    fn line(&mut self) {
        self.out.push('\n');
        for _ in 0..self.depth {
            self.out.push_str(INDENT);
        }
    }

    fn push(&mut self) {
        self.depth += 1;
    }

    fn pop(&mut self) {
        self.depth -= 1;
    }

    fn fmt_block(&mut self, b: &ast::Block) {
        self.out.push_str("{");
        let has_body = !b.stmts.is_empty() || b.tail.is_some();
        if !has_body {
            self.out.push_str(" }");
            return;
        }
        self.push();
        for s in &b.stmts {
            self.line();
            self.fmt_stmt(s);
        }
        if let Some(t) = &b.tail {
            self.line();
            self.fmt_expr(t, 0);
        }
        self.pop();
        self.line();
        self.out.push_str("}");
    }

    fn fmt_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let(ls) => {
                if ls.is_state {
                    self.out.push_str("@state ");
                }
                if ls.is_mut {
                    self.out.push_str("var ");
                } else {
                    self.out.push_str("let ");
                }
                self.fmt_pattern(&ls.pat);
                if let Some(t) = &ls.ty {
                    self.out.push_str(": ");
                    self.fmt_type(t);
                }
                if let Some(init) = &ls.init {
                    self.out.push_str(" = ");
                    self.fmt_expr(init, 0);
                }
            }
            Stmt::Expr(e) => self.fmt_expr(e, 0),
            Stmt::Return(v) => {
                self.out.push_str("return");
                if let Some(e) = v {
                    self.out.push(' ');
                    self.fmt_expr(e, 0);
                }
            }
            Stmt::If(ifexpr) => self.fmt_if(ifexpr),
            Stmt::While { cond, body, .. } => {
                self.out.push_str("while ");
                self.fmt_expr(cond, 0);
                self.out.push(' ');
                self.fmt_block(body);
            }
            Stmt::For { pat, iter, body, .. } => {
                self.out.push_str("for ");
                self.fmt_pattern(pat);
                self.out.push_str(" in ");
                self.fmt_expr(iter, 0);
                self.out.push(' ');
                self.fmt_block(body);
            }
            Stmt::Break(_) => self.out.push_str("break"),
            Stmt::Continue(_) => self.out.push_str("continue"),
            Stmt::Defer(e) => {
                self.out.push_str("defer ");
                self.fmt_expr(e, 0);
            }
            Stmt::TryCatch(tc) => {
                self.out.push_str("try ");
                self.fmt_block(&tc.try_block);
                self.out.push(' ');
                self.out.push_str("catch");
                if let Some(p) = &tc.catch_param {
                    self.out.push_str(" (");
                    self.out.push_str(&p.name);
                    if let Some(t) = &p.ty {
                        self.out.push_str(": ");
                        self.fmt_type(t);
                    }
                    self.out.push(')');
                }
                self.out.push(' ');
                self.fmt_block(&tc.catch_block);
            }
            Stmt::Block(b) => self.fmt_block(b),
        }
    }

    fn fmt_if(&mut self, ifexpr: &ast::IfExpr) {
        self.out.push_str("if ");
        self.fmt_expr(&ifexpr.cond, 0);
        self.out.push(' ');
        self.fmt_block(&ifexpr.then_branch);
        if let Some(e) = &ifexpr.else_branch {
            self.out.push(' ');
            self.out.push_str("else");
            match &**e {
                Expr::If(nested) => {
                    self.out.push(' ');
                    self.fmt_if(nested);
                }
                Expr::Block(b) => {
                    self.out.push(' ');
                    self.fmt_block(b);
                }
                _ => {
                    self.out.push(' ');
                    self.fmt_expr(e, 0);
                }
            }
        }
    }

    fn fmt_pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Ident { name, .. } => self.out.push_str(name),
            Pattern::Wildcard => self.out.push_str("_"),
            Pattern::Lit(e) => self.fmt_expr(e, P_PRIMARY),
            Pattern::Path(path) => self.out.push_str(&path.segments.join("::")),
            Pattern::Variant { name, fields } => {
                self.out.push_str(name);
                self.out.push('(');
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let mut s = Formatter { out: String::new(), depth: 0 };
                        s.fmt_pattern(f);
                        s.out
                    })
                    .collect();
                self.out.push_str(&parts.join(", "));
                self.out.push(')');
            }
        }
    }

    // ---- items ----

    fn fmt_item(&mut self, item: &Item) {
        match item {
            Item::Import(i) => self.out.push_str(&format!("import {}", i.segments.join("::"))),
            Item::Struct(s) => {
                self.out.push_str("struct ");
                self.out.push_str(&s.name);
                if !s.generics.is_empty() {
                    self.out.push('<');
                    self.out.push_str(&s.generics.join(", "));
                    self.out.push('>');
                }
                if s.fields.is_empty() {
                    self.out.push_str(" { }");
                } else {
                    self.out.push(' ');
                    self.fmt_struct_fields(&s.fields);
                }
            }
            Item::Enum(e) => {
                self.out.push_str("enum ");
                self.out.push_str(&e.name);
                if !e.generics.is_empty() {
                    self.out.push('<');
                    self.out.push_str(&e.generics.join(", "));
                    self.out.push('>');
                }
                if e.variants.is_empty() {
                    self.out.push_str(" { }");
                } else {
                    self.out.push_str(" {");
                    self.push();
                    for v in e.variants.iter() {
                        self.line();
                        self.out.push_str(&v.name);
                        if !v.payload.is_empty() {
                            self.out.push('(');
                            let tys: Vec<String> = v.payload.iter().map(|t| self.type_str(t)).collect();
                            self.out.push_str(&tys.join(", "));
                            self.out.push(')');
                        }
                    }
                    self.pop();
                    self.line();
                    self.out.push_str("}");
                }
            }
            Item::TypeAlias(t) => {
                self.out.push_str("type ");
                self.out.push_str(&t.name);
                self.out.push_str(" = ");
                self.fmt_type(&t.ty);
            }
            Item::Fn(f) => {
                if f.is_pub {
                    self.out.push_str("pub ");
                }
                if f.is_unsafe {
                    self.out.push_str("unsafe ");
                }
                if f.is_extern {
                    self.out.push_str("extern ");
                }
                self.out.push_str("fn ");
                self.out.push_str(&f.sig.name);
                if !f.sig.generics.is_empty() {
                    self.out.push('<');
                    self.out.push_str(&f.sig.generics.join(", "));
                    self.out.push('>');
                }
                self.out.push('(');
                let params: Vec<String> = f
                    .sig
                    .params
                    .iter()
                    .map(|p| {
                        let mut s = String::new();
                        if p.is_move {
                            s.push_str("move ");
                        }
                        s.push_str(&p.name);
                        if let Some(t) = &p.ty {
                            s.push_str(": ");
                            s.push_str(&self.type_str(t));
                        }
                        s
                    })
                    .collect();
                self.out.push_str(&params.join(", "));
                self.out.push(')');
                if let Some(r) = &f.sig.ret {
                    self.out.push_str(" -> ");
                    self.fmt_type(r);
                }
                self.out.push(' ');
                self.fmt_block(&f.body);
            }
            Item::Ui(u) => {
                self.out.push_str("ui ");
                self.out.push_str(&u.name);
                self.out.push(' ');
                self.out.push_str("{");
                let has_body = !u.props.is_empty() || !u.states.is_empty() || u.render.is_some();
                if !has_body {
                    self.out.push_str(" }");
                    return;
                }
                self.push();
                for p in &u.props {
                    self.line();
                    self.out.push_str(&p.name);
                    self.out.push_str(" = ");
                    self.fmt_expr(&p.value, 0);
                }
                for s in &u.states {
                    self.line();
                    self.fmt_stmt(&Stmt::Let(ast::LetStmt {
                        pat: Pattern::Ident { name: s.pat.ident_name().unwrap_or_default(), span: s.span },
                        ty: s.ty.clone(),
                        init: s.init.clone(),
                        is_mut: s.is_mut,
                        is_state: true,
                        span: s.span,
                    }));
                }
                if let Some(r) = &u.render {
                    self.line();
                    self.out.push_str("render ");
                    self.fmt_block(r);
                }
                self.pop();
                self.line();
                self.out.push_str("}");
            }
            Item::GlobalState(g) => {
                self.out.push_str("@global");
                self.line();
                self.out.push_str("var ");
                self.out.push_str(&g.name);
                if let Some(t) = &g.ty {
                    self.out.push_str(": ");
                    self.fmt_type(t);
                }
                if let Some(init) = &g.init {
                    self.out.push_str(" = ");
                    self.fmt_expr(init, 0);
                }
            }
            Item::Mod(m) => {
                if m.is_file {
                    self.out.push_str("mod ");
                    self.out.push_str(&m.name);
                    self.out.push(';');
                    return;
                }
                self.out.push_str("mod ");
                self.out.push_str(&m.name);
                self.out.push(' ');
                self.out.push_str("{");
                if m.items.is_empty() {
                    self.out.push_str(" }");
                    return;
                }
                self.push();
                for it in &m.items {
                    self.line();
                    self.fmt_item(it);
                }
                self.pop();
                self.line();
                self.out.push_str("}");
            }
            Item::Trait(t) => {
                self.out.push_str("trait ");
                self.out.push_str(&t.name);
                self.out.push(' ');
                self.out.push_str("{ }");
            }
            Item::Impl(_) | Item::ExternBlock(_) => {
                self.out.push_str("// (unsupported item omitted)");
            }
        }
    }

    fn fmt_struct_fields(&mut self, fields: &[ast::StructField]) {
        self.out.push_str("{");
        self.push();
        for f in fields {
            self.line();
            self.out.push_str(&f.name);
            self.out.push_str(": ");
            self.fmt_type(&f.ty);
        }
        self.pop();
        self.line();
        self.out.push_str("}");
    }

    fn fmt_type(&mut self, t: &Type) {
        self.out.push_str(&self.type_str(t));
    }

    fn type_str(&self, t: &Type) -> String {
        match t {
            Type::Fn { params, ret } => {
                let ps: Vec<String> = params.iter().map(|p| self.type_str(p)).collect();
                let rs = match ret {
                    Some(r) => self.type_str(r),
                    None => "()".to_string(),
                };
                format!("fn({}) -> {}", ps.join(", "), rs)
            }
            Type::Path(p) => {
                let mut s = p.segments.join("::");
                for gs in &p.generics {
                    let args: Vec<String> = gs.iter().map(|g| self.type_str(g)).collect();
                    s.push('<');
                    s.push_str(&args.join(", "));
                    s.push('>');
                }
                s
            }
            Type::Ref { ty, mutable } => {
                if *mutable {
                    format!("&mut {}", self.type_str(ty))
                } else {
                    format!("&{}", self.type_str(ty))
                }
            }
            Type::Tuple(tys) => {
                if tys.is_empty() {
                    "()".to_string()
                } else {
                    let inner: Vec<String> = tys.iter().map(|t| self.type_str(t)).collect();
                    format!("({})", inner.join(", "))
                }
            }
            Type::Infer => "_".to_string(),
        }
    }

    // ---- expressions ----

    fn fmt_expr(&mut self, e: &Expr, parent_prec: u8) {
        match e {
            Expr::Int(v) => self.out.push_str(&v.to_string()),
            Expr::Float(v) => {
                let s = format!("{}", v);
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    self.out.push_str(&format!("{}.0", s));
                } else {
                    self.out.push_str(&s);
                }
            }
            Expr::Str(s) => self.out.push_str(&format!("{}", escape_str(s, false))),
            Expr::FmtStr(s) => self.out.push_str(&format!("{}", escape_str(s, true))),
            Expr::Bool(b) => self.out.push_str(if *b { "true" } else { "false" }),
            Expr::Ident { name, .. } => self.out.push_str(name),
            Expr::Path(p) => self.out.push_str(&p.segments.join("::")),
            Expr::Array(items) => {
                self.out.push('[');
                let parts: Vec<String> = items.iter().map(|x| self.expr_str(x, 0)).collect();
                self.out.push_str(&parts.join(", "));
                self.out.push(']');
            }
            Expr::Tuple(items) => {
                if items.is_empty() {
                    self.out.push_str("()");
                    return;
                }
                self.out.push('(');
                let parts: Vec<String> = items.iter().map(|x| self.expr_str(x, 0)).collect();
                self.out.push_str(&parts.join(", "));
                self.out.push(')');
            }
            Expr::StructLit { name, fields } => {
                self.out.push_str(&name.segments.join("::"));
                self.out.push(' ');
                self.out.push_str("{");
                if fields.is_empty() {
                    self.out.push_str(" }");
                    return;
                }
                self.push();
                for (fname, fval) in fields {
                    self.line();
                    self.out.push_str(fname);
                    self.out.push_str(": ");
                    self.fmt_expr(fval, 0);
                }
                self.pop();
                self.line();
                self.out.push_str("}");
            }
            Expr::Unary { op, expr } => {
                let needs_paren = parent_prec > P_UNARY;
                if needs_paren {
                    self.out.push('(');
                }
                let op_s = match op {
                    ast::UnOp::Neg => "-",
                    ast::UnOp::Not => "!",
                    ast::UnOp::Ref => "&",
                    ast::UnOp::Deref => "*",
                };
                self.out.push_str(op_s);
                self.fmt_expr(expr, P_UNARY);
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let (prec, op_s, right_assoc) = binop_info(*op);
                let needs_paren = parent_prec > prec;
                if needs_paren {
                    self.out.push('(');
                }
                let lhs_prec = if right_assoc { prec - 1 } else { prec };
                self.fmt_expr(lhs, lhs_prec);
                self.out.push(' ');
                self.out.push_str(op_s);
                self.out.push(' ');
                let rhs_prec = if right_assoc { prec } else { prec + 1 };
                self.fmt_expr(rhs, rhs_prec);
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Assign { op, target, value } => {
                let needs_paren = parent_prec > P_ASSIGN;
                if needs_paren {
                    self.out.push('(');
                }
                self.fmt_expr(target, P_ASSIGN + 1);
                self.out.push(' ');
                self.out.push_str(assign_op_str(*op));
                self.out.push(' ');
                self.fmt_expr(value, P_ASSIGN);
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Call { callee, args, trailing, .. } => self.fmt_call(callee, args, *trailing),
            Expr::Field { base, field, .. } => {
                let needs_paren = parent_prec > P_POSTFIX;
                if needs_paren {
                    self.out.push('(');
                }
                self.fmt_expr(base, P_POSTFIX);
                self.out.push('.');
                self.out.push_str(field);
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Index { base, index } => {
                let needs_paren = parent_prec > P_POSTFIX;
                if needs_paren {
                    self.out.push('(');
                }
                self.fmt_expr(base, P_POSTFIX);
                self.out.push('[');
                self.fmt_expr(index, 0);
                self.out.push(']');
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Try(e) => {
                let needs_paren = parent_prec > P_POSTFIX;
                if needs_paren {
                    self.out.push('(');
                }
                self.fmt_expr(e, P_POSTFIX);
                self.out.push('?');
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Range { start, end } => {
                let needs_paren = parent_prec > P_RANGE;
                if needs_paren {
                    self.out.push('(');
                }
                if let Some(s) = start {
                    self.fmt_expr(s, P_RANGE + 1);
                }
                self.out.push_str("..");
                if let Some(e) = end {
                    self.fmt_expr(e, P_RANGE + 1);
                }
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::Block(b) => {
                let needs_paren = parent_prec > P_PRIMARY;
                if needs_paren {
                    self.out.push('(');
                }
                self.fmt_block(b);
                if needs_paren {
                    self.out.push(')');
                }
            }
            Expr::If(ifexpr) => {
                self.out.push_str("(if ");
                self.fmt_expr(&ifexpr.cond, 0);
                self.out.push(' ');
                self.fmt_block(&ifexpr.then_branch);
                if let Some(e) = &ifexpr.else_branch {
                    self.out.push(' ');
                    self.out.push_str("else ");
                    self.fmt_expr(e, 0);
                }
                self.out.push(')');
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.out.push_str("match ");
                self.fmt_expr(scrutinee, 0);
                self.out.push(' ');
                self.out.push_str("{");
                self.push();
                for arm in arms {
                    self.line();
                    self.fmt_pattern(&arm.pat);
                    self.out.push_str(" => ");
                    self.fmt_expr(&arm.value, 0);
                }
                self.pop();
                self.line();
                self.out.push_str("}");
            }
            Expr::Closure { params, body, .. } => {
                // G2.0 ③：x => expr / (a, b) => expr / () => expr
                if params.is_empty() {
                    self.out.push_str("() => ");
                } else if params.len() == 1 {
                    self.out.push_str(&params[0].name);
                    self.out.push_str(" => ");
                } else {
                    self.out.push('(');
                    let names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                    self.out.push_str(&names.join(", "));
                    self.out.push_str(") => ");
                }
                self.fmt_expr(body, 0);
            }
            Expr::Go { block, detached, .. } => {
                self.out.push_str("go");
                if *detached {
                    self.out.push('!');
                }
                self.out.push(' ');
                self.fmt_block(block);
            }
            Expr::UiOp { block, .. } => {
                self.out.push_str("ui ");
                self.fmt_block(block);
            }
            Expr::Paren(inner) => {
                self.out.push('(');
                self.fmt_expr(inner, 0);
                self.out.push(')');
            }
        }
    }

    fn fmt_call(&mut self, callee: &Expr, args: &[ast::CallArg], has_trailing: bool) {
        self.fmt_expr(callee, P_POSTFIX);
        // when a trailing block/closure was attached in postfix position,
        // emit it AFTER the parens; otherwise closures stay inline
        let regular: Vec<&ast::CallArg> = if has_trailing && !args.is_empty() {
            args[..args.len() - 1].iter().collect()
        } else {
            args.iter().collect()
        };
        let trailing: Vec<&ast::CallArg> = if has_trailing && !args.is_empty() {
            vec![&args[args.len() - 1]]
        } else {
            Vec::new()
        };
        if !regular.is_empty() || trailing.is_empty() {
            self.out.push('(');
            let parts: Vec<String> = regular
                .iter()
                .map(|a| {
                    let mut s = String::new();
                    if let Some(n) = &a.name {
                        s.push_str(n);
                        s.push_str(" = ");
                    }
                    s.push_str(&self.expr_str(&a.expr, 0));
                    s
                })
                .collect();
            self.out.push_str(&parts.join(", "));
            self.out.push(')');
        }
        for a in trailing {
            if let Expr::Closure { params, body, .. } = &a.expr {
                if params.is_empty() {
                    self.out.push(' ');
                    if let Expr::Block(b) = &**body {
                        self.fmt_block(b);
                    } else {
                        self.fmt_expr(body, 0);
                    }
                } else if params.len() == 1 {
                    self.out.push(' ');
                    self.out.push_str(&params[0].name);
                    self.out.push_str(" => ");
                    self.fmt_expr(body, 0);
                } else {
                    self.out.push(' ');
                    self.out.push('(');
                    let names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                    self.out.push_str(&names.join(", "));
                    self.out.push_str(") => ");
                    self.fmt_expr(body, 0);
                }
            }
        }
    }

    /// Render an expression as a string (used for inline contexts).
    fn expr_str(&self, e: &Expr, prec: u8) -> String {
        let mut f = Formatter { out: String::new(), depth: 0 };
        // inline sub-emitter: reuse fmt_expr on a shallow clone-free path is
        // impossible without &mut; instead format into a temp Formatter.
        f.fmt_expr(e, prec);
        f.out
    }
}

fn binop_info(op: ast::BinOp) -> (u8, &'static str, bool) {
    use ast::BinOp::*;
    match op {
        Add => (P_ADD, "+", false),
        Sub => (P_ADD, "-", false),
        Mul => (P_MUL, "*", false),
        Div => (P_MUL, "/", false),
        Rem => (P_MUL, "%", false),
        Eq => (P_EQ, "==", false),
        Ne => (P_EQ, "!=", false),
        Lt => (P_CMP, "<", false),
        Le => (P_CMP, "<=", false),
        Gt => (P_CMP, ">", false),
        Ge => (P_CMP, ">=", false),
        And => (P_AND, "&&", false),
        Or => (P_OR, "||", false),
        BitAnd => (P_BITAND, "&", false),
        BitOr => (P_BITOR, "|", false),
        BitXor => (P_BITXOR, "^", false),
        Shl => (P_SHIFT, "<<", false),
        Shr => (P_SHIFT, ">>", false),
        Range => (P_RANGE, "..", false),
    }
}

fn assign_op_str(op: ast::AssignOp) -> &'static str {
    use ast::AssignOp::*;
    match op {
        Assign => "=",
        AddAssign => "+=",
        SubAssign => "-=",
        MulAssign => "*=",
        DivAssign => "/=",
        RemAssign => "%=",
    }
}

/// Escape a string value for re-emission as a Aine string literal.
fn escape_str(s: &str, is_fmt: bool) -> String {
    let mut out = String::new();
    if is_fmt {
        out.push('f');
    }
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn fmt(src: &str) -> String {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        Formatter::new().format_program(&parsed.program.unwrap())
    }

    fn reparses_clean(src: &str) {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex after fmt: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse after fmt: {:?}", parsed.diagnostics.diagnostics);
    }

    #[test]
    fn formats_basic_fn() {
        let out = fmt("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(out, "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}");
        reparses_clean(&out);
    }

    #[test]
    fn formats_let_and_indent() {
        let out = fmt("fn f() { let x = 1\nif x > 0 {\nprint(x)\n} }");
        assert!(out.contains("    let x = 1"));
        assert!(out.contains("    if x > 0 {\n        print(x)\n    }"));
        reparses_clean(&out);
    }

    #[test]
    fn preserves_precedence() {
        // a + b * c must stay unparenthesized; (a + b) * c keeps parens
        let out1 = fmt("fn f() { let x = a + b * c }");
        assert!(out1.contains("a + b * c"), "got: {out1}");
        let out2 = fmt("fn f() { let x = (a + b) * c }");
        assert!(out2.contains("(a + b) * c"), "got: {out2}");
        reparses_clean(&out1);
        reparses_clean(&out2);
    }

    #[test]
    fn trailing_block_and_inline_closure() {
        // Button("x") { } stays trailing; map(|r| r.x) stays inline
        let out = fmt("fn f() { Button(\"x\") { click() }\nlet s = records.iter().map(r => r.amount).sum() }");
        assert!(out.contains("Button(\"x\") {"), "got: {out}");
        assert!(out.contains(".map(r => r.amount).sum()"), "got: {out}");
        reparses_clean(&out);
    }

    #[test]
    fn formats_ui_def() {
        let src = "ui Counter { title = \"x\" @state var count = 0 render { Column { Text(f\"{count}\") } } }";
        let out = fmt(src);
        assert!(out.contains("ui Counter {"));
        assert!(out.contains("    title = \"x\""));
        assert!(out.contains("    @state var count = 0"));
        assert!(out.contains("    render {"));
        reparses_clean(&out);
    }

    #[test]
    fn formats_struct_and_enum() {
        let out = fmt("struct User { name: String age: i32 }\nenum R<T, E> { Ok(T) Err(E) }");
        assert!(out.contains("struct User {\n    name: String\n    age: i32\n}"));
        assert!(out.contains("enum R<T, E> {\n    Ok(T)\n    Err(E)\n}"));
        reparses_clean(&out);
    }

    #[test]
    fn formats_go_and_ui_ops() {
        let out = fmt("fn f() { go { work() }\ngo! { fire() }\nui { records = new_records } }");
        assert!(out.contains("go {\n        work()\n    }"));
        assert!(out.contains("go! {"));
        assert!(out.contains("ui {"));
        reparses_clean(&out);
    }

    #[test]
    fn string_escaping_roundtrip() {
        let out = fmt("fn f() { let s = \"a\\\"b\\\\c\" }");
        assert!(out.contains("\\\"b\\\\c"), "got: {out}");
        reparses_clean(&out);
    }

    #[test]
    fn float_keeps_decimal_point() {
        let out = fmt("fn f() { let x = 1.0\nlet y = 2.5 }");
        assert!(out.contains("1.0"), "got: {out}");
        assert!(out.contains("2.5"));
        reparses_clean(&out);
    }

    #[test]
    fn idempotent() {
        for src in [
            "fn f() { let x = 1\nif x > 0 { print(x) }\nelse { print(0) } }",
            "ui C { @state let mut v: Vec<i32> = [] render { List(v) |r| { Text(f\"{r}\") } } }",
            "fn f() { let r = Record { id: 1 desc: \"d\" } }",
        ] {
            let once = fmt(src);
            let twice = fmt(&once);
            assert_eq!(once, twice, "not idempotent for: {src}");
            reparses_clean(&once);
        }
    }

    #[test]
    fn examples_roundtrip() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let out = fmt(&src);
            reparses_clean(&out);
            // idempotence
            let out2 = fmt(&out);
            assert_eq!(out, out2, "{name} not idempotent");
        }
    }
}
