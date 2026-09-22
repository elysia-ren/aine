//! HIR (high-level intermediate representation) — the resolved item layer.
//!
//! Design doc §56 pipeline: AST + Source Span → HIR → Name Resolution → ...
//! M1.3 delivers:
//! - a symbol table of every definition (items, params, locals, states, ...);
//! - a resolved item layer (HirProgram) where item names are bound to symbols;
//! - a Resolution map binding every identifier occurrence (by Span) to its
//!   defining symbol — the input for M2 (VIR / valueal analysis).
//!
//! Function bodies stay as ast::Block at this stage; expression-level
//! lowering into HirExpr happens in M2 when VIR is introduced.

use std::collections::HashMap;

use crate::ast;
use crate::token::Span;

/// Unique id of a symbol in the program's symbol table.
pub type SymbolId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// top-level items
    Fn,
    Struct,
    Enum,
    /// a variant of a user enum (Number in enum Expr)
    EnumVariant,
    TypeAlias,
    Import,
    Global,
    Ui,
    Mod,
    /// local bindings
    Param,
    Local,
    State,
    Prop,
    ClosureParam,
    ForPattern,
    /// predeclared runtime / stdlib surface
    Builtin,
    BuiltinType,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub is_mut: bool,
}

/// Resolved program: items bound to symbols + the full symbol table.
#[derive(Debug)]
pub struct HirProgram {
    pub symbols: Vec<Symbol>,
    pub items: Vec<HirItem>,
}

#[derive(Debug)]
pub enum HirItem {
    Import {
        path: String,
        span: Span,
        sym: SymbolId,
    },
    Struct {
        name: String,
        /// (field name, span, declared type)
        fields: Vec<(String, Span, Option<ast::Type>)>,
        span: Span,
        sym: SymbolId,
        /// @-attributes（如 @not_send）
        attributes: Vec<String>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
        span: Span,
        sym: SymbolId,
    },
    TypeAlias {
        name: String,
        span: Span,
        sym: SymbolId,
    },
    Fn(FnInfo),
    Ui(UiInfo),
    Global {
        name: String,
        ty: Option<ast::Type>,
        init: Option<ast::Expr>,
        span: Span,
        sym: SymbolId,
    },
    Mod {
        name: String,
        items: Vec<HirItem>,
        span: Span,
    },
}

#[derive(Debug)]
pub struct FnInfo {
    pub name: String,
    pub params: Vec<ParamInfo>,
    pub body: ast::Block,
    pub ret: Option<ast::Type>,
    pub span: Span,
    pub sym: SymbolId,
    pub is_pub: bool,
    pub is_unsafe: bool,
    pub is_extern: bool,
    pub is_catchable: bool,
}

#[derive(Debug)]
pub struct ParamInfo {
    pub name: String,
    pub sym: SymbolId,
    pub span: Span,
    pub ty: Option<ast::Type>,
    /// declared with the move keyword (design §22.2)
    pub is_move: bool,
    /// declared with the mut modifier
    pub is_mut: bool,
}

#[derive(Debug)]
pub struct UiState {
    pub name: String,
    pub sym: SymbolId,
    pub span: Span,
    pub ty: Option<ast::Type>,
    pub init: Option<ast::Expr>,
}

#[derive(Debug)]
pub struct UiInfo {
    pub name: String,
    pub props: Vec<(String, Span)>,
    pub states: Vec<UiState>,
    pub render: Option<ast::Block>,
    pub span: Span,
    pub sym: SymbolId,
}

/// Binds every identifier occurrence in value position to its defining symbol.
/// Keyed by the identifier's source Span.
#[derive(Debug, Default)]
pub struct Resolution {
    pub idents: HashMap<Span, SymbolId>,
    /// spans of expressions that are references to type names (struct/enum/ui)
    pub type_refs: Vec<Span>,
    /// binding definitions: keyed by the definition's span (let stmt, for
    /// pattern, closure param, state decl, ...) -> symbol id
    pub bindings: HashMap<Span, SymbolId>,
}

impl Resolution {
    pub fn get(&self, span: &Span) -> Option<SymbolId> {
        self.idents.get(span).copied()
    }
}

/// Symbols predeclared by the runtime / stdlib so examples resolve cleanly.
/// Full stdlib registration arrives with the standard library implementation.
pub const BUILTIN_GLOBALS: &[&str] = &[
    // I/O and runtime
    "print", "now", "run_ui", "db", "http", "read_file", "write_file", "append_file", "file_exists",
    "read_bytes", "write_bytes", "base64_encode", "base64_decode",
    // Std API (docs/STD_API_SPEC.md): 服务化档
    "read_line", "env", "sleep", "json_encode", "json_decode", "http_request", "file_list",
    "edit_new", "listbox_ondblclick", "listbox_get",
    // UI 档(docs/STD_API_SPEC.md): 仅原生路径
    "window_new", "label_new", "button_new", "set_text", "get_text", "button_onclick", "ui_run",
    "input_new", "listbox_new", "listbox_add", "listbox_clear", "listbox_selected",
    // rt_* 自绘运行时 (D2D/DWrite, alee C 模板)
    "rt_window", "rt_comp", "rt_run", "rt_set_comp_text", "rt_begin", "rt_fill", "rt_rrect",
    "rt_srect", "rt_hline", "rt_text", "rt_click", "rt_wait_click", "rt_invalidate", "rt_on_style", "sci_get_line",
    "rt_w", "rt_h", "rt_wait_event",
    // Scintilla 编辑控件 (bin/Scintilla.dll + Lexilla.dll)
    "sci_init", "sci_new", "sci_msg", "sci_msgp", "sci_set_text", "sci_get_text", "sci_fit", "sys_exec", "sys_open",
    // ui{} DSL 组件名(声明式块内出现; S1a 映射到上述原语)
    "Window", "Text", "Button", "Input", "List", "Row", "Column", "Panel",
    "input_new", "listbox_new", "listbox_add", "listbox_clear", "listbox_selected",
    // 断言内建（测试与契约检查）
    "assert",
    // 命令行实参（aine run/build 注入，宿主侧全局）
    "cli_args",
    // stdlib
    "File", "Path",
    // UI component library
    "Column", "Row", "Text", "Button", "TextInput", "List", "ListItem",
    "Window", "Dialog", "Image",
    // UI value constants (e.g. TextInput type = number)
    "number", "text",
];

/// Builtin primitive / stdlib type names (resolved without definition).
pub const BUILTIN_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize",
    "f32", "f64", "bool", "char", "String", "Vec", "Option", "Result",
    "Map", "CStr", "CString", "CArray",
];

/// Enum variant names provided by builtin generics (Option / Result).
pub const BUILTIN_VARIANTS: &[&str] = &["Some", "None", "Ok", "Err"];
