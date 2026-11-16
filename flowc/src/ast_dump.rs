//! 规范化 AST 转储(P0-3 双解析器解析树 diff 的宿主侧)。
//!
//! 形态 = 带缩进的 S 表达式,与 examples/astdump.aine(自举侧转储)约定逐 token 一致:
//! - 宿主 Bool(KwTrue/KwFalse)→ (id true/false)(自举把 true/false 解析为 EIdent);
//! - Paren 透明展开;Path → (id a::b);
//! - Block.tail 追加为尾 (expr e)(自举无独立 tail 槽,尾表达式留在 stmts 内);
//! - 类型一律输出字符串形态且去空白(自举 parse_ty 是逐 token 拼接,无空格);
//! - Defer/TryCatch/Break/Continue 仅宿主可产出(自举 AST 缺节点,W6),语料规避。

use crate::ast::*;

pub fn dump_program(p: &Program) -> String {
    let mut out = String::from("(program\n");
    for item in &p.items {
        out.push_str(&dump_item(item, 1));
    }
    out.push_str(")\n");
    out
}

fn ind(n: usize) -> String {
    "  ".repeat(n)
}

fn ty_str(t: &Type) -> String {
    let raw = match t {
        Type::Path(p) => {
            let mut s = p.segments.join("::");
            for g in &p.generics {
                s.push('<');
                let parts: Vec<String> = g.iter().map(ty_str).collect();
                s.push_str(&parts.join(","));
                s.push('>');
            }
            s
        }
        Type::Ref { ty, mutable } => {
            if *mutable {
                format!("&mut {}", ty_str(ty))
            } else {
                format!("&{}", ty_str(ty))
            }
        }
        Type::Tuple(ts) => {
            if ts.is_empty() {
                "()".to_string()
            } else {
                let parts: Vec<String> = ts.iter().map(ty_str).collect();
                format!("({})", parts.join(","))
            }
        }
        Type::Fn { params, ret } => {
            let parts: Vec<String> = params.iter().map(ty_str).collect();
            match ret {
                Some(r) => format!("fn({})->{}", parts.join(","), ty_str(r)),
                None => format!("fn({})", parts.join(",")),
            }
        }
        Type::Infer => "_".to_string(),
    };
    raw.replace(' ', "")
}

fn opt_ty(t: &Option<Type>) -> String {
    match t {
        Some(t) => format!(" (ty {})", ty_str(t)),
        None => String::new(),
    }
}

fn esc(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn dump_item(item: &Item, depth: usize) -> String {
    match item {
        Item::Fn(f) => {
            let mut s = format!("{}(fn {}", ind(depth), f.sig.name);
            if f.is_pub {
                s.push_str(" :pub");
            }
            if f.is_extern {
                s.push_str(" :extern");
            }
            for p in &f.sig.params {
                let mv = if p.is_move { " :move" } else { "" };
                s.push_str(&format!(" (param {}{}{})", p.name, opt_ty(&p.ty), mv));
            }
            if let Some(r) = &f.sig.ret {
                s.push_str(&format!(" (ret (ty {}))", ty_str(r)));
            }
            s.push('\n');
            s.push_str(&dump_block(&f.body, depth + 1));
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Item::Struct(sd) => {
            let mut s = format!("{}(struct {}\n", ind(depth), sd.name);
            for f in &sd.fields {
                s.push_str(&format!(
                    "{}(field {} (ty {}))\n",
                    ind(depth + 1),
                    f.name,
                    ty_str(&f.ty)
                ));
            }
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Item::Enum(ed) => {
            let mut s = format!("{}(enum {}\n", ind(depth), ed.name);
            for v in &ed.variants {
                if v.payload.is_empty() {
                    s.push_str(&format!("{}(variant {})\n", ind(depth + 1), v.name));
                } else {
                    let parts: Vec<String> = v.payload.iter().map(ty_str).collect();
                    s.push_str(&format!(
                        "{}(variant {} (payload {}))\n",
                        ind(depth + 1),
                        v.name,
                        parts.join(" ")
                    ));
                }
            }
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Item::Impl(im) => {
            let mut s = format!("{}(impl {}\n", ind(depth), ty_str(&im.self_ty));
            for it in &im.items {
                s.push_str(&dump_item(it, depth + 1));
            }
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Item::Import(imp) => format!("{}(import {})\n", ind(depth), imp.segments.join("::")),
        Item::Mod(md) => {
            let mut s = format!(
                "{}(mod {}{}\n",
                ind(depth),
                md.name,
                if md.is_file { " :file" } else { "" }
            );
            for it in &md.items {
                s.push_str(&dump_item(it, depth + 1));
            }
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Item::GlobalState(g) => {
            let mut s = format!("{}(global {}{}", ind(depth), g.name, opt_ty(&g.ty));
            if let Some(e) = &g.init {
                s.push_str(&format!(" {}", dump_expr(e)));
            }
            s.push_str(")\n");
            s
        }
        Item::Ui(_) => format!("{}(ui ...)\n", ind(depth)),
        Item::TypeAlias(t) => {
            format!("{}(typealias {} (ty {}))\n", ind(depth), t.name, ty_str(&t.ty))
        }
        Item::Trait(_) => format!("{}(trait ...)\n", ind(depth)),
        Item::ExternBlock(_) => format!("{}(extern ...)\n", ind(depth)),
    }
}

fn dump_block(b: &Block, depth: usize) -> String {
    let mut out = format!("{}(block\n", ind(depth));
    for st in &b.stmts {
        out.push_str(&dump_stmt(st, depth + 1));
    }
    if let Some(t) = &b.tail {
        out.push_str(&format!("{}(expr {})\n", ind(depth + 1), dump_expr(t)));
    }
    out.push_str(&format!("{})\n", ind(depth)));
    out
}

fn dump_stmt(st: &Stmt, depth: usize) -> String {
    match st {
        Stmt::Let(l) => {
            let name = pat_name(&l.pat).unwrap_or_else(|| "_".to_string());
            let mut s = format!(
                "{}(let {}{}",
                ind(depth),
                if l.is_mut { "mut " } else { "" },
                name
            );
            s.push_str(&opt_ty(&l.ty));
            if let Some(e) = &l.init {
                s.push_str(&format!(" {}", dump_expr(e)));
            }
            s.push_str(")\n");
            s
        }
        Stmt::Expr(e) => format!("{}(expr {})\n", ind(depth), dump_expr(e)),
        Stmt::Return(e) => match e {
            Some(e) => format!("{}(return {})\n", ind(depth), dump_expr(e)),
            None => format!("{}(return)\n", ind(depth)),
        },
        Stmt::If(i) => dump_if(i, depth),
        Stmt::While { cond, body, .. } => {
            let mut s = format!("{}(while {}\n", ind(depth), dump_expr(cond));
            s.push_str(&dump_block(body, depth + 1));
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Stmt::For { pat, iter, body, .. } => {
            let name = pat_name(pat).unwrap_or_else(|| "_".to_string());
            let mut s = format!("{}(for {} {}\n", ind(depth), name, dump_expr(iter));
            s.push_str(&dump_block(body, depth + 1));
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Stmt::Break(_) => format!("{}(break)\n", ind(depth)),
        Stmt::Continue(_) => format!("{}(continue)\n", ind(depth)),
        Stmt::Defer(e) => format!("{}(defer {})\n", ind(depth), dump_expr(e)),
        Stmt::TryCatch(t) => {
            let mut s = format!("{}(try-catch\n", ind(depth));
            s.push_str(&dump_block(&t.try_block, depth + 1));
            if let Some(p) = &t.catch_param {
                s.push_str(&format!(
                    "{}(catch {}{})\n",
                    ind(depth + 1),
                    p.name,
                    opt_ty(&p.ty)
                ));
            }
            s.push_str(&dump_block(&t.catch_block, depth + 1));
            s.push_str(&format!("{})\n", ind(depth)));
            s
        }
        Stmt::Block(b) => dump_block(b, depth),
    }
}

fn dump_if(i: &IfExpr, depth: usize) -> String {
    let mut s = format!("{}(if {}\n", ind(depth), dump_expr(&i.cond));
    s.push_str(&dump_block(&i.then_branch, depth + 1));
    if let Some(e) = &i.else_branch {
        match &**e {
            Expr::Block(b) => {
                s.push_str(&dump_block(b, depth + 1));
            }
            Expr::If(inner) => {
                // else if 链:宿主嵌套 If 表达式;自举是 else_s 内的 SIf
                s.push_str(&dump_if(inner, depth + 1));
            }
            other => {
                s.push_str(&format!("{}(else\n", ind(depth + 1)));
                s.push_str(&format!("{}(expr {})\n", ind(depth + 2), dump_expr(other)));
                s.push_str(&format!("{})\n", ind(depth + 1)));
            }
        }
    }
    s.push_str(&format!("{})\n", ind(depth)));
    s
}

fn pat_name(p: &Pattern) -> Option<String> {
    match p {
        Pattern::Ident { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn dump_pattern(p: &Pattern) -> String {
    match p {
        Pattern::Ident { name, .. } => {
            if name == "_" {
                "(p-wild)".to_string()
            } else {
                format!("(p-id {})", name)
            }
        }
        Pattern::Wildcard => "(p-wild)".to_string(),
        Pattern::Lit(e) => format!("(p-lit {})", dump_expr(e)),
        Pattern::Path(p) => format!("(p-id {})", p.segments.join("::")),
        Pattern::Variant { name, fields } => {
            if fields.is_empty() {
                format!("(p-variant {})", name)
            } else {
                let parts: Vec<String> = fields.iter().map(dump_pattern).collect();
                format!("(p-variant {} {})", name, parts.join(" "))
            }
        }
    }
}

fn stmt_inline(st: &Stmt) -> String {
    dump_stmt(st, 0).trim_end().to_string()
}

pub fn dump_expr(e: &Expr) -> String {
    match e {
        Expr::Int(n) => format!("(int {})", n),
        Expr::Float(f) => format!("(float {})", f),
        Expr::Str(s) => format!("(str \"{}\")", esc(s)),
        Expr::FmtStr(s) => format!("(fstr f\"{}\")", esc(s)),
        Expr::Bool(b) => format!("(id {})", if *b { "true" } else { "false" }),
        Expr::Ident { name, .. } => format!("(id {})", name),
        Expr::Path(p) => format!("(id {})", p.segments.join("::")),
        Expr::Array(items) => {
            if items.is_empty() {
                return "(vec)".to_string();
            }
            let parts: Vec<String> = items.iter().map(dump_expr).collect();
            format!("(vec {})", parts.join(" "))
        }
        Expr::Tuple(items) => {
            let parts: Vec<String> = items.iter().map(dump_expr).collect();
            format!("(tuple {})", parts.join(" "))
        }
        Expr::StructLit { name, fields } => {
            let mut s = format!("(struct-lit {}", name.segments.join("::"));
            for (fname, fexpr) in fields {
                s.push_str(&format!(" (field {} {})", fname, dump_expr(fexpr)));
            }
            s.push(')');
            s
        }
        Expr::Unary { op, expr } => format!("(unary {} {})", unop_str(op), dump_expr(expr)),
        Expr::Binary { op, lhs, rhs } => {
            format!("(bin {} {} {})", binop_str(op), dump_expr(lhs), dump_expr(rhs))
        }
        Expr::Assign { op, target, value } => {
            format!(
                "(assign {} {} {})",
                dump_expr(target),
                assignop_str(op),
                dump_expr(value)
            )
        }
        Expr::Call { callee, args, .. } => {
            let mut s = format!("(call {}", dump_expr(callee));
            for a in args {
                s.push(' ');
                s.push_str(&dump_expr(&a.expr));
            }
            s.push(')');
            s
        }
        Expr::Field { base, field, .. } => format!("(field {} {})", dump_expr(base), field),
        Expr::Index { base, index } => {
            format!("(index {} {})", dump_expr(base), dump_expr(index))
        }
        Expr::Try(x) => format!("(try {})", dump_expr(x)),
        Expr::Range { start, end } => {
            let s0 = match start {
                Some(x) => dump_expr(x),
                None => String::new(),
            };
            let s1 = match end {
                Some(x) => dump_expr(x),
                None => String::new(),
            };
            format!("(range {} {})", s0, s1)
        }
        Expr::Block(b) => {
            let mut s = String::from("(block");
            for st in &b.stmts {
                s.push(' ');
                s.push_str(&stmt_inline(st));
            }
            if let Some(t) = &b.tail {
                s.push_str(&format!(" (expr {})", dump_expr(t)));
            }
            s.push(')');
            s
        }
        Expr::If(i) => dump_if_inline(i),
        Expr::Match { scrutinee, arms, .. } => {
            let mut s = format!("(match {}", dump_expr(scrutinee));
            for arm in arms {
                s.push_str(&format!(
                    " (arm {} {})",
                    dump_pattern(&arm.pat),
                    dump_expr(&arm.value)
                ));
            }
            s.push(')');
            s
        }
        Expr::Closure { params, body, .. } => {
            let mut s = String::from("(closure");
            for p in params {
                s.push_str(&format!(" (param {}{})", p.name, opt_ty(&p.ty)));
            }
            s.push(' ');
            s.push_str(&dump_expr(body));
            s.push(')');
            s
        }
        Expr::Go { block, .. } => {
            let mut s = String::from("(go");
            for st in &block.stmts {
                s.push(' ');
                s.push_str(&stmt_inline(st));
            }
            s.push(')');
            s
        }
        Expr::UiOp { block, .. } => {
            let mut s = String::from("(ui-op");
            for st in &block.stmts {
                s.push(' ');
                s.push_str(&stmt_inline(st));
            }
            s.push(')');
            s
        }
        Expr::Paren(x) => dump_expr(x),
    }
}

fn dump_if_inline(i: &IfExpr) -> String {
    let mut s = format!("(if {} ", dump_expr(&i.cond));
    s.push_str("(then");
    for st in &i.then_branch.stmts {
        s.push(' ');
        s.push_str(&stmt_inline(st));
    }
    if let Some(t) = &i.then_branch.tail {
        s.push_str(&format!(" (expr {})", dump_expr(t)));
    }
    s.push(')');
    if let Some(e) = &i.else_branch {
        s.push_str(" (else");
        match &**e {
            Expr::Block(b) => {
                for st in &b.stmts {
                    s.push(' ');
                    s.push_str(&stmt_inline(st));
                }
                if let Some(t) = &b.tail {
                    s.push_str(&format!(" (expr {})", dump_expr(t)));
                }
            }
            Expr::If(inner) => {
                // else if 链:宿主嵌套 If 表达式;自举是 else_s 内的 SIf
                s.push(' ');
                s.push_str(&dump_if_inline(inner));
            }
            other => {
                s.push_str(&format!(" (expr {})", dump_expr(other)));
            }
        }
        s.push(')');
    }
    s
}

fn unop_str(op: &UnOp) -> String {
    let s = match op {
        UnOp::Neg => "-",
        UnOp::Not => "!",
        UnOp::Ref => "&",
        UnOp::Deref => "*",
    };
    s.to_string()
}

fn binop_str(op: &BinOp) -> String {
    let s = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::Range => "..",
    };
    s.to_string()
}

fn assignop_str(op: &AssignOp) -> String {
    let s = match op {
        AssignOp::Assign => "=",
        AssignOp::AddAssign => "+=",
        AssignOp::SubAssign => "-=",
        AssignOp::MulAssign => "*=",
        AssignOp::DivAssign => "/=",
        AssignOp::RemAssign => "%=",
    };
    s.to_string()
}
