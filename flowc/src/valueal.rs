//! Valueal analysis (M2) — design doc §12-21.
//!
//! Implements the Value Aine Core on top of the resolved HIR + types:
//! - Value Provenance (§16): Input(i) / Field / Element / Transform /
//!   Owned / Copy / Unknown;
//! - VIR (Value Aine IR, §13-14): per-function node list capturing
//!   Source / Copy / Move / Consume / Projection / Borrow / Materialize /
//!   Call / Return / Capture / TaskTransfer / UITransfer / StateRead /
//!   StateWrite;
//! - Function Semantic Summary (§15-18) with composition at call sites (§17);
//! - Ownership checks: use-after-move (V4001), state write outside ui{} (V4002).
//!
//! Ownership model (design §5, §10, §22, §25, §34, §39):
//! - ordinary binds/reads are COPY semantics (compiler may view-optimize
//!   internally); explicit moves happen only at task/ui boundaries and
//!   move params — this keeps the owned-value user model;
//! - go{}/go!{} capture of a non-Copy value transfers ownership (consume);
//! - ui { state = v } consumes v (StateWrite);
//! - move params consume the caller's argument.

use std::collections::{HashMap, HashSet};

use crate::ast::{self, Expr, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticSink, Severity};
use crate::hir::{FnInfo, HirItem, HirProgram, Resolution, SymbolId};
use crate::token::{Span, Token};
use crate::typeck::{Ty, TypeckResult};

/// Value provenance (design §16).
#[derive(Debug, Clone, PartialEq)]
pub enum Prov {
    /// derived from the i-th parameter
    Input(usize),
    /// field projection: base.field
    Field(Box<Prov>, String),
    /// element projection: base[index]
    Element(Box<Prov>),
    /// transformation (call result, ?, etc.)
    Transform(Box<Prov>),
    /// freshly produced value (literal, struct literal, ...)
    Owned,
    /// copied from a source value
    Copy(Box<Prov>),
    Unknown,
}

impl Prov {
    pub fn display(&self) -> String {
        match self {
            Prov::Input(i) => format!("Input({})", i),
            Prov::Field(b, f) => format!("{}.{}", b.display(), f),
            Prov::Element(b) => format!("{}[]", b.display()),
            Prov::Transform(b) => format!("transform({})", b.display()),
            Prov::Owned => "Owned".to_string(),
            Prov::Copy(b) => format!("copy({})", b.display()),
            Prov::Unknown => "Unknown".to_string(),
        }
    }

    /// Substitute a summary's return provenance with concrete argument
    /// provenances at a call site (§17 composition).
    pub fn substitute(&self, args: &[Prov]) -> Prov {
        match self {
            Prov::Input(i) => args.get(*i).cloned().unwrap_or(Prov::Unknown),
            Prov::Field(b, f) => Prov::Field(Box::new(b.substitute(args)), f.clone()),
            Prov::Element(b) => Prov::Element(Box::new(b.substitute(args))),
            Prov::Transform(b) => Prov::Transform(Box::new(b.substitute(args))),
            Prov::Copy(b) => Prov::Copy(Box::new(b.substitute(args))),
            Prov::Owned => Prov::Owned,
            Prov::Unknown => Prov::Unknown,
        }
    }

    /// Collapse to the design's normalized form (DerivedFrom chains).
    /// Exceeding the depth budget degrades the WHOLE chain to Unknown
    /// (§19/§59: 超预算保守回退，不无限分析).
    pub fn normalize(&self) -> Prov {
        if self.depth() > PROV_DEPTH_CAP {
            return Prov::Unknown;
        }
        self.normalize_depth(0)
    }

    /// Chain depth (for the budget check).
    pub fn depth(&self) -> usize {
        match self {
            Prov::Field(b, _) | Prov::Element(b) | Prov::Transform(b) | Prov::Copy(b) => 1 + b.depth(),
            _ => 0,
        }
    }

    fn normalize_depth(&self, depth: usize) -> Prov {
        let _ = depth;
        match self {
            Prov::Field(b, f) => match b.normalize_depth(0) {
                Prov::Field(bb, bf) => Prov::Field(bb, format!("{}.{}", bf, f)),
                other => Prov::Field(Box::new(other), f.clone()),
            },
            Prov::Element(b) => Prov::Element(Box::new(b.normalize_depth(0))),
            Prov::Transform(b) => Prov::Transform(Box::new(b.normalize_depth(0))),
            Prov::Copy(b) => Prov::Copy(Box::new(b.normalize_depth(0))),
            other => other.clone(),
        }
    }
}

/// Per-parameter summary of a function.
#[derive(Debug, Clone)]
pub struct ParamSummary {
    pub name: String,
    /// declared with the move keyword (design §22.2)
    pub is_move: bool,
    /// the callee consumes the parameter (moved into a task / ui write / ...)
    pub consumed: bool,
    /// the parameter's declared type (for call-site view decisions)
    pub ty: Ty,
}

/// Function semantic summary (§15).
#[derive(Debug, Clone)]
pub struct FnSummary {
    pub name: String,
    pub params: Vec<ParamSummary>,
    /// provenance of the return value (None if no return)
    pub ret: Option<Prov>,
    /// the function body spawns tasks
    pub has_tasks: bool,
    pub writes_state: bool,
    /// indices of params that escape the function (returned / stored)
    /// (design §15-18: escape = value leaves the fn's ownership)
    pub escapes: Vec<usize>,
}

impl FnSummary {
    pub fn display(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                let mut s = String::new();
                if p.is_move {
                    s.push_str("move ");
                }
                s.push_str(&p.name);
                if p.consumed {
                    s.push_str(" (consumed)");
                }
                s
            })
            .collect();
        let ret = self.ret.as_ref().map(|p| p.display()).unwrap_or_else(|| "-".into());
        let esc: Vec<String> = self.escapes.iter().map(|i| format!("{}", i)).collect();
        let esc_s = if esc.is_empty() { String::new() } else { format!(" [esc:{}]", esc.join(",")) };
        format!("{} ({}) -> {}{}", self.name, params.join(", "), ret, esc_s)
    }
}

/// VIR node (§13-14).
#[derive(Debug, Clone)]
pub enum VirNode {
    Param { index: usize, name: String },
    Source { sym: Option<SymbolId>, prov: Prov },
    Copy { from: usize },
    Move { from: usize },
    Consume { from: usize, reason: &'static str },
    Projection { from: usize, field: Option<String>, element: bool },
    Borrow { from: usize, mutable: bool },
    Materialize { from: usize },
    Call { callee: String, args: Vec<usize> },
    Return { from: usize },
    StateRead { sym: SymbolId },
    StateWrite { state: String, from: usize },
    TaskTransfer { from: usize, detached: bool },
    UiTransfer { from: usize },
    Literal,
    Unknown,
}

/// Per-function VIR.
#[derive(Debug, Clone)]
pub struct VirFn {
    pub name: String,
    pub nodes: Vec<VirNode>,
}

/// Result of the valueal pass.
pub struct ValueFlowResult {
    pub diagnostics: DiagnosticSink,
    /// functions in VIR form
    pub vfns: Vec<VirFn>,
    /// function summaries for composition
    pub summaries: HashMap<String, FnSummary>,
    /// optimization decisions (design §8-11: Borrow vs Materialize)
    pub decisions: Vec<OptDecision>,
}

/// How the compiler implements a value (design §8: Owned is the semantic;
/// Borrow/View is an optimization the compiler may choose).
#[derive(Debug, Clone, PartialEq)]
pub enum OptDecision {
    /// freshly produced value; no view possible or needed
    Owned,
    /// copy semantics with Cheap cost
    Copy,
    /// Compiler View: zero-copy borrow (the success case)
    Borrow { span: crate::token::Span },
    /// the view was not possible; value materialized (copied)
    Materialized { span: crate::token::Span, class: SizeClass },
}

impl OptDecision {
    pub fn display(&self) -> String {
        match self {
            OptDecision::Owned => "Owned".to_string(),
            OptDecision::Copy => "Copy".to_string(),
            OptDecision::Borrow { .. } => "Borrow (零拷贝视图)".to_string(),
            OptDecision::Materialized { class, .. } => format!("Materialized ({})", class.display()),
        }
    }
}

/// Is this type Copy (implicit copies are free)?
/// Unknown is treated as Copy: the safe fallback is to NOT consume a value
/// whose type we cannot prove non-Copy (design: safe, transparent fallback).
pub fn ty_is_copy(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::I32 | Ty::I64 | Ty::U8 | Ty::U16 | Ty::U32 | Ty::U64 | Ty::Usize | Ty::F32 | Ty::F64
            | Ty::Bool | Ty::Char | Ty::Unit | Ty::Ref(_, _) | Ty::Unknown
    )
}

/// Cost class of a value by type (design §9: Cheap / Material / Large).
/// Copy types are Cheap; unknown-size owned types are conservatively Large.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeClass {
    Cheap,
    Material,
    Large,
}

impl SizeClass {
    pub fn display(&self) -> &'static str {
        match self {
            SizeClass::Cheap => "Cheap",
            SizeClass::Material => "Material (64KB-1MB)",
            SizeClass::Large => "Large (>1MB)",
        }
    }
}

/// Estimate the cost class of a value by its type (design §9, DDR D2 §3).
pub fn size_class(ty: &Ty) -> SizeClass {
    match ty {
        Ty::Fn(_, _)
        | Ty::I32
        | Ty::I64
        | Ty::U8
        | Ty::U16
        | Ty::U32
        | Ty::U64
        | Ty::Usize
        | Ty::F32
        | Ty::F64
        | Ty::Bool
        | Ty::Char
        | Ty::Unit
        | Ty::Ref(_, _)
        | Ty::Unknown => SizeClass::Cheap,
        Ty::Str | Ty::Vec(_) | Ty::Map(_) | Ty::OptionT(_) | Ty::ResultT(_, _) | Ty::Tuple(_) | Ty::Named(_) => {
            SizeClass::Large
        }
        Ty::Error => SizeClass::Cheap,
    }
}

/// Is this provenance derived from a function input (view-eligible)?
/// A value with Input-derived provenance CAN be implemented as a Compiler
/// View (zero-copy) when it does not escape; Owned/Unknown values cannot.
pub fn prov_is_input_derived(p: &Prov) -> bool {
    match p {
        Prov::Input(_) => true,
        Prov::Field(b, _) => prov_is_input_derived(b),
        Prov::Element(b) => prov_is_input_derived(b),
        Prov::Transform(b) => prov_is_input_derived(b),
        Prov::Copy(b) => prov_is_input_derived(b),
        Prov::Owned | Prov::Unknown => false,
    }
}

/// Analysis budget: maximum provenance chain depth before falling back to
/// Unknown (design §19/§59: 超预算保守回退，不无限分析).
pub const PROV_DEPTH_CAP: usize = 16;

/// Error codes for valueal (V4xxx range) and optimization hints (F2xxx).
pub mod vcodes {
    pub const USE_AFTER_MOVE: &str = "V4001";
    pub const STATE_WRITE_OUTSIDE_UI: &str = "V4002";
    pub const PARAM_CONSUMED_TWICE: &str = "V4003";

    /// Optimization Explain (design §58 / DDR D2): a view-eligible value
    /// escaped and had to be materialized (copied).
    pub const MATERIALIZATION: &str = "F2001";
    /// analysis budget exceeded; summary degraded (design §59)
    pub const BUDGET_DEGRADED: &str = "F2002";
    /// Optimization Explain (design §15-18): a param escapes the callee
    /// (returned/stored), so the call-site view must materialize.
    pub const ESCAPE: &str = "F2003";
    /// UI rule (UI Guide §4.2): go{}/go!{} inside ui{} is forbidden.
    pub const UI_GO_INSIDE: &str = "V4004";
    /// UI rule (UI Guide §5.3): @global writes must happen inside ui{}.
    pub const GLOBAL_WRITE_OUTSIDE_UI: &str = "V4005";
    /// Send rule (Concurrency Guide §3): non-Send value crossing a task boundary.
    pub const NOT_SEND: &str = "V4006";
}

/// Does this provenance chain reference the i-th parameter?
/// (used for escape detection: ret prov referencing Input(i) ⇒ param escapes)
pub fn prov_refs_input(p: &Prov, i: usize) -> bool {
    match p {
        Prov::Input(k) => *k == i,
        Prov::Field(b, _) => prov_refs_input(b, i),
        Prov::Element(b) => prov_refs_input(b, i),
        Prov::Transform(b) => prov_refs_input(b, i),
        Prov::Copy(b) => prov_refs_input(b, i),
        _ => false,
    }
}

/// Is this provenance the i-th parameter itself (not a projection)?
/// Projection returns (Field/Element of Input) are borrow-returns — the
/// call-site already materializes the returned view; the param itself does
/// NOT escape. Only the bare Input (ownership transfer / storage) escapes.
pub fn prov_is_exact_input(p: &Prov, i: usize) -> bool {
    matches!(p, Prov::Input(k) if *k == i)
}

/// Send 自动推断（Concurrency Guide §3.2）：字段组合 —— 全字段 Send 则 Send；
/// 容器按类型参数；标量/String/闭包快照 Send。structs 表缺失时保守视为 Send。
pub fn ty_is_send(
    ty: &Ty,
    structs: &HashMap<String, Vec<(String, Ty)>>,
    not_send: &HashSet<String>,
) -> bool {
    match ty {
        Ty::Vec(x) => ty_is_send(x, structs, not_send),
        Ty::Map(x) => ty_is_send(x, structs, not_send),
        Ty::OptionT(x) => ty_is_send(x, structs, not_send),
        Ty::ResultT(a, b) => ty_is_send(a, structs, not_send) && ty_is_send(b, structs, not_send),
        Ty::Tuple(items) => items.iter().all(|t| ty_is_send(t, structs, not_send)),
        Ty::Named(n) => {
            if not_send.contains(n) {
                return false;
            }
            structs
                .get(n)
                .map(|fields| fields.iter().all(|(_, t)| ty_is_send(t, structs, not_send)))
                .unwrap_or(true)
        }
        _ => true,
    }
}

/// A valuealing through the analysis.
#[derive(Debug, Clone)]
struct FlowVal {
    prov: Prov,
    node: usize,
    consumed: bool,
    ty: Ty,
    /// scope depth at definition (for capture detection)
    depth: usize,
}

/// The analyzer.
pub struct ValueFlow<'a> {
    hir: &'a HirProgram,
    resolution: &'a Resolution,
    types: &'a TypeckResult,
    /// struct field table (Send 推断)
    typeck_structs: HashMap<String, Vec<(String, Ty)>>,
    /// @not_send 类型集合
    typeck_not_send: HashSet<String>,
    /// fn name -> summary (filled before body analysis)
    summaries: HashMap<String, FnSummary>,
    /// env: SymbolId -> valueal value (scope stack)
    env: Vec<HashMap<SymbolId, FlowVal>>,
    scope_depth: usize,
    /// (task-start scope depth, captured set) — reset per go block
    task_captures: Vec<(usize, HashSet<SymbolId>)>,
    /// ui-op nesting depth
    ui_depth: usize,
    /// state symbols defined in enclosing ui defs
    state_symbols: HashSet<SymbolId>,
    /// @global symbols (UI Guide §5: writes only inside ui{})
    global_symbols: HashSet<SymbolId>,
    /// current function being analyzed
    cur_fn: String,
    /// return provenances collected for the summary
    ret_provs: Vec<Prov>,
    /// current VIR builder
    vir: Vec<VirNode>,
    /// current function spawns tasks?
    has_tasks: bool,
    /// current function writes state?
    writes_state: bool,
    /// params of current fn
    cur_params: Vec<ParamSummary>,
    /// symbol -> param index (for consumed-param summary)
    param_syms: HashMap<SymbolId, usize>,
    /// param indexes consumed during analysis
    consumed_params: HashSet<usize>,
    /// optimization decisions (Borrow / Materialize / Owned / Copy)
    decisions: Vec<OptDecision>,
    diagnostics: DiagnosticSink,
    locs: Vec<(usize, u32, u32)>,
}

impl<'a> ValueFlow<'a> {
    pub fn new(
        hir: &'a HirProgram,
        resolution: &'a Resolution,
        types: &'a TypeckResult,
        tokens: &[Token],
    ) -> Self {
        let locs: Vec<(usize, u32, u32)> = tokens
            .iter()
            .map(|t| (t.span.start, t.loc.line, t.loc.col))
            .collect();
        ValueFlow {
            hir,
            resolution,
            types,
            typeck_structs: types.structs.clone(),
            typeck_not_send: types.not_send.clone(),
            summaries: HashMap::new(),
            env: Vec::new(),
            scope_depth: 0,
            task_captures: Vec::new(),
            ui_depth: 0,
            state_symbols: HashSet::new(),
            global_symbols: HashSet::new(),
            cur_fn: String::new(),
            ret_provs: Vec::new(),
            vir: Vec::new(),
            has_tasks: false,
            writes_state: false,
            cur_params: Vec::new(),
            param_syms: HashMap::new(),
            consumed_params: HashSet::new(),
            decisions: Vec::new(),
            diagnostics: DiagnosticSink::new(),
            locs,
        }
    }

    pub fn analyze(mut self) -> ValueFlowResult {
        self.collect_states(&self.hir.items);
        let mut vfns = Vec::new();
        self.analyze_items(&self.hir.items, &mut vfns);
        ValueFlowResult {
            diagnostics: self.diagnostics,
            vfns,
            summaries: self.summaries,
            decisions: std::mem::take(&mut self.decisions),
        }
    }

    fn collect_states(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Ui(u) => {
                    for st in &u.states {
                        self.state_symbols.insert(st.sym);
                    }
                }
                HirItem::Global { sym, .. } => {
                    self.global_symbols.insert(*sym);
                }
                HirItem::Mod { items, .. } => self.collect_states(items),
                _ => {}
            }
        }
    }

    fn analyze_items(&mut self, items: &[HirItem], out: &mut Vec<VirFn>) {
        for item in items {
            match item {
                HirItem::Fn(f) => {
                    let vf = self.analyze_fn(f);
                    out.push(vf);
                }
                HirItem::Ui(u) => {
                    self.push_scope();
                    for st in &u.states {
                        let ty = self.types.binding_types.get(&st.sym).cloned().unwrap_or(Ty::Unknown);
                        let init_val = st.init.as_ref().map(|e| self.expr_val(e));
                        let v = init_val.unwrap_or_else(|| {
                            let node = self.node(VirNode::Unknown);
                            FlowVal {
                                prov: Prov::Unknown,
                                node,
                                consumed: false,
                                ty,
                                depth: self.scope_depth,
                            }
                        });
                        self.define(st.sym, v);
                    }
                    if let Some(render) = &u.render {
                        self.block(render);
                    }
                    self.pop_scope();
                }
                HirItem::Global { init, .. } => {
                    if let Some(e) = init {
                        self.expr_val(e);
                    }
                }
                HirItem::Mod { items, .. } => self.analyze_items(items, out),
                _ => {}
            }
        }
    }

    // ---- VIR builder ----

    fn node(&mut self, n: VirNode) -> usize {
        let id = self.vir.len();
        self.vir.push(n);
        id
    }

    // ---- environment ----

    fn push_scope(&mut self) {
        self.env.push(HashMap::new());
        self.scope_depth += 1;
    }

    fn pop_scope(&mut self) {
        self.env.pop();
        self.scope_depth -= 1;
    }

    fn define(&mut self, sym: SymbolId, v: FlowVal) {
        if let Some(frame) = self.env.last_mut() {
            frame.insert(sym, v);
        }
    }

    fn lookup(&self, sym: SymbolId) -> Option<FlowVal> {
        for frame in self.env.iter().rev() {
            if let Some(v) = frame.get(&sym) {
                return Some(v.clone());
            }
        }
        None
    }

    fn loc(&self, byte: usize) -> (u32, u32) {
        match self.locs.binary_search_by_key(&byte, |&(b, _, _)| b) {
            Ok(i) => (self.locs[i].1, self.locs[i].2),
            Err(0) => (0, 0),
            Err(i) => (self.locs[i - 1].1, self.locs[i - 1].2),
        }
    }

    fn err(&mut self, code: &str, msg: &str, span: Span) {
        let (line, col) = self.loc(span.start);
        self.diagnostics.push(
            Diagnostic::new(code, Severity::Error, "ownership error", msg, line, col)
                .with_span(span.start, span.end),
        );
    }

    fn symbol_name(&self, id: SymbolId) -> String {
        self.hir.symbols.get(id).map(|s| s.name.clone()).unwrap_or_default()
    }

    fn is_state(&self, sym: SymbolId) -> bool {
        self.state_symbols.contains(&sym)
    }

    /// Can this value be implemented as a zero-copy Compiler View?
    /// (design §7: input-derived provenance + non-Copy type)
    fn is_view(&self, v: &FlowVal) -> bool {
        prov_is_input_derived(&v.prov) && !ty_is_copy(&v.ty)
    }

    /// A view-eligible value escaped (stored / transferred / passed to an
    /// owning parameter): it must be materialized. Emit F2001 (design §11/§58).
    fn materialize(&mut self, v: &FlowVal, span: Span, reason: &str) {
        let class = size_class(&v.ty);
        self.decisions.push(OptDecision::Materialized { span, class });
        let (line, col) = self.loc(span.start);
        self.diagnostics.push(
            Diagnostic::new(
                vcodes::MATERIALIZATION,
                Severity::Warning,
                "materialization",
                &format!("这里物化了一个 {} 的值（{}）", class.display(), v.ty.display()),
                line,
                col,
            )
            .with_span(span.start, span.end)
            .with_cause(&format!("原因: 值来源 {}，被{}，无法继续借用", v.prov.display(), reason))
            .with_suggestion(
                "可选方案: 1) 接受当前拥有值实现  2) 拆分函数以帮助分析  3) 使用显式高级借用 API &T",
            ),
        );
    }

    /// Record the implementation decision for a newly created binding.
    fn record_decision(&mut self, span: Span, v: &FlowVal) {
        let decision = if self.is_view(v) {
            OptDecision::Borrow { span }
        } else if ty_is_copy(&v.ty) {
            OptDecision::Copy
        } else if matches!(v.prov, Prov::Owned) {
            OptDecision::Owned
        } else {
            OptDecision::Copy
        };
        self.decisions.push(decision);
    }

    fn ty_of(&self, sym: SymbolId) -> Ty {
        self.types.binding_types.get(&sym).cloned().unwrap_or(Ty::Unknown)
    }

    /// Try to read a binding as a value; reports use-after-move.
    fn read_binding(&mut self, sym: SymbolId, span: Span) -> FlowVal {
        if let Some(v) = self.lookup(sym) {
            if v.consumed && !self.captured_by_active_task(sym) {
                let name = self.symbol_name(sym);
                self.err(
                    vcodes::USE_AFTER_MOVE,
                    &format!("值 '{}' 已被移入任务或 UI 状态，此处不能再使用", name),
                    span,
                );
            }
            return v;
        }
        FlowVal {
            prov: Prov::Unknown,
            node: self.node(VirNode::Unknown),
            consumed: false,
            ty: self.ty_of(sym),
            depth: self.scope_depth,
        }
    }

    /// Mark a binding consumed (moved into a task / ui write / move param).
    /// Tracks consumed params for the summary.
    fn consume(&mut self, sym: SymbolId, reason: &'static str) {
        if let Some(idx) = self.param_syms.get(&sym) {
            self.consumed_params.insert(*idx);
        }
        for frame in self.env.iter_mut().rev() {
            if let Some(v) = frame.get_mut(&sym) {
                v.consumed = true;
                let node = v.node;
                self.node(VirNode::Consume { from: node, reason });
                return;
            }
        }
    }

    // ---- function analysis ----

    fn analyze_fn(&mut self, f: &FnInfo) -> VirFn {
        self.cur_fn = f.name.clone();
        self.vir = Vec::new();
        self.ret_provs = Vec::new();
        self.has_tasks = false;
        self.writes_state = false;
        self.consumed_params.clear();
        self.param_syms.clear();
        self.cur_params = f
            .params
            .iter()
            .map(|p| {
                let ty = self.types.binding_types.get(&p.sym).cloned().unwrap_or(Ty::Unknown);
                ParamSummary { name: p.name.clone(), is_move: p.is_move, consumed: false, ty }
            })
            .collect();
        self.push_scope();
        for (i, p) in f.params.iter().enumerate() {
            self.param_syms.insert(p.sym, i);
            let ty = self.types.binding_types.get(&p.sym).cloned().unwrap_or(Ty::Unknown);
            let prov = Prov::Input(i);
            let node = self.node(VirNode::Param { index: i, name: p.name.clone() });
            let v = FlowVal { prov, node, consumed: false, ty: ty.clone(), depth: self.scope_depth };
            // a non-move param may be implemented as a view of the caller's
            // value (design §22.1); move params are owned (§22.2)
            let decision = if p.is_move {
                OptDecision::Owned
            } else if self.is_view(&v) {
                OptDecision::Borrow { span: p.span }
            } else {
                OptDecision::Copy
            };
            self.decisions.push(decision);
            self.define(p.sym, v);
        }
        self.block(&f.body);
        self.pop_scope();

        let ret = self.reduce_ret_provs();
        for (i, p) in self.cur_params.iter_mut().enumerate() {
            p.consumed = self.consumed_params.contains(&i);
        }
        // Escape: 参数本体被返回（Input(i) 精确；投影返回是借用返回，
        // 调用点已物化返回视图，参数本身不逃逸）
        //（任务捕获逃逸已由 consumed 机制覆盖：捕获即消费）
        let mut escapes: Vec<usize> = Vec::new();
        if let Some(rp) = &ret {
            for (i, _) in f.params.iter().enumerate() {
                if prov_is_exact_input(rp, i) {
                    escapes.push(i);
                }
            }
        }
        escapes.sort_unstable();
        let summary = FnSummary {
            name: f.name.clone(),
            params: self.cur_params.clone(),
            ret,
            has_tasks: self.has_tasks,
            writes_state: self.writes_state,
            escapes,
        };
        self.summaries.insert(f.name.clone(), summary);
        VirFn { name: f.name.clone(), nodes: std::mem::take(&mut self.vir) }
    }

    /// Merge multiple return provenances: identical -> that; else Unknown
    /// (design §24: multi-source returns cannot be proven -> Materialize).
    fn reduce_ret_provs(&self) -> Option<Prov> {
        if self.ret_provs.is_empty() {
            return None;
        }
        let first = self.ret_provs[0].normalize();
        let all_same = self.ret_provs.iter().all(|p| p.normalize() == first);
        Some(if all_same { first } else { Prov::Unknown })
    }

    // ---- blocks & statements ----

    fn block(&mut self, b: &ast::Block) {
        self.push_scope();
        for s in &b.stmts {
            self.stmt(s);
        }
        if let Some(t) = &b.tail {
            let v = self.expr_val(t);
            if self.ui_depth == 0 {
                self.ret_provs.push(v.prov.clone());
                // returning an input-derived view materializes it (the return
                // value must be owned — the caller may store it)
                if self.is_view(&v) {
                    self.materialize(&v, expr_span(t), "返回");
                }
            }
        }
        self.pop_scope();
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let(ls) => {
                let res = self.resolution;
                let types = self.types;
                let ty = binding_sym(res, ls)
                    .and_then(|s| types.binding_types.get(&s).cloned())
                    .unwrap_or(Ty::Unknown);
                let init_val = ls.init.as_ref().map(|e| self.expr_val(e));
                let mut v = match init_val {
                    Some(mut v) => {
                        v.prov = Prov::Copy(Box::new(v.prov));
                        let node = self.node(VirNode::Copy { from: v.node });
                        v.node = node;
                        v.consumed = false;
                        v
                    }
                    None => {
                        let node = self.node(VirNode::Unknown);
                        FlowVal {
                            prov: Prov::Unknown,
                            node,
                            consumed: false,
                            ty: Ty::Unknown,
                            depth: self.scope_depth,
                        }
                    }
                };
                // the binding's type comes from typeck (not the init expr's
                // provisional type) — critical for Copy/move decisions
                v.ty = ty;
                if let Some(sym) = self.resolution.bindings.get(&ls.span) {
                    self.record_decision(ls.span, &v);
                    self.define(*sym, v);
                }
            }
            Stmt::Expr(e) => {
                self.expr_val(e);
            }
            Stmt::Return(v) => {
                if let Some(e) = v {
                    let val = self.expr_val(e);
                    self.ret_provs.push(val.prov);
                    self.node(VirNode::Return { from: val.node });
                }
            }
            Stmt::If(ifexpr) => {
                self.expr_val(&ifexpr.cond);
                self.block(&ifexpr.then_branch);
                if let Some(e) = &ifexpr.else_branch {
                    self.expr_val(e);
                }
            }
            Stmt::While { cond, body, .. } => {
                self.expr_val(cond);
                self.block(body);
            }
            Stmt::For { pat, iter, body, .. } => {
                self.expr_val(iter);
                self.push_scope();
                let res = self.resolution;
                let types = self.types;
                if let Some(sym) = res.bindings.get(&pat_span(pat)) {
                    let ty = types.binding_types.get(sym).cloned().unwrap_or(Ty::Unknown);
                    let node = self.node(VirNode::Unknown);
                    self.define(
                        *sym,
                        FlowVal {
                            prov: Prov::Unknown,
                            node,
                            consumed: false,
                            ty,
                            depth: self.scope_depth,
                        },
                    );
                }
                self.block(body);
                self.pop_scope();
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Defer(e) => {
                self.expr_val(e);
            }
            Stmt::TryCatch(tc) => {
                self.block(&tc.try_block);
                if let Some(p) = &tc.catch_param {
                    self.push_scope();
                    if let Some(sym) = self.resolution.bindings.get(&p.span) {
                        let ty = self.types.binding_types.get(sym).cloned().unwrap_or(Ty::Unknown);
                        let node = self.node(VirNode::Unknown);
                        self.define(
                            *sym,
                            FlowVal {
                                prov: Prov::Unknown,
                                node,
                                consumed: false,
                                ty,
                                depth: self.scope_depth,
                            },
                        );
                    }
                    self.block(&tc.catch_block);
                    self.pop_scope();
                } else {
                    self.block(&tc.catch_block);
                }
            }
            Stmt::Block(b) => self.block(b),
        }
    }

    // ---- expressions ----

    fn expr_val(&mut self, e: &Expr) -> FlowVal {
        match e {
            Expr::Int(_) | Expr::Float(_) => FlowVal {
                prov: Prov::Owned,
                node: self.node(VirNode::Literal),
                consumed: false,
                ty: Ty::I32,
                depth: self.scope_depth,
            },
            Expr::Str(_) | Expr::FmtStr(_) => FlowVal {
                prov: Prov::Owned,
                node: self.node(VirNode::Literal),
                consumed: false,
                ty: Ty::Str,
                depth: self.scope_depth,
            },
            Expr::Bool(_) => FlowVal {
                prov: Prov::Owned,
                node: self.node(VirNode::Literal),
                consumed: false,
                ty: Ty::Bool,
                depth: self.scope_depth,
            },
            Expr::Ident { span, .. } => {
                let sym = self.resolution.get(span);
                                match sym {
                    Some(sym) => {
                        // UI-domain state read is not a task capture (§34)
                        if self.is_state(sym) && self.ui_depth > 0 {
                            let node = self.node(VirNode::StateRead { sym });
                            return FlowVal {
                                prov: Prov::Unknown,
                                node,
                                consumed: false,
                                ty: self.ty_of(sym),
                                depth: self.scope_depth,
                            };
                        }
                        self.check_capture(sym);
                        self.read_binding(sym, *span)
                    }
                    None => FlowVal {
                        prov: Prov::Unknown,
                        node: self.node(VirNode::Unknown),
                        consumed: false,
                        ty: Ty::Unknown,
                        depth: self.scope_depth,
                    },
                }
            }
            Expr::Path(_) => FlowVal {
                prov: Prov::Unknown,
                node: self.node(VirNode::Unknown),
                consumed: false,
                ty: Ty::Unknown,
                depth: self.scope_depth,
            },
            Expr::Array(items) => {
                for i in items {
                    let iv = self.expr_val(i);
                    if self.is_view(&iv) {
                        self.materialize(&iv, expr_span(i), "存入数组");
                    }
                }
                FlowVal {
                    prov: Prov::Owned,
                    node: self.node(VirNode::Literal),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Tuple(items) => {
                for i in items {
                    let iv = self.expr_val(i);
                    if self.is_view(&iv) {
                        self.materialize(&iv, expr_span(i), "存入元组");
                    }
                }
                FlowVal {
                    prov: Prov::Owned,
                    node: self.node(VirNode::Literal),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::StructLit { fields, .. } => {
                for (_, v) in fields {
                    let fv = self.expr_val(v);
                    if self.is_view(&fv) {
                        self.materialize(&fv, expr_span(v), "存入结构体字段");
                    }
                }
                FlowVal {
                    prov: Prov::Owned,
                    node: self.node(VirNode::Literal),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Unary { op, expr } => {
                let v = self.expr_val(expr);
                match op {
                    crate::ast::UnOp::Ref => {
                        let node = self.node(VirNode::Borrow { from: v.node, mutable: false });
                        FlowVal { prov: Prov::Unknown, node, consumed: false, ty: v.ty, depth: self.scope_depth }
                    }
                    _ => v,
                }
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.expr_val(lhs);
                self.expr_val(rhs);
                FlowVal {
                    prov: Prov::Owned,
                    node: self.node(VirNode::Literal),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Assign { target, value, .. } => {
                let is_state_write = self.assign_target(target);
                let v = self.expr_val(value);
                if is_state_write {
                    if let Expr::Ident { span, .. } = &**value {
                        if let Some(sym) = self.resolution.get(span) {
                            if self.is_view(&v) {
                                self.materialize(&v, *span, "写入 UI 状态");
                            }
                            self.consume(sym, "ui state write");
                            self.node(VirNode::StateWrite {
                                state: self.symbol_name(sym),
                                from: v.node,
                            });
                        }
                    }
                    self.writes_state = true;
                } else if let Expr::Ident { span, .. } = &**target {
                    if let Some(sym) = self.resolution.get(span) {
                        self.define(
                            sym,
                            FlowVal {
                                prov: Prov::Copy(Box::new(v.prov.clone())),
                                node: v.node,
                                consumed: false,
                                ty: v.ty.clone(),
                                depth: self.scope_depth,
                            },
                        );
                    }
                }
                v
            }
            Expr::Call { callee, args, .. } => self.call_val(callee, args),
            Expr::Field { base, field, span } => {
                let b = self.expr_val(base);
                let node = self.node(VirNode::Projection {
                    from: b.node,
                    field: Some(field.clone()),
                    element: false,
                });
                // the field type comes from typeck (critical for view decisions)
                let ty = self.types.types.get(span).cloned().unwrap_or(Ty::Unknown);
                FlowVal {
                    prov: Prov::Field(Box::new(b.prov), field.clone()),
                    node,
                    consumed: false,
                    ty,
                    depth: self.scope_depth,
                }
            }
            Expr::Index { base, index } => {
                let b = self.expr_val(base);
                self.expr_val(index);
                let node = self.node(VirNode::Projection { from: b.node, field: None, element: true });
                FlowVal {
                    prov: Prov::Element(Box::new(b.prov)),
                    node,
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Try(e) => {
                let v = self.expr_val(e);
                FlowVal { prov: Prov::Transform(Box::new(v.prov)), ..v }
            }
            Expr::Range { start, end } => {
                if let Some(s) = start {
                    self.expr_val(s);
                }
                if let Some(e) = end {
                    self.expr_val(e);
                }
                FlowVal {
                    prov: Prov::Owned,
                    node: self.node(VirNode::Literal),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Block(b) => {
                self.block(b);
                FlowVal {
                    prov: Prov::Unknown,
                    node: self.node(VirNode::Unknown),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::If(ifexpr) => {
                self.expr_val(&ifexpr.cond);
                self.block(&ifexpr.then_branch);
                if let Some(e) = &ifexpr.else_branch {
                    self.expr_val(e);
                }
                FlowVal {
                    prov: Prov::Unknown,
                    node: self.node(VirNode::Unknown),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Match { scrutinee, arms, .. } => {
                self.expr_val(scrutinee);
                for arm in arms {
                    self.push_scope();
                    let res = self.resolution;
                    let types = self.types;
                    for (_, span) in arm.pat.bindings() {
                        if let Some(sym) = res.bindings.get(&span) {
                            let ty = types.binding_types.get(sym).cloned().unwrap_or(Ty::Unknown);
                            let node = self.node(VirNode::Unknown);
                            self.define(
                                *sym,
                                FlowVal {
                                    prov: Prov::Unknown,
                                    node,
                                    consumed: false,
                                    ty,
                                    depth: self.scope_depth,
                                },
                            );
                        }
                    }
                    self.expr_val(&arm.value);
                    self.pop_scope();
                }
                FlowVal {
                    prov: Prov::Unknown,
                    node: self.node(VirNode::Unknown),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Closure { params, body, .. } => {
                // ordinary closure: borrow captures (§25.1), no consumption
                self.push_scope();
                let res = self.resolution;
                let types = self.types;
                for p in params {
                    if let Some(sym) = res.bindings.get(&p.span) {
                        let ty = types.binding_types.get(sym).cloned().unwrap_or(Ty::Unknown);
                        let node = self.node(VirNode::Unknown);
                        self.define(
                            *sym,
                            FlowVal {
                                prov: Prov::Unknown,
                                node,
                                consumed: false,
                                ty,
                                depth: self.scope_depth,
                            },
                        );
                    }
                }
                self.expr_val(body);
                self.pop_scope();
                FlowVal {
                    prov: Prov::Unknown,
                    node: self.node(VirNode::Unknown),
                    consumed: false,
                    ty: Ty::Unknown,
                    depth: self.scope_depth,
                }
            }
            Expr::Go { block, detached, .. } => self.go_val(block, *detached),
            Expr::UiOp { block, .. } => self.ui_val(block),
            Expr::Paren(inner) => self.expr_val(inner),
        }
    }

    /// Task capture: an ident use of a binding defined outside the task's
    /// scope transfers ownership if the value is not Copy (§25.2, §31).
    fn check_capture(&mut self, sym: SymbolId) {
        let Some(v) = self.lookup(sym) else { return };
        let Some((task_start, _)) = self.task_captures.last() else { return };
        // bindings at or above the task-start depth live OUTSIDE the task
        // (the go expression sits in the enclosing scope); only bindings
        // strictly deeper are task-internal and not captures.
        if v.depth > *task_start {
            return;
        }
        let is_new = {
            let captures = &mut self.task_captures.last_mut().unwrap().1;
            captures.insert(sym)
        };
        if is_new && !ty_is_copy(&v.ty) {
            if self.is_view(&v) {
                self.materialize(&v, symbol_span(&self.hir, sym), "移入任务");
            }
            self.consume(sym, "task capture");
            self.node(VirNode::TaskTransfer { from: v.node, detached: false });
        }
        // Send 检查（Concurrency Guide §3）：跨任务值必须 Send；非 Send 报业务语言诊断
        if is_new && !ty_is_send(&v.ty, &self.typeck_structs, &self.typeck_not_send) {
            let (line, col) = self.loc(symbol_span(&self.hir, sym).start);
            self.diagnostics.push(
                Diagnostic::new(
                    vcodes::NOT_SEND,
                    Severity::Error,
                    "not-send",
                    "这个值不能安全地发送到后台任务。",
                    line,
                    col,
                )
                .with_span(symbol_span(&self.hir, sym).start, symbol_span(&self.hir, sym).end)
                .with_cause("原因：它包含了无法跨任务安全移动的数据（如平台句柄）。")
                .with_suggestion(
                    "建议：提取真正需要的数据（id / 查询参数）后发送；或使用 @not_send 显式声明（不推荐用于跨任务）。",
                ),
            );
        }
    }

    /// Is this symbol captured by an active task (legal to read inside it)?
    fn captured_by_active_task(&self, sym: SymbolId) -> bool {
        self.task_captures.iter().any(|(_, set)| set.contains(&sym))
    }

    fn go_val(&mut self, block: &ast::Block, detached: bool) -> FlowVal {
        // UI Guide §4.2 禁 1：ui{} 内不允许发起新任务
        if self.ui_depth > 0 {
            self.err(
                vcodes::UI_GO_INSIDE,
                "ui{} 内不允许发起 go{}/go!{}——请在 ui{} 外部创建后台任务",
                block.span,
            );
        }
        self.has_tasks = true;
        self.task_captures.push((self.scope_depth, HashSet::new()));
        self.push_scope();
        self.block(block);
        self.pop_scope();
        self.task_captures.pop();
        let node = self.node(VirNode::TaskTransfer { from: 0, detached });
        FlowVal {
            prov: Prov::Unknown,
            node,
            consumed: false,
            ty: Ty::Unknown,
            depth: self.scope_depth,
        }
    }

    fn ui_val(&mut self, block: &ast::Block) -> FlowVal {
        self.ui_depth += 1;
        self.push_scope();
        self.block(block);
        self.pop_scope();
        self.ui_depth -= 1;
        FlowVal {
            prov: Prov::Unknown,
            node: self.node(VirNode::Unknown),
            consumed: false,
            ty: Ty::Unknown,
            depth: self.scope_depth,
        }
    }

    /// Handle an assignment target. Returns true if it is a ui state write
    /// (the value will be consumed). State writes outside ui{} are errors.
    fn assign_target(&mut self, target: &Expr) -> bool {
        if let Expr::Ident { span, .. } = target {
            if let Some(sym) = self.resolution.get(span) {
                if self.global_symbols.contains(&sym) && self.ui_depth == 0 {
                    self.err(
                        vcodes::GLOBAL_WRITE_OUTSIDE_UI,
                        "@global 只能在 ui{} 内写入（受控消息路径）",
                        *span,
                    );
                }
                if self.is_state(sym) {
                    if self.ui_depth > 0 {
                        return true;
                    } else {
                        let name = self.symbol_name(sym);
                        self.err(
                            vcodes::STATE_WRITE_OUTSIDE_UI,
                            &format!("状态 '{}' 只能在 ui {{}} 中写入", name),
                            *span,
                        );
                    }
                }
            }
        }
        false
    }

    /// Call site: apply fn summary composition (§17); move params consume args.
    fn call_val(&mut self, callee: &Expr, args: &[crate::ast::CallArg]) -> FlowVal {
        if let Expr::Field { base, .. } = callee {
            self.expr_val(base);
            for a in args {
                self.expr_val(&a.expr);
            }
            return FlowVal {
                prov: Prov::Unknown,
                node: self.node(VirNode::Unknown),
                consumed: false,
                ty: Ty::Unknown,
                depth: self.scope_depth,
            };
        }
        let callee_name = match callee {
            Expr::Ident { name, .. } => name.clone(),
            _ => String::new(),
        };
        let known = self.summaries.get(&callee_name).cloned();
        let mut arg_vals: Vec<FlowVal> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let v = self.expr_val(&a.expr);
            if let Some(sig) = &known {
                if let Some(p) = sig.params.get(i) {
                    if p.is_move {
                        if self.is_view(&v) {
                            self.materialize(&v, a.span, "move 参数转移");
                        }
                        if let Expr::Ident { span, .. } = &a.expr {
                            if let Some(sym) = self.resolution.get(span) {
                                self.consume(sym, "move param");
                                self.node(VirNode::Move { from: v.node });
                            }
                        }
                    } else if !matches!(p.ty, Ty::Ref(_, _)) && p.consumed {
                        // non-move value param that the CALLEE consumes (its
                        // body moves it into a task / ui write): the caller
                        // must provide an owned value — materialize a view.
                        // (design §22.1: otherwise the callee borrows internally)
                        if self.is_view(&v) {
                            self.materialize(&v, a.span, "按值传参（被调方会消费）");
                        }
                    }
                    // Escape (§15-18): callee returns/stores the param — the
                    // caller's view can no longer be borrowed; materialize.
                    if sig.escapes.contains(&i) {
                        if self.is_view(&v) {
                            self.materialize(&v, a.span, "参数逃逸（被调方返回/存储）");
                            let (line, col) = self.loc(a.span.start);
                            self.diagnostics.push(
                                Diagnostic::new(
                                    vcodes::ESCAPE,
                                    Severity::Info,
                                    "escape",
                                    &format!("参数 {} 逃逸进 {}(返回值/存储)，视图需物化", i, callee_name),
                                    line,
                                    col,
                                )
                                .with_span(a.span.start, a.span.end)
                                .with_cause(&format!("被调方摘要: {} 的返回 prov 引用该参数", callee_name)),
                            );
                        }
                    }
                }
            }
            arg_vals.push(v);
        }
        let arg_provs: Vec<Prov> = arg_vals.iter().map(|v| v.prov.clone()).collect();
        let result_prov = match &known {
            Some(sig) => sig.ret.as_ref().map(|r| r.substitute(&arg_provs)).unwrap_or(Prov::Unknown),
            None => Prov::Unknown,
        };
        let node = self.node(VirNode::Call {
            callee: callee_name,
            args: arg_vals.iter().map(|v| v.node).collect(),
        });
        FlowVal {
            prov: result_prov,
            node,
            consumed: false,
            ty: Ty::Unknown,
            depth: self.scope_depth,
        }
    }
}

// ---- helpers ----

fn binding_sym(resolution: &Resolution, ls: &crate::ast::LetStmt) -> Option<SymbolId> {
    resolution.bindings.get(&ls.span).copied()
}

/// Span of a symbol's definition (for diagnostics).
fn symbol_span(hir: &HirProgram, sym: SymbolId) -> Span {
    hir.symbols.get(sym).map(|s| s.span).unwrap_or(Span::new(0, 0))
}

/// Best-effort span of an expression (for diagnostics).
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
        _ => Span::new(0, 0),
    }
}

fn pat_span(p: &crate::ast::Pattern) -> Span {
    match p {
        crate::ast::Pattern::Ident { span, .. } => *span,
        crate::ast::Pattern::Wildcard => Span::new(0, 0),
        crate::ast::Pattern::Lit(_) => Span::new(0, 0),
        crate::ast::Pattern::Path(path) => path.span,
        crate::ast::Pattern::Variant { .. } => Span::new(0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Resolver;
    use crate::typeck::TypeChecker;

    fn va_src(src: &str) -> (ValueFlowResult, DiagnosticSink, DiagnosticSink) {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let resolver = Resolver::new(&lexed.tokens);
        let resolved = resolver.resolve(&program);
        let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
        let typeck = checker.check();
        let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
        (vf, typeck.diagnostics, resolved.diagnostics)
    }

    fn va_errs(src: &str) -> Vec<String> {
        let (vf, _, _) = va_src(src);
        vf.diagnostics.diagnostics.iter().map(|d| d.code.clone()).collect()
    }

    #[test]
    fn task_capture_consumes_non_copy() {
        let src = "fn big() -> String { \"d\" }\nfn work(s: String) { }\nfn main() { let data = big()\ngo { work(data) }\nprint(data) }";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::USE_AFTER_MOVE.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn task_capture_legal_inside_task() {
        let src = "fn big() -> String { \"d\" }\nfn work(s: String) { }\nfn main() { let data = big()\ngo { work(data)\nprint(data) }\nprint(\"done\") }";
        let (vf, _, _) = va_src(src);
        assert_eq!(vf.diagnostics.error_count(), 0, "{:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn copy_types_not_consumed_by_capture() {
        let src = "fn main() { let x = 5\ngo { print(x) }\nprint(x) }";
        let (vf, _, _) = va_src(src);
        assert_eq!(vf.diagnostics.error_count(), 0, "{:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn ui_write_consumes_value() {
        let src = "ui C {\n    @state\n    var records: Vec<i32> = []\n    render {\n        Button(\"q\") {\n            go {\n                let fresh = load()\n                ui {\n                    records = fresh\n                }\n                print(fresh)\n            }\n        }\n    }\n}\nfn load() -> Vec<i32> { [] }";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::USE_AFTER_MOVE.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn move_param_consumes_argument() {
        let src = "struct User { name: String }\nfn store(move user: User) { }\nfn main() { let u = User { name: \"a\" }\nstore(u)\nprint(u) }";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::USE_AFTER_MOVE.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn field_read_does_not_consume() {
        let src = "struct User { name: String }\nfn main(u: User) { let n = u.name\nprint(u) }";
        let (vf, _, _) = va_src(src);
        assert_eq!(vf.diagnostics.error_count(), 0, "{:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn borrow_param_does_not_consume() {
        let src = "struct User { name: String }\nfn inspect(s: &User) { }\nfn main(u: User) { inspect(u)\nprint(u) }";
        let (vf, _, _) = va_src(src);
        assert_eq!(vf.diagnostics.error_count(), 0, "{:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn summary_single_source_field() {
        let (vf, _, _) = va_src("struct User { name: String }\nfn get_label(user: User) -> String { user.name }");
        let s = vf.summaries.get("get_label").expect("summary");
        let ret = s.ret.as_ref().expect("ret");
        assert_eq!(ret.normalize(), Prov::Field(Box::new(Prov::Input(0)), "name".to_string()));
    }

    #[test]
    fn summary_multi_source_unknown() {
        let (vf, _, _) = va_src(
            "struct User { name: String age: i32 }\nfn choose(a: User, b: User) -> String { if a.age > b.age { return a.name }\nreturn b.name }",
        );
        let s = vf.summaries.get("choose").expect("summary");
        assert_eq!(s.ret.as_ref().expect("ret").normalize(), Prov::Unknown);
    }

    #[test]
    fn composition_propagates_field() {
        let (vf, _, _) = va_src(
            "struct User { name: String }\nfn get_label(user: User) -> String { user.name }\nfn show(u: User) { print(get_label(u)) }",
        );
        assert!(vf.summaries.contains_key("get_label"));
        assert!(vf.summaries.contains_key("show"));
    }

    #[test]
    fn state_write_outside_ui_errors() {
        let src = "ui C {\n    @state\n    var records: Vec<i32> = []\n    render {\n        Button(\"x\") {\n            records = []\n        }\n    }\n}";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::STATE_WRITE_OUTSIDE_UI.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn canonical_examples_clean() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let (vf, t, r) = va_src(&src);
            assert_eq!(vf.diagnostics.error_count(), 0, "{} vf: {:?}", name, vf.diagnostics.diagnostics);
            assert_eq!(t.error_count(), 0, "{} type: {:?}", name, t.diagnostics);
            assert_eq!(r.error_count(), 0, "{} sem: {:?}", name, r.diagnostics);
        }
    }

    #[test]
    fn materialization_hint_on_consuming_call() {
        let src = "struct User {\n    name: String\n}\nfn get_label(user: User) -> String {\n    user.name\n}\nfn stash(s: String) {\n    go {\n        burn(s)\n    }\n}\nfn burn(s: String) { }\nfn f(u: User) {\n    let label = get_label(u)\n    stash(label)\n}";
        let (vf, _, _) = va_src(src);
        let hints = vf.diagnostics.diagnostics.iter().filter(|d| d.code == vcodes::MATERIALIZATION).count();
        assert!(hints >= 1, "expected materialization hints, got {:?}", vf.diagnostics.diagnostics);
        assert!(vf.decisions.iter().any(|d| matches!(d, OptDecision::Borrow { .. })));
    }

    #[test]
    fn view_reuse_no_hint_for_non_consuming_callee() {
        let src = "struct User {\n    name: String\n}\nfn get_label(user: User) -> String {\n    user.name\n}\nfn save(s: String) { }\nfn f(u: User) {\n    let label = get_label(u)\n    save(label)\n    print(label)\n}";
        let (vf, _, _) = va_src(src);
        let hints = vf.diagnostics.diagnostics.iter().filter(|d| d.code == vcodes::MATERIALIZATION).count();
        assert_eq!(hints, 1, "got {:?}", vf.diagnostics.diagnostics);
        assert!(vf.decisions.iter().any(|d| matches!(d, OptDecision::Borrow { .. })));
    }

    #[test]
    fn examples_have_no_materialization_hints() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let (vf, _, _) = va_src(&src);
            let hints = vf.diagnostics.diagnostics.iter().filter(|d| d.code == vcodes::MATERIALIZATION).count();
            assert_eq!(hints, 0, "{} has materialization hints: {:?}", name, vf.diagnostics.diagnostics);
        }
    }

    #[test]
    fn budget_deep_provenance_degrades_to_unknown() {
        let mut p = Prov::Input(0);
        for _ in 0..(PROV_DEPTH_CAP + 5) {
            p = Prov::Field(Box::new(p), "f".to_string());
        }
        assert_eq!(p.normalize(), Prov::Unknown);
        let shallow = Prov::Field(Box::new(Prov::Input(0)), "name".to_string());
        assert_eq!(shallow.normalize(), Prov::Field(Box::new(Prov::Input(0)), "name".to_string()));
    }

    #[test]
    fn size_classes() {
        assert_eq!(size_class(&Ty::I32), SizeClass::Cheap);
        assert_eq!(size_class(&Ty::Str), SizeClass::Large);
        assert_eq!(size_class(&Ty::Vec(Box::new(Ty::I32))), SizeClass::Large);
        assert_eq!(size_class(&Ty::Unknown), SizeClass::Cheap);
    }

    #[test]
    fn prov_substitution_and_is_input_derived() {
        let ret = Prov::Field(Box::new(Prov::Input(0)), "name".to_string());
        let args = vec![Prov::Owned];
        assert_eq!(ret.substitute(&args), Prov::Field(Box::new(Prov::Owned), "name".to_string()));
        assert!(prov_is_input_derived(&ret));
        assert!(!prov_is_input_derived(&Prov::Owned));
        assert!(!prov_is_input_derived(&Prov::Unknown));
    }

    #[test]
    fn go_inside_ui_errors() {
        let src = "ui C {
    @state
    var count: i32 = 0
    render {
        Button(\"x\") {
            ui {
                go { count = 1 }
            }
        }
    }
}";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::UI_GO_INSIDE.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn global_write_outside_ui_errors() {
        let src = "@global
var settings: String = \"a\"
fn main() { settings = \"b\" }";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::GLOBAL_WRITE_OUTSIDE_UI.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn not_send_cross_task_errors() {
        let src = "@not_send
struct Handle { raw: i32 }
struct User { name: String
h: Handle }
fn main() { let u = User { name: \"a\" h: Handle { raw: 1 } }
go { print(u.name) }
print(\"done\") }";
        let errs = va_errs(src);
        assert!(errs.contains(&vcodes::NOT_SEND.to_string()), "got: {:?}", errs);
    }

    #[test]
    fn send_struct_cross_task_ok() {
        let src = "struct User { name: String }
fn main() { let u = User { name: \"a\" }
go { print(u.name) }
print(\"done\") }";
        let (vf, _, _) = va_src(src);
        assert_eq!(vf.diagnostics.error_count(), 0, "got: {:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn escape_detected_on_returning_param() {
        let src = "struct User { name: String }
fn pass(u: User) -> User { u }
fn f(u: User) { let r = pass(u)
print(r.name) }";
        let (vf, _, _) = va_src(src);
        let sig = vf.summaries.get("pass").expect("pass summary");
        assert_eq!(sig.escapes, vec![0], "param 0 should escape via return");
        assert!(
            vf.diagnostics.diagnostics.iter().any(|d| d.code == vcodes::ESCAPE),
            "expected escape hint, got {:?}",
            vf.diagnostics.diagnostics
        );
    }

    #[test]
    fn no_escape_materialization_for_copy_types() {
        let src = "fn id(x: i32) -> i32 { x }
fn f() { let a = 5
let b = id(a)
print(b) }";
        let (vf, _, _) = va_src(src);
        let sig = vf.summaries.get("id").expect("id summary");
        assert_eq!(sig.escapes, vec![0], "param escapes via return even for copy");
        let esc_hints = vf.diagnostics.diagnostics.iter().filter(|d| d.code == vcodes::ESCAPE).count();
        assert_eq!(esc_hints, 0, "copy param should not materialize, got {:?}", vf.diagnostics.diagnostics);
    }
}
