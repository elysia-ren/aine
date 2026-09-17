//! AST (abstract syntax tree) for the Aine language.
//!
//! Every node carries a byte Span so errors and IDE features can map back
//! to canonical source (design doc §53 Source Map).
//!
//! Grammar notes (design doc §55):
//! - statements are terminated by newline (soft); leading postfix operators
//!   (. ( [ ?) continue the previous expression across lines;
//! - blocks yield a value: the last statement, if an expression, is the tail.

use crate::token::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Import(ImportItem),
    Struct(StructDef),
    Enum(EnumDef),
    TypeAlias(TypeAlias),
    Trait(TraitDef),
    Impl(ImplBlock),
    ExternBlock(ExternBlock),
    Fn(FnDef),
    Ui(UiDef),
    GlobalState(GlobalStateDef),
    Mod(ModDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportItem {
    /// path segments, e.g. ["sqlite"] or ["std", "fs"]
    pub segments: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<StructField>,
    pub span: Span,
    /// @-attributes（如 @not_send）
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub generics: Vec<String>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    /// tuple payload types, e.g. Ok(T) -> [T]
    pub payload: Vec<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub name: String,
    pub generics: Vec<String>,
    pub methods: Vec<FnSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplBlock {
    pub trait_path: Option<Path>,
    pub self_ty: Type,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    pub abi: Option<String>,
    pub fns: Vec<FnSig>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    pub name: String,
    pub generics: Vec<String>,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDef {
    pub sig: FnSig,
    pub body: Block,
    pub is_pub: bool,
    pub is_unsafe: bool,
    pub is_extern: bool,
    pub is_catchable: bool,
    pub attributes: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_move: bool,
    /// declared with the mut modifier (mutable parameter)
    pub is_mut: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiDef {
    pub name: String,
    /// property assignments: title = "...", size = (400, 300)
    pub props: Vec<UiProp>,
    /// @state let declarations
    pub states: Vec<LetStmt>,
    pub render: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiProp {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalStateDef {
    pub name: String,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModDef {
    pub name: String,
    pub items: Vec<Item>,
    /// 文件模块形态 `mod name;`（加载器据此读入 name.aine 内联）
    pub is_file: bool,
    pub span: Span,
}

// ---- types ----

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// i32, f64, String, Vec<Record>, Result<(), AppError>, CStr, ...
    Path(Path),
    /// &T, &mut T
    Ref { ty: Box<Type>, mutable: bool },
    /// () unit or (A, B) tuple
    Tuple(Vec<Type>),
    /// fn(A, B) -> R —— 统一函数类型（捕获/非捕获同型，D-xx 捕获即快照）
    Fn { params: Vec<Type>, ret: Option<Box<Type>> },
    /// _ (inferred)
    Infer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<String>,
    pub generics: Vec<Vec<Type>>,
    pub span: Span,
}

impl Path {
    pub fn simple(name: &str) -> Path {
        Path {
            segments: vec![name.to_string()],
            generics: Vec::new(),
            span: Span::new(0, 0),
        }
    }
    pub fn display(&self) -> String {
        self.segments.join("::")
    }
}

// ---- statements ----

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// value of the block: the last statement if it was an expression
    pub tail: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(LetStmt),
    Expr(Box<Expr>),
    Return(Option<Box<Expr>>),
    If(Box<IfExpr>),
    While { cond: Box<Expr>, body: Block, span: Span },
    For { pat: Pattern, iter: Box<Expr>, body: Block, span: Span },
    Break(Span),
    Continue(Span),
    Defer(Box<Expr>),
    TryCatch(TryCatchStmt),
    Block(Box<Block>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub pat: Pattern,
    pub ty: Option<Type>,
    pub init: Option<Box<Expr>>,
    pub is_mut: bool,
    /// @state decorator (runtime-managed state storage, design doc §33)
    pub is_state: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TryCatchStmt {
    pub try_block: Block,
    pub catch_param: Option<Param>,
    pub catch_block: Block,
    pub span: Span,
}

// ---- patterns ----

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Ident { name: String, span: Span },
    Wildcard,
    Lit(Expr),
    Path(Path),
    /// enum variant pattern: Number(n), Binary(l, r)
    Variant { name: String, fields: Vec<Pattern> },
}

impl Pattern {
    /// Name bound by this pattern, if it binds one.
    pub fn ident_name(&self) -> Option<String> {
        match self {
            Pattern::Ident { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    /// All identifier bindings in this pattern (including nested variant
    /// fields), as (name, source span).
    pub fn bindings(&self) -> Vec<(String, Span)> {
        let mut out = Vec::new();
        self.collect_bindings(&mut out);
        out
    }

    fn collect_bindings(&self, out: &mut Vec<(String, Span)>) {
        match self {
            Pattern::Ident { name, span } => out.push((name.clone(), *span)),
            Pattern::Variant { fields, .. } => {
                for f in fields {
                    f.collect_bindings(out);
                }
            }
            _ => {}
        }
    }
}

// ---- expressions ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
    Ref,
    Deref,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    /// None for positional args, Some(name) for named args like placeholder = "..."
    pub name: Option<String>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub cond: Box<Expr>,
    pub then_branch: Block,
    pub else_branch: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pat: Pattern,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    FmtStr(String),
    Bool(bool),
    Ident { name: String, span: Span },
    Path(Path),
    Array(Vec<Expr>),
    Tuple(Vec<Expr>),
    /// Record { id: 0, desc: "..." } — named-field construction
    StructLit { name: Path, fields: Vec<(String, Expr)> },
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    Assign { op: AssignOp, target: Box<Expr>, value: Box<Expr> },
    /// callee(args) — trailing block/closure arguments are appended to args
    /// as closures (design doc §69: Button("...") { }, List(records) |r| { })
    /// and trailing marks that the last arg was attached in postfix position.
    Call { callee: Box<Expr>, args: Vec<CallArg>, span: Span, trailing: bool },
    Field { base: Box<Expr>, field: String, span: Span },
    Index { base: Box<Expr>, index: Box<Expr> },
    /// expr? — try propagation (design doc §45)
    Try(Box<Expr>),
    Range { start: Option<Box<Expr>>, end: Option<Box<Expr>> },
    Block(Box<Block>),
    If(Box<IfExpr>),
    Match { scrutinee: Box<Expr>, arms: Vec<MatchArm>, span: Span },
    Closure { params: Vec<Param>, body: Box<Expr>, span: Span },
    /// go { ... } / go! { ... } — structured / detached task (design doc §29/§30)
    Go { block: Box<Block>, detached: bool, span: Span },
    /// ui { ... } — UI Domain Operation (design doc §36)
    UiOp { block: Box<Block>, span: Span },
    Paren(Box<Expr>),
}
