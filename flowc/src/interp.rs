//! Tree-walking interpreter (minimal executable closure for Aine).
//!
//! Lets the aine run command actually execute Aine programs. Scope (honest):
//! - full core semantics: fn/let/if/else/for/while/return/break/continue/
//!   match (incl. enum variant patterns), arithmetic/comparison/logic,
//!   strings + f-string interpolation, arrays, structs, Maps, user enums,
//!   impl methods, modules, closures with map/sum, Option/Result with ?
//!   propagation, defer, try/catch;
//! - STUBS (runnable, not real): UI components render nothing, db methods
//!   are no-ops/empty, go{}/ui{} run synchronously.

use std::collections::HashMap;

use crate::ast::{self, Expr, Item, Stmt};

/// FNV-1a hasher: fast for long string keys (char cache lookups).
#[derive(Default)]
#[allow(dead_code)]
struct FnvHasher(u64);
impl std::hash::Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf2_9ce4_8422_2325 } else { self.0 };
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
    }
}

/// A runtime value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(std::sync::Arc<str>),
    Unit,
    Vec(std::sync::Arc<Vec<Value>>),
    Tuple(Vec<Value>),
    Struct { ty: String, fields: Vec<(String, Value)> },
    /// ordered Map<String, V>
    Map(Vec<(String, Value)>),
    /// the Map type itself (for Map.new())
    MapType,
    /// user enum variant: Number(5), Binary(l, r)
    Variant { ty: String, name: String, payload: Vec<Value> },
    Option { some: bool, value: Box<Value> },
    Result { ok: bool, value: Box<Value> },
    Closure {
        params: Vec<String>,
        body: Box<Expr>,
        captured: Vec<HashMap<String, Value>>,
    },
    Range { start: i64, end: i64 },
    /// UI component stub result
    Nil,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "i32",
            Value::Float(_) => "f64",
            Value::Bool(_) => "bool",
            Value::Str(_) => "String",
            Value::Unit => "()",
            Value::Vec(_) => "Vec",
            Value::Tuple(_) => "tuple",
            Value::Struct { .. } => "struct",
            Value::Map(_) | Value::MapType => "Map",
            Value::Variant { .. } => "enum-variant",
            Value::Option { some, .. } => {
                if *some {
                    "Some"
                } else {
                    "None"
                }
            }
            _ => "value",
        }
    }

    pub fn display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                let s = format!("{}", f);
                if s.contains('.') || s.contains('e') || s.contains('E') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::Unit => "()".to_string(),
            Value::Vec(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.display()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Tuple(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.display()).collect();
                format!("({})", inner.join(", "))
            }
            Value::Struct { ty, fields } => {
                let inner: Vec<String> = fields.iter().map(|(n, v)| format!("{}: {}", n, v.display())).collect();
                format!("{} {{ {} }}", ty, inner.join(", "))
            }
            Value::Map(entries) => {
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display()))
                    .collect();
                format!("{{ {} }}", inner.join(", "))
            }
            Value::MapType => "<Map>".to_string(),
            Value::Variant { name, payload, .. } => {
                if payload.is_empty() {
                    name.clone()
                } else {
                    let inner: Vec<String> = payload.iter().map(|v| v.display()).collect();
                    format!("{}({})", name, inner.join(", "))
                }
            }
            Value::Option { some, value } => {
                if *some {
                    format!("Some({})", value.display())
                } else {
                    "None".to_string()
                }
            }
            Value::Result { ok, value } => {
                if *ok {
                    format!("Ok({})", value.display())
                } else {
                    format!("Err({})", value.display())
                }
            }
            Value::Closure { .. } => "<closure>".to_string(),
            Value::Range { start, end } => format!("{}..{}", start, end),
            Value::Nil => "<nil>".to_string(),
        }
    }
}

/// How a statement execution ended.
#[derive(Debug, Clone)]
pub enum ControlFlow {
    Ok,
    Return(Value),
    Break,
    Continue,
}

/// Runtime abort: either a hard error or a ?-propagated error value.
#[derive(Debug, Clone)]
pub enum RtError {
    /// hard error: aborts the whole program
    Msg(String),
    /// a Err value propagated by ? — becomes the current function's Result
    Propagate(Value),
    /// a return statement executed inside an expression block; the
    /// enclosing function must return this value
    Return(Value),
}

impl RtError {
    pub fn msg(s: impl Into<String>) -> RtError {
        RtError::Msg(s.into())
    }
}

/// The interpreter.
pub struct Interp {
    /// recursion depth guard (debug)
    eval_depth: usize,
    /// call recursion depth guard (debug)
    call_depth: usize,
    /// exec recursion depth guard (debug)
    exec_depth: usize,
    /// user function definitions by name
    fns: HashMap<String, std::rc::Rc<ast::FnDef>>,
    /// #[test] function names in source order (test framework)
    test_fns: Vec<String>,
    /// struct definitions by name
    structs: HashMap<String, Vec<(String, Option<ast::Type>)>>,
    /// enum variant name -> (enum type name, payload arity)
    variants: HashMap<String, (String, usize)>,
    /// enum definitions by name
    enums: HashMap<String, Vec<String>>,
    /// impl methods by struct name (self bound as first param)
    impls: HashMap<String, Vec<std::rc::Rc<ast::FnDef>>>,
    /// global variables (@global)
    globals: HashMap<String, Value>,
    /// ui component names (values in run_ui(...))
    components: std::collections::HashSet<String>,
    /// ui component definitions (UI 运行时渲染)
    ui_defs: std::collections::HashMap<String, ast::UiDef>,
    /// output sink (captured by tests)
    pub output: String,
    /// read_line 注入缓冲(测试/服务用): 优先消费, 空则读真实 stdin
    pub input_lines: std::collections::VecDeque<String>,
    current_fn: String,
    fn_stack: Vec<String>,
    /// string char cache keyed by Arc address; the stored Arc keeps the
    /// allocation alive so a recycled address can never hit stale chars
    char_cache: std::cell::RefCell<std::collections::HashMap<usize, (std::sync::Arc<str>, std::sync::Arc<Vec<char>>)>>,
    /// 调试断点（行号）与命中记录（行号 + 变量快照摘要）
    pub breakpoints: std::collections::HashSet<usize>,
    pub bp_hits: Vec<(usize, String)>,
    /// token 行号表（byte offset → (line, col)）
    locs: Vec<(usize, u32, u32)>,
}

/// Environment: a stack of variable frames.
#[derive(Clone, Debug)]
pub struct Env {
    frames: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Env {
        Env { frames: vec![HashMap::new()] }
    }

    pub fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        self.frames.pop();
    }

    pub fn define(&mut self, name: &str, v: Value) {
        self.frames.last_mut().unwrap().insert(name.to_string(), v);
    }

    pub fn assign(&mut self, name: &str, v: Value) -> bool {
        for frame in self.frames.iter_mut().rev() {
            if frame.contains_key(name) {
                frame.insert(name.to_string(), v);
                return true;
            }
        }
        false
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        for frame in self.frames.iter().rev() {
            if let Some(v) = frame.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    pub fn snapshot(&self) -> Vec<HashMap<String, Value>> {
        self.frames.clone()
    }
}

impl Interp {
    pub fn new() -> Interp {
        Interp {
            eval_depth: 0,
            call_depth: 0,
            exec_depth: 0,
            fns: HashMap::new(),
            test_fns: Vec::new(),
            structs: HashMap::new(),
            variants: HashMap::new(),
            enums: HashMap::new(),
            impls: HashMap::new(),
            globals: HashMap::new(),
            components: std::collections::HashSet::new(),
            ui_defs: std::collections::HashMap::new(),
            output: String::new(),
            input_lines: std::collections::VecDeque::new(),
            breakpoints: std::collections::HashSet::new(),
            bp_hits: Vec::new(),
            locs: Vec::new(),
            char_cache: std::cell::RefCell::new(std::collections::HashMap::new()),
            current_fn: String::new(),
            fn_stack: Vec::new(),
        }
    }

    /// 设置断点行号表（调试用）
    pub fn set_locs(&mut self, tokens: &[crate::token::Token]) {
        self.locs = tokens
            .iter()
            .map(|t| (t.span.start, t.loc.line, t.loc.col))
            .collect();
    }

    /// 断点行号（字节偏移 → 1-based 行号）
    fn line_of(&self, byte: usize) -> u32 {
        match self.locs.binary_search_by_key(&byte, |&(b, _, _)| b) {
            Ok(i) => self.locs[i].1,
            Err(0) => 1,
            Err(i) => self.locs[i - 1].1,
        }
    }

    fn stmt_line(&self, s: &Stmt) -> Option<u32> {
        let sp = match s {
            Stmt::Let(ls) => ls.span,
            Stmt::If(ifx) => ifx.span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Break(sp) | Stmt::Continue(sp) => *sp,
            _ => return None,
        };
        Some(self.line_of(sp.start))
    }

    fn check_breakpoint(&mut self, s: &Stmt, env: &Env) {
        if self.breakpoints.is_empty() {
            return;
        }
        if let Some(line) = self.stmt_line(s) {
            if self.breakpoints.contains(&(line as usize)) {
                let vars: Vec<String> = env
                    .frames
                    .iter()
                    .flat_map(|f| f.iter().map(|(k, v)| format!("{}={}", k, value_short(v))))
                    .collect();
                self.bp_hits.push((line as usize, vars.join(", ")));
            }
        }
    }

    pub fn load(&mut self, program: &ast::Program) {
        for item in &program.items {
            match item {
                Item::Fn(f) => {
                    if f.attributes.iter().any(|a| a == "test") && !self.test_fns.contains(&f.sig.name) {
                        self.test_fns.push(f.sig.name.clone());
                    }
                    self.fns.insert(f.sig.name.clone(), std::rc::Rc::new(f.clone()));
                }
                Item::Struct(s) => {
                    let fields: Vec<(String, Option<ast::Type>)> = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), Some(f.ty.clone())))
                        .collect();
                    self.structs.insert(s.name.clone(), fields);
                }
                Item::Enum(en) => {
                    let names: Vec<String> = en.variants.iter().map(|v| v.name.clone()).collect();
                    self.enums.insert(en.name.clone(), names.clone());
                    for v in &en.variants {
                        self.variants
                            .insert(v.name.clone(), (en.name.clone(), v.payload.len()));
                    }
                }
                Item::GlobalState(g) => {
                    let v = g.init.as_ref().map(|e| self.eval(&mut Env::new(), e)).unwrap_or(Ok(Value::Unit));
                    match v {
                        Ok(v) => {
                            self.globals.insert(g.name.clone(), v);
                        }
                        Err(e) => {
                            self.output.push_str(&format!("全局初始化错误: {}\n", err_text(&e)));
                        }
                    }
                }
                Item::Ui(u) => {
                    self.components.insert(u.name.clone());
                    self.ui_defs.insert(u.name.clone(), u.clone());
                }
                Item::Impl(i) => {
                    if let crate::ast::Type::Path(p) = &i.self_ty {
                        let ty_name = p.segments.join("::");
                        let methods: Vec<std::rc::Rc<ast::FnDef>> = i
                            .items
                            .iter()
                            .filter_map(|it| match it {
                                Item::Fn(f) => Some(std::rc::Rc::new(f.clone())),
                                _ => None,
                            })
                            .collect();
                        self.impls.entry(ty_name).or_default().extend(methods);
                    }
                }
                Item::Mod(m) => {
                    // flat registration of nested items (module execution)
                    for it in &m.items {
                        self.load_one(it);
                    }
                }
                _ => {}
            }
        }
    }

    fn load_one(&mut self, item: &Item) {
        let fake = ast::Program { items: vec![item.clone()], span: crate::token::Span::new(0, 0) };
        self.load(&fake);
    }

    /// Run fn main (or a named entry). Returns the program's exit value.
    pub fn run_main(&mut self) -> Result<Value, RtError> {
        if let Some(f) = self.fns.get("main").cloned() {
            self.call_fn(&f, &[])
        } else {
            Err(RtError::msg("未找到 main 函数"))
        }
    }

    /// #[test] 函数名列表（源码顺序，含模块内平铺注册的测试）
    pub fn test_names(&self) -> &[String] {
        &self.test_fns
    }

    /// 宿主侧注入全局变量（CLI 实参等）
    pub fn inject_global(&mut self, name: &str, v: Value) {
        self.globals.insert(name.to_string(), v);
    }

    pub fn call_named(&mut self, name: &str, args: Vec<Value>) -> Result<Value, RtError> {
        let f = self.fns.get(name).cloned().ok_or_else(|| RtError::msg(format!("未定义的函数 {}", name)))?;
        self.call_fn(&f, &args)
    }
    // ---- function calls ----

    fn call_fn(&mut self, f: &ast::FnDef, args: &[Value]) -> Result<Value, RtError> {
        self.call_depth += 1;
        if self.call_depth > 50000 {
            self.call_depth -= 1;
            return Err(RtError::msg(format!("函数调用深度超限(500): {}", f.sig.name)));
        }
        let prev = self.current_fn.clone();
        self.fn_stack.push(prev.clone());
        self.current_fn = f.sig.name.clone();
        let r = self.call_fn_inner(f, args);
        self.current_fn = prev;
        self.fn_stack.pop();
        self.call_depth -= 1;
        r
    }

    /// 当前函数调用栈（错误诊断用）
    fn fn_context(&self) -> String {
        let start = if self.fn_stack.len() > 3 { self.fn_stack.len() - 3 } else { 0 };
        let mut names: Vec<String> = Vec::new();
        for i in start..self.fn_stack.len() {
            if !self.fn_stack[i].is_empty() {
                names.push(self.fn_stack[i].clone());
            }
        }
        if !self.current_fn.is_empty() && names.last().map(|n| n != &self.current_fn).unwrap_or(true) {
            names.push(self.current_fn.clone());
        }
        names.join(" <- ")
    }

    fn call_fn_inner(&mut self, f: &ast::FnDef, args: &[Value]) -> Result<Value, RtError> {
        if f.is_extern {
            // FFI：extern 声明无函数体，符号由外部 C 提供 —— 仅原生运行时支持
            return Err(RtError::msg(format!(
                "extern 函数 {} 仅原生运行时支持（aine build + C 实现）",
                f.sig.name
            )));
        }
        let mut env = Env::new();
        for (i, p) in f.sig.params.iter().enumerate() {
            let v = args.get(i).cloned().unwrap_or(Value::Unit);
            env.define(&p.name, v);
        }
        let mut defers: Vec<Expr> = Vec::new();
        match self.exec_block(&mut env, &f.body, &mut defers) {
            Ok((cf, tail_val)) => {
                self.run_defers(&mut env, &mut defers)?;
                match cf {
                    ControlFlow::Return(v) => Ok(v),
                    _ => Ok(tail_val),
                }
            }
            // ? on an Err inside this function makes the function return Err
            Err(RtError::Propagate(v)) => Ok(Value::Result { ok: false, value: Box::new(v) }),
            // return executed inside a nested expression block
            Err(RtError::Return(v)) => Ok(v),
            Err(e) => Err(e),
        }
    }

    /// UI 运行时：执行组件的 render 块，控件调用求值为缩进树（终端渲染）。
    /// @state 读取走组件状态存储；事件回调（尾块闭包）不自动触发（由事件驱动）。
    fn render_component(&mut self, name: &str) -> String {
        let Some(u) = self.ui_defs.get(name).cloned() else {
            return format!("(未找到组件 {})", name);
        };
        let mut env = Env::new();
        // @state 初始化（组件状态存储）
        let mut state_frame: HashMap<String, Value> = HashMap::new();
        for st in &u.states {
            if let Some(init) = &st.init {
                let v = self.eval(&mut env, init).unwrap_or(Value::Unit);
                let n = st.pat.ident_name().unwrap_or_default();
                state_frame.insert(n.clone(), v.clone());
                env.define(&n, v);
            }
        }
        let mut out = String::new();
        if let Some(render) = &u.render {
            if std::env::var("AINE_UI_DEBUG").is_ok() {
                eprintln!("RENDER stmts={} first={:?}", render.stmts.len(), render.stmts.first());
            }
            self.render_block(&mut env, render, 0, &mut out, true);
        }
        // 状态摘要
        let mut keys: Vec<&String> = state_frame.keys().collect();
        keys.sort();
        let mut sv: Vec<String> = Vec::new();
        for k in keys {
            sv.push(format!("{}={}", k, value_short(state_frame.get(k).unwrap())));
        }
        if !sv.is_empty() {
            out.push_str(&format!("[state] {}\n", sv.join(", ")));
        }
        out
    }

    fn render_block(&mut self, env: &mut Env, b: &ast::Block, depth: usize, out: &mut String, evaluate: bool) {
        for st in &b.stmts {
            self.render_stmt(env, st, depth, out, evaluate);
        }
        // 块尾表达式（render 块的最后一句控件调用被解析为 tail）
        if let Some(t) = &b.tail {
            self.render_expr(env, t, depth, out, evaluate);
        }
    }

    fn render_stmt(&mut self, env: &mut Env, s: &Stmt, depth: usize, out: &mut String, evaluate: bool) {
        match s {
            Stmt::Expr(e) => {
                self.render_expr(env, e, depth, out, evaluate);
            }
            _ => {}
        }
    }

    /// 控件调用：Text(...) / Column { 子控件 }（尾块闭包 → 子控件求值）
    fn render_expr(&mut self, env: &mut Env, e: &Expr, depth: usize, out: &mut String, evaluate: bool) {
        if let Expr::Call { callee, args, .. } = e {
            let widget = match &**callee {
                Expr::Ident { name, .. } => name.clone(),
                _ => String::new(),
            };
            let widgets = ["Text", "Button", "TextInput", "List", "Row", "Column", "ListItem"];
            if widgets.contains(&widget.as_str()) {
                let pad = "  ".repeat(depth);
                // 参数摘要（跳过尾块闭包）
                let mut params: Vec<String> = Vec::new();
                let mut children: Option<ast::Block> = None;
                for a in args {
                    match &a.expr {
                        Expr::Closure { body, .. } => {
                            if let Expr::Block(b) = &**body {
                                children = Some(b.as_ref().clone());
                            }
                        }
                        other => {
                            let v = self.eval(env, other).unwrap_or(Value::Unit);
                            params.push(value_short(&v));
                        }
                    }
                }
                out.push_str(&format!("{}{}({})\n", pad, widget, params.join(", ")));
                if let Some(c) = children {
                    self.render_block(env, &c, depth + 1, out, false);
                }
                return;
            }
        }
        // 非控件表达式：仅顶层求值（副作用语句如赋值）；回调块内跳过（事件驱动）
        if evaluate {
            let _ = self.eval(env, e);
        }
    }

    fn run_defers(&mut self, env: &mut Env, defers: &mut Vec<Expr>) -> Result<(), RtError> {
        while let Some(e) = defers.pop() {
            self.eval(env, &e)?;
        }
        Ok(())
    }

    fn call_closure(&mut self, c: &Value, args: &[Value]) -> Result<Value, RtError> {
        if let Value::Closure { params, body, captured } = c {
            let mut env = Env { frames: captured.clone() };
            env.push();
            for (i, p) in params.iter().enumerate() {
                let v = args.get(i).cloned().unwrap_or(Value::Unit);
                env.define(p, v);
            }
            match self.exec_expr_stmt(&mut env, body) {
                Ok((ControlFlow::Return(v), _)) => Ok(v),
                Ok((_, v)) => Ok(v),
                Err(RtError::Return(v)) => Ok(v),
                Err(e) => Err(e),
            }
        } else {
            Err(RtError::msg("调用非闭包值"))
        }
    }

    /// 语句形/块尾 match 的求值: 臂体经块语义执行,
    /// break/continue/return 以 ControlFlow 上抛(不再被静默吞掉)
    fn eval_match_flow(&mut self, env: &mut Env, scrutinee: &Expr, arms: &[crate::ast::MatchArm], defers: &mut Vec<Expr>) -> Result<(ControlFlow, Value), RtError> {
        let sv = self.eval(env, scrutinee)?;
        for arm in arms {
            if self.pattern_matches(env, &arm.pat, &sv)? {
                env.push();
                let (cf, v) = match &arm.value {
                    Expr::Block(b) => self.exec_block(env, b, defers)?,
                    other => {
                        let v = self.eval(env, other)?;
                        (ControlFlow::Ok, v)
                    }
                };
                env.pop();
                return Ok((cf, v));
            }
        }
        Err(RtError::msg("match 未匹配任何分支"))
    }

    /// Execute an expression as a statement (handles return inside closures).
    fn exec_expr_stmt(&mut self, env: &mut Env, e: &Expr) -> Result<(ControlFlow, Value), RtError> {
        match e {
            Expr::Match { scrutinee, arms, .. } => {
                let mut no_defers: Vec<Expr> = Vec::new();
                return self.eval_match_flow(env, scrutinee, arms, &mut no_defers);
            }
            Expr::Block(b) => {
                let mut defers = Vec::new();
                env.push();
                let r = self.exec_block(env, b, &mut defers)?;
                env.pop();
                self.run_defers(env, &mut defers)?;
                Ok(r)
            }
            _ => {
                let v = self.eval(env, e)?;
                Ok((ControlFlow::Ok, v))
            }
        }
    }

    /// Execute a block's statements; returns the control flow and the
    /// value of the last expression (the block's value).
    fn exec_block(&mut self, env: &mut Env, b: &ast::Block, defers: &mut Vec<Expr>) -> Result<(ControlFlow, Value), RtError> {
        self.exec_depth += 1;
        if self.exec_depth > 50000 {
            self.exec_depth -= 1;
            return Err(RtError::msg(format!("执行块深度超限(2000)")));
        }
        let r = self.exec_block_inner(env, b, defers);
        self.exec_depth -= 1;
        return r;
    }

    fn exec_block_inner(&mut self, env: &mut Env, b: &ast::Block, defers: &mut Vec<Expr>) -> Result<(ControlFlow, Value), RtError> {
        for s in &b.stmts {
            let cf = self.exec_stmt(env, s, defers)?;
            match cf {
                ControlFlow::Ok => {}
                other => return Ok((other, Value::Unit)),
            }
        }
        let tail_val = if let Some(tail) = &b.tail {
            // 块尾 match: break/continue 需上抛(不能走普通 eval)
            if let Expr::Match { scrutinee, arms, .. } = &**tail {
                let (cf2, v2) = self.eval_match_flow(env, scrutinee, arms, defers)?;
                match cf2 {
                    ControlFlow::Ok => v2,
                    other => return Ok((other, Value::Unit)),
                }
            } else {
                match self.eval(env, tail) {
                    Err(RtError::Return(v)) => {
                        let rv = v.clone();
                        return Ok((ControlFlow::Return(v), rv));
                    }
                    other => other?,
                }
            }
        } else {
            Value::Unit
        };
        Ok((ControlFlow::Ok, tail_val))
    }

    fn exec_stmt(&mut self, env: &mut Env, s: &Stmt, defers: &mut Vec<Expr>) -> Result<ControlFlow, RtError> {
        self.check_breakpoint(s, env);
        match s {
            Stmt::Let(ls) => {
                let v = ls.init.as_ref().map(|e| self.eval(env, e)).transpose()?.unwrap_or(Value::Unit);
                if let Some(name) = ls.pat.ident_name() {
                    env.define(&name, v);
                }
                Ok(ControlFlow::Ok)
            }
            Stmt::Expr(e) => {
                // 语句形 match: 臂体经 exec_block 求值, break/continue 需要
                // 向上传播(普通 eval 只返回 Value, 会静默吞掉循环控制流)
                if let Expr::Match { scrutinee, arms, .. } = &**e {
                        let sv = self.eval(env, scrutinee)?;
                    for arm in arms {
                        if self.pattern_matches(env, &arm.pat, &sv)? {
                            env.push();
                            let (cf, _v) = match &arm.value {
                                Expr::Block(b) => self.exec_block(env, b, defers)?,
                                other => {
                                    let v = self.eval(env, other)?;
                                    (ControlFlow::Ok, v)
                                }
                            };
                            env.pop();
                            return Ok(cf);
                        }
                    }
                    return Err(RtError::msg("match 未匹配任何分支"));
                }
                match self.eval(env, e) {
                    Err(RtError::Return(v)) => Ok(ControlFlow::Return(v)),
                    other => {
                        other?;
                        Ok(ControlFlow::Ok)
                    }
                }
            }
            Stmt::Return(v) => {
                let val = v.as_ref().map(|e| self.eval(env, e)).transpose()?.unwrap_or(Value::Unit);
                Ok(ControlFlow::Return(val))
            }
            Stmt::If(ifexpr) => {
                let cond = self.eval(env, &ifexpr.cond)?;
                if self.truthy(&cond) {
                    env.push();
                    let (cf, _) = self.exec_block(env, &ifexpr.then_branch, defers)?;
                    env.pop();
                    return Ok(cf);
                } else if let Some(e) = &ifexpr.else_branch {
                    match &**e {
                        Expr::Block(b) => {
                            env.push();
                            let (cf, _) = self.exec_block(env, b, defers)?;
                            env.pop();
                            return Ok(cf);
                        }
                        Expr::If(nested) => {
                            let inner = Stmt::If(Box::new((**nested).clone()));
                            return self.exec_stmt(env, &inner, defers);
                        }
                        other => {
                            self.eval(env, other)?;
                        }
                    }
                }
                Ok(ControlFlow::Ok)
            }
            Stmt::While { cond, body, .. } => {
                loop {
                    let c = self.eval(env, cond)?;
                    if !self.truthy(&c) {
                        break;
                    }
                    env.push();
                    let (cf, _) = self.exec_block(env, body, defers)?;
                    env.pop();
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(_) => return Ok(cf),
                        ControlFlow::Ok => {}
                    }
                }
                Ok(ControlFlow::Ok)
            }
            Stmt::For { pat, iter, body, .. } => {
                let it = self.eval(env, iter)?;
                let items: Vec<Value> = match &it {
                    Value::Vec(items) => items.iter().cloned().collect(),
                    Value::Range { start, end } => (*start..*end).map(|i| Value::Int(i)).collect(),
                    Value::Str(s) => s.chars().map(|c| Value::Str(c.to_string().into())).collect(),
                    _ => Vec::new(),
                };
                for item in items {
                    env.push();
                    if let Some(name) = pat.ident_name() {
                        env.define(&name, item);
                    }
                    let (cf, _) = self.exec_block(env, body, defers)?;
                    env.pop();
                    match cf {
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(_) => return Ok(cf),
                        ControlFlow::Ok => {}
                    }
                }
                Ok(ControlFlow::Ok)
            }
            Stmt::Break(_) => Ok(ControlFlow::Break),
            Stmt::Continue(_) => Ok(ControlFlow::Continue),
            Stmt::Defer(e) => {
                defers.push((**e).clone());
                Ok(ControlFlow::Ok)
            }
            Stmt::TryCatch(tc) => {
                env.push();
                let mut d = Vec::new();
                let try_cf = self.exec_block(env, &tc.try_block, &mut d).map(|(cf, _)| cf);
                env.pop();
                self.run_defers(env, &mut d).ok();
                match try_cf {
                    Ok(cf) => Ok(cf),
                    Err(err) => {
                        if let Some(p) = &tc.catch_param {
                            env.push();
                            let err_val = err_value(&err);
                            env.define(&p.name, err_val);
                            let (cf, _) = self.exec_block(env, &tc.catch_block, defers)?;
                            env.pop();
                            Ok(cf)
                        } else {
                            Err(err)
                        }
                    }
                }
            }
            Stmt::Block(b) => {
                env.push();
                let (cf, _) = self.exec_block(env, b, defers)?;
                env.pop();
                Ok(cf)
            }
        }
    }

    fn truthy(&self, v: &Value) -> bool {
        match v {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Option { some, .. } => *some,
            Value::Result { ok, .. } => *ok,
            _ => true,
        }
    }
    // ---- expressions ----

    fn eval(&mut self, env: &mut Env, e: &Expr) -> Result<Value, RtError> {
        self.eval_depth += 1;
        if self.eval_depth > 100000 {
            self.eval_depth -= 1;
            return Err(RtError::msg(format!("解释器递归深度超限(3000)")));
        }
        let r = self.eval_inner(env, e);
        self.eval_depth -= 1;
        return r;
    }

    fn eval_inner(&mut self, env: &mut Env, e: &Expr) -> Result<Value, RtError> {
        match e {
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Str(s) => Ok(Value::Str(s.clone().into())),
            Expr::FmtStr(s) => self.eval_fmt(env, s),
            Expr::Ident { name, .. } => {
                // builtin None value
                if name == "None" {
                    return Ok(Value::Option { some: false, value: Box::new(Value::Unit) });
                }
                // zero-arity enum variant as a value (e.g. JNull)
                if let Some((ty, arity)) = self.variants.get(name).cloned() {
                    if arity == 0 {
                        return Ok(Value::Variant { ty, name: name.clone(), payload: Vec::new() });
                    }
                }
                if name == "Map" {
                    return Ok(Value::MapType);
                }
                if name == "db" || name == "http" || self.structs.contains_key(name) {
                    return Ok(Value::Nil);
                }
                if self.components.contains(name) {
                    // 组件名作值：以名字字符串承载（run_ui(Counter) 用）
                    return Ok(Value::Str(name.clone().into()));
                }
                if let Some(v) = env.get(name) {
                    Ok(v)
                } else if let Some(v) = self.globals.get(name) {
                    Ok(v.clone())
                } else if let Some(f) = self.fns.get(name) {
                    Ok(Value::Closure {
                        params: f.sig.params.iter().map(|p| p.name.clone()).collect(),
                        body: Box::new(Expr::Block(Box::new(f.body.clone()))),
                        captured: env.snapshot(),
                    })
                } else {
                    Err(RtError::msg(format!("未定义的名称 '{}'", name)))
                }
            }
            Expr::Path(p) => {
                Err(RtError::msg(format!("未定义的值 '{}'", p.segments.join("::"))))
            }
            Expr::Array(items) => {
                let mut vals = Vec::new();
                for i in items {
                    vals.push(self.eval(env, i)?);
                }
                Ok(Value::Vec(vals.into()))
            }
            Expr::Tuple(items) => {
                let mut vals = Vec::new();
                for i in items {
                    vals.push(self.eval(env, i)?);
                }
                if vals.len() == 1 {
                    Ok(vals.remove(0))
                } else {
                    Ok(Value::Tuple(vals))
                }
            }
            Expr::StructLit { name, fields } => {
                let mut vals = Vec::new();
                for (fname, fexpr) in fields {
                    vals.push((fname.clone(), self.eval(env, fexpr)?));
                }
                Ok(Value::Struct { ty: name.segments.join("::"), fields: vals })
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(env, expr)?;
                match op {
                    crate::ast::UnOp::Neg => match v {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(RtError::msg("一元负号要求数值")),
                    },
                    crate::ast::UnOp::Not => match v {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err(RtError::msg("! 要求 bool")),
                    },
                    _ => Ok(v),
                }
            }
            Expr::Binary { op, lhs, rhs } => self.eval_binary(op, lhs, rhs, env),
            Expr::Assign { op, target, value } => {
                let v = self.eval(env, value)?;
                let name = match &**target {
                    Expr::Ident { name, .. } => name.clone(),
                    // v[i] = x / m[k] = v：值语义下的原地写入（重建副本后重绑定）
                    Expr::Index { base, index } => {
                        let name = match &**base {
                            Expr::Ident { name, .. } => name.clone(),
                            _ => {
                                return Err(RtError::msg(
                                    "索引赋值目标必须是变量（let mut 声明），不支持嵌套路径",
                                ))
                            }
                        };
                        let iv = self.eval(env, index)?;
                        let idx = int_of(&iv);
                        let cur = env
                            .get(&name)
                            .ok_or_else(|| RtError::msg(format!("未定义的变量 '{}'", name)))?;
                        match cur {
                            Value::Vec(items) => {
                                if idx < 0 || idx as usize >= items.len() {
                                    return Err(RtError::msg(format!(
                                        "索引越界: idx={} len={} 于函数 {}",
                                        idx,
                                        items.len(),
                                        self.fn_context()
                                    )));
                                }
                                let mut new_items = (*items).clone();
                                match op {
                                    crate::ast::AssignOp::Assign => new_items[idx as usize] = v.clone(),
                                    _ => {
                                        let cur_e = new_items[idx as usize].clone();
                                        let r = self.arith(op, &cur_e, &v)?;
                                        new_items[idx as usize] = r;
                                    }
                                }
                                let newv = Value::Vec(std::sync::Arc::new(new_items));
                                env.assign(&name, newv.clone());
                                return Ok(v);
                            }
                            Value::Map(entries) => {
                                let key = match &iv {
                                    Value::Str(s) => s.to_string(),
                                    other => {
                                        return Err(RtError::msg(format!(
                                            "Map 键必须是 String，实际是 {}",
                                            other.display()
                                        )))
                                    }
                                };
                                let val = match op {
                                    crate::ast::AssignOp::Assign => v.clone(),
                                    _ => {
                                        let cur_e = entries
                                            .iter()
                                            .find(|(k, _)| *k == key)
                                            .map(|(_, val)| val.clone())
                                            .unwrap_or(Value::Int(0));
                                        self.arith(op, &cur_e, &v)?
                                    }
                                };
                                let mut new_entries = entries.clone();
                                match new_entries.iter_mut().find(|(k, _)| *k == key) {
                                    Some(slot) => slot.1 = val,
                                    None => new_entries.push((key, val)),
                                }
                                let newv = Value::Map(new_entries);
                                env.assign(&name, newv.clone());
                                return Ok(v);
                            }
                            Value::Str(_) => {
                                return Err(RtError::msg(
                                    "字符串是整体值，不支持局部写入；请用切片拼接构造新串（s[0..i] + x + s[i+1..]）",
                                ))
                            }
                            other => {
                                return Err(RtError::msg(format!(
                                    "该值不支持索引赋值: {}",
                                    other.display()
                                )))
                            }
                        }
                    }
                    _ => return Err(RtError::msg("赋值目标必须是变量")),
                };
                let current = env.get(&name);
                match op {
                    crate::ast::AssignOp::Assign => {
                        env.assign(&name, v.clone());
                        Ok(v)
                    }
                    _ => {
                        let cur = current.unwrap_or(Value::Int(0));
                        let result = self.arith(op, &cur, &v)?;
                        env.assign(&name, result.clone());
                        Ok(result)
                    }
                }
            }
            Expr::Call { callee, args, .. } => self.eval_call(env, callee, args),
            Expr::Field { base, field, .. } => {
                let b = self.eval(env, base)?;
                match b {
                    Value::Struct { ty, fields } => {
                        let names: String = fields.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ");
                        fields
                            .into_iter()
                            .find(|(n, _)| n == field)
                            .map(|(_, v)| v)
                            .ok_or_else(|| {
                                RtError::msg(format!(
                                    "结构体 '{}' 没有字段 '{}'（现有字段: {}）",
                                    ty, field, names
                                ))
                            })
                    }
                    other => Err(RtError::msg(format!(
                        "类型 {} 的值没有字段 '{}'",
                        other.type_name(),
                        field
                    ))),
                }
            }
            Expr::Index { base, index } => {
                let b = self.eval(env, base)?;
                let i = self.eval(env, index)?;
                if let Value::Range { start, end } = i {
                    let (s, e) = (start.max(0) as usize, end.max(0) as usize);
                    return match b {
                        Value::Str(sv) => {
                            let chars = self.chars_of(&sv);
                            if e > chars.len() {
                                return Err(RtError::msg("字符串切片越界"));
                            }
                            let sub: String = chars[s.min(chars.len())..e].iter().collect();
                            Ok(Value::Str(sub.into()))
                        }
                        Value::Vec(items) => {
                            if e > items.len() {
                                return Err(RtError::msg("切片越界"));
                            }
                            Ok(Value::Vec(items[s.min(items.len())..e].to_vec().into()))
                        }
                        _ => Err(RtError::msg("该值不可切片")),
                    };
                }
                // m["key"]：String 索引仅用于 Map（与 m["k"] = v 写入对称）
                if let Value::Str(key) = i {
                    return match b {
                        Value::Map(entries) => entries
                            .iter()
                            .find(|(k, _)| k.as_str() == key.as_ref())
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| {
                                RtError::msg(format!("Map 中不存在键 '{}'（于函数 {}）", key, self.fn_context()))
                            }),
                        _ => Err(RtError::msg("String 索引仅用于 Map；Vec/字符串索引必须是整数")),
                    };
                }
                let idx = match i {
                    Value::Int(i) => i as usize,
                    _ => return Err(RtError::msg("索引必须是整数")),
                };
                match b {
                    Value::Vec(items) => items
                        .get(idx)
                        .cloned()
                        .ok_or_else(|| {
                            let bdisp: String = items.iter().map(|v| v.display()).collect::<Vec<_>>().join(", ");
                            let bshow: String = if bdisp.len() > 120 { bdisp[..120].to_string() } else { bdisp };
                            // 用户面向：默认只给 idx/len/内容/函数栈；宿主机栈帧仅在
                            // FLOWC_RT_BACKTRACE=1 时追加（调试解释器自身才需要）
                            let mut msg = format!("索引越界: idx={} len={} vec=[{}] 于函数 {}",
                                idx, items.len(), bshow, self.fn_context());
                            if std::env::var("FLOWC_RT_BACKTRACE").as_deref() == Ok("1") {
                                msg.push_str(&format!("\n{}", std::backtrace::Backtrace::force_capture()));
                            }
                            RtError::msg(msg)
                        }),
                    Value::Str(s) => {
                        let chars = self.chars_of(&s);
                        if let Some(c) = chars.get(idx) {
                            Ok(Value::Str(c.to_string().into()))
                        } else {
                            Err(RtError::msg("字符串索引越界"))
                        }
                    }
                    _ => Err(RtError::msg("该值不可索引")),
                }
            }
            Expr::Try(e) => {
                let v = self.eval(env, e)?;
                match v {
                    Value::Result { ok: true, value } => Ok(*value),
                    Value::Result { ok: false, value } => Err(RtError::Propagate(*value)),
                    Value::Option { some: true, value } => Ok(*value),
                    Value::Option { some: false, .. } => {
                        Err(RtError::Propagate(Value::Option { some: false, value: Box::new(Value::Unit) }))
                    }
                    other => Ok(other),
                }
            }
            Expr::Range { start, end } => {
                let s = start.as_ref().map(|e| self.eval(env, e)).transpose()?.map(|v| int_of(&v)).unwrap_or(0);
                let en = end.as_ref().map(|e| self.eval(env, e)).transpose()?.map(|v| int_of(&v)).unwrap_or(0);
                Ok(Value::Range { start: s, end: en })
            }
            Expr::Block(b) => {
                env.push();
                let mut defers = Vec::new();
                let (cf, tail_val) = self.exec_block(env, b, &mut defers)?;
                env.pop();
                self.run_defers(env, &mut defers)?;
                match cf {
                    ControlFlow::Return(v) => Err(RtError::Return(v)),
                    _ => Ok(tail_val),
                }
            }
            Expr::If(ifexpr) => {
                let cond = self.eval(env, &ifexpr.cond)?;
                if self.truthy(&cond) {
                    env.push();
                    let mut d = Vec::new();
                    let r = self.exec_block(env, &ifexpr.then_branch, &mut d);
                    env.pop();
                    let _ = self.run_defers(env, &mut d);
                    match r {
                        Ok((ControlFlow::Return(v), _)) => Err(RtError::Return(v)),
                        Ok((_, v)) => Ok(v),
                        Err(e) => Err(e),
                    }
                } else if let Some(e2) = &ifexpr.else_branch {
                    self.eval(env, e2)
                } else {
                    Ok(Value::Unit)
                }
            }
            Expr::Match { scrutinee, arms, .. } => {
                let sv = self.eval(env, scrutinee)?;
                for arm in arms {
                    if self.pattern_matches(env, &arm.pat, &sv)? {
                        env.push();
                        let r = self.eval(env, &arm.value);
                        env.pop();
                        return r;
                    }
                }
                Err(RtError::msg("match 未匹配任何分支"))
            }
            Expr::Closure { params, body, .. } => Ok(Value::Closure {
                params: params.iter().map(|p| p.name.clone()).collect(),
                body: body.clone(),
                captured: env.snapshot(),
            }),
            Expr::Go { block, .. } => {
                env.push();
                let mut d = Vec::new();
                let r = self.exec_block(env, block, &mut d);
                env.pop();
                let _ = self.run_defers(env, &mut d);
                let _ = r;
                Ok(Value::Unit)
            }
            Expr::UiOp { block, .. } => {
                env.push();
                let mut d = Vec::new();
                let r = self.exec_block(env, block, &mut d);
                env.pop();
                let _ = self.run_defers(env, &mut d);
                let _ = r;
                Ok(Value::Unit)
            }
            Expr::Paren(inner) => self.eval(env, inner),
        }
    }

    fn pattern_matches(&mut self, env: &mut Env, pat: &crate::ast::Pattern, v: &Value) -> Result<bool, RtError> {
        match pat {
            crate::ast::Pattern::Ident { name, .. } => {
                if name == "_" {
                    Ok(true)
                } else if name == "None" {
                    // builtin unit variant (Option)
                    Ok(matches!(v, Value::Option { some: false, .. }))
                } else if let Some((_, arity)) = self.variants.get(name) {
                    // unit-variant pattern: JNull (no parens) matches the
                    // variant value, NOT a variable binding
                    if *arity == 0 {
                        Ok(matches!(
                            v,
                            Value::Variant { name: vn, payload, .. }
                                if vn == name && payload.is_empty()
                        ))
                    } else {
                        Ok(false)
                    }
                } else {
                    env.define(name, v.clone());
                    Ok(true)
                }
            }
            crate::ast::Pattern::Wildcard => Ok(true),
            crate::ast::Pattern::Lit(e) => {
                let lv = self.eval(env, e)?;
                Ok(lv == *v)
            }
            crate::ast::Pattern::Path(_) => Ok(false),
            crate::ast::Pattern::Variant { name, fields } => {
                // builtin option/result variant patterns
                if name == "Some" {
                    if let Value::Option { some: true, value } = v {
                        if fields.len() == 1 {
                            return self.pattern_matches(env, &fields[0], value);
                        }
                    }
                    return Ok(false);
                }
                if name == "None" {
                    return Ok(matches!(v, Value::Option { some: false, .. }));
                }
                if name == "Ok" {
                    if let Value::Result { ok: true, value } = v {
                        if fields.len() == 1 {
                            return self.pattern_matches(env, &fields[0], value);
                        }
                    }
                    return Ok(false);
                }
                if name == "Err" {
                    if let Value::Result { ok: false, value } = v {
                        if fields.len() == 1 {
                            return self.pattern_matches(env, &fields[0], value);
                        }
                    }
                    return Ok(false);
                }
                if let Value::Variant { name: vname, payload, .. } = v {
                    if name != vname {
                        return Ok(false);
                    }
                    if fields.len() != payload.len() {
                        return Ok(false);
                    }
                    for (p, pv) in fields.iter().zip(payload.iter()) {
                        if !self.pattern_matches(env, p, pv)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else if let Value::Struct { ty: sty, fields: sfields } = v {
                    // 结构体模式: AEntry(k, v) — 按字段顺序绑定
                    if name != sty {
                        return Ok(false);
                    }
                    if fields.len() != sfields.len() {
                        return Ok(false);
                    }
                    for (p, (_, fv)) in fields.iter().zip(sfields.iter()) {
                        if !self.pattern_matches(env, p, fv)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
    }
    // ---- binary ops ----

    fn eval_binary(&mut self, op: &crate::ast::BinOp, lhs: &Expr, rhs: &Expr, env: &mut Env) -> Result<Value, RtError> {
        use crate::ast::BinOp;
        match op {
            BinOp::And => {
                let l = self.eval(env, lhs)?;
                if !self.truthy(&l) {
                    return Ok(Value::Bool(false));
                }
                let r = self.eval(env, rhs)?;
                Ok(Value::Bool(self.truthy(&r)))
            }
            BinOp::Or => {
                let l = self.eval(env, lhs)?;
                if self.truthy(&l) {
                    return Ok(Value::Bool(true));
                }
                let r = self.eval(env, rhs)?;
                Ok(Value::Bool(self.truthy(&r)))
            }
            BinOp::Eq | BinOp::Ne => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                let eq = l == r;
                Ok(Value::Bool(if matches!(op, BinOp::Eq) { eq } else { !eq }))
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                self.compare(op, &l, &r)
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                if matches!(op, BinOp::Add) {
                    if let (Value::Str(a), Value::Str(b)) = (&l, &r) {
                        return Ok(Value::Str(format!("{}{}", a, b).into()));
                    }
                }
                self.arith_op(op, &l, &r)
            }
            BinOp::Range => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                Ok(Value::Range { start: int_of(&l), end: int_of(&r) })
            }
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                let l = self.eval(env, lhs)?;
                let r = self.eval(env, rhs)?;
                let (a, b) = (int_of(&l), int_of(&r));
                let v = match op {
                    BinOp::BitAnd => a & b,
                    BinOp::BitOr => a | b,
                    BinOp::BitXor => a ^ b,
                    BinOp::Shl => a << b,
                    BinOp::Shr => a >> b,
                    _ => 0,
                };
                Ok(Value::Int(v))
            }
        }
    }

    fn arith(&mut self, op: &crate::ast::AssignOp, a: &Value, b: &Value) -> Result<Value, RtError> {
        use crate::ast::AssignOp;
        let binop = match op {
            AssignOp::AddAssign => crate::ast::BinOp::Add,
            AssignOp::SubAssign => crate::ast::BinOp::Sub,
            AssignOp::MulAssign => crate::ast::BinOp::Mul,
            AssignOp::DivAssign => crate::ast::BinOp::Div,
            AssignOp::RemAssign => crate::ast::BinOp::Rem,
            AssignOp::Assign => return Ok(b.clone()),
        };
        self.arith_op(&binop, a, b)
    }

    fn arith_op(&mut self, op: &crate::ast::BinOp, a: &Value, b: &Value) -> Result<Value, RtError> {
        use crate::ast::BinOp;
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => {
                let v = match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => {
                        if *y == 0 {
                            return Err(RtError::msg("除零错误"));
                        }
                        x / y
                    }
                    BinOp::Rem => {
                        if *y == 0 {
                            return Err(RtError::msg("除零错误"));
                        }
                        x % y
                    }
                    _ => return Err(RtError::msg("非法算术运算")),
                };
                Ok(Value::Int(v))
            }
            (Value::Float(_), Value::Float(_)) | (Value::Float(_), Value::Int(_)) | (Value::Int(_), Value::Float(_)) => {
                let (x, y) = (num_as_f64(a), num_as_f64(b));
                let v = match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => x / y,
                    BinOp::Rem => x % y,
                    _ => return Err(RtError::msg("非法算术运算")),
                };
                Ok(Value::Float(v))
            }
            _ => Err(RtError::msg("算术运算要求数值")),
        }
    }

    fn compare(&mut self, op: &crate::ast::BinOp, a: &Value, b: &Value) -> Result<Value, RtError> {
        use crate::ast::BinOp;
        if let (Value::Str(x), Value::Str(y)) = (a, b) {
            let v = match op {
                BinOp::Lt => x < y,
                BinOp::Le => x <= y,
                BinOp::Gt => x > y,
                BinOp::Ge => x >= y,
                _ => return Err(RtError::msg("非法比较")),
            };
            return Ok(Value::Bool(v));
        }
        let (x, y) = (num_as_f64(a), num_as_f64(b));
        let v = match op {
            BinOp::Lt => x < y,
            BinOp::Le => x <= y,
            BinOp::Gt => x > y,
            BinOp::Ge => x >= y,
            _ => return Err(RtError::msg("非法比较")),
        };
        Ok(Value::Bool(v))
    }

    // ---- calls ----

    /// memoized char vector for a string, keyed by Arc allocation address
    /// (O(1) index/slice on repeated access to the same string value)
    fn chars_of(&self, s: &std::sync::Arc<str>) -> std::sync::Arc<Vec<char>> {
        let key = std::sync::Arc::as_ptr(s) as *const u8 as usize;
        if let Some((stored, chars)) = self.char_cache.borrow().get(&key) {
            if std::sync::Arc::ptr_eq(stored, s) {
                return chars.clone();
            }
        }
        let v = std::sync::Arc::new(s.chars().collect::<Vec<char>>());
        self.char_cache.borrow_mut().insert(key, (s.clone(), v.clone()));
        v
    }

    fn eval_call(&mut self, env: &mut Env, callee: &Expr, args: &[crate::ast::CallArg]) -> Result<Value, RtError> {
        let mut arg_vals = Vec::new();
        for (ai, a) in args.iter().enumerate() {
            arg_vals.push(self.eval(env, &a.expr).map_err(|e| {
                RtError::msg(format!("[在调用 {} 的第 {} 个实参中] {}", match callee {
                    Expr::Ident { name, .. } => name.clone(),
                    Expr::Field { base, field, .. } => {
                        let bc = match &**base {
                            Expr::Ident { name, .. } => name.clone(),
                            _ => "?".to_string(),
                        };
                        format!("{}.{}", bc, field)
                    }
                    _ => "?".to_string(),
                }, ai + 1, crate::interp::rt_error_text(&e)))
            })?);
        }
        match callee {
            Expr::Ident { name, .. } => self.call_named_value(name, &arg_vals, env),
            Expr::Field { base, field, .. } => {
                let receiver_name = match &**base {
                    Expr::Ident { name, .. } => Some(name.clone()),
                    _ => None,
                };
                // db 内建模块（文件 JSONL 存储）——仅解释器运行（原生需 C 扩展）
                if receiver_name.as_deref() == Some("db") {
                    let r = self.call_db_method(field, &arg_vals)?;
                    return Ok(r);
                }
                // http 内建模块（std::net GET）——仅解释器运行（原生需 C 扩展）
                if receiver_name.as_deref() == Some("http") {
                    let r = self.call_http_method(field, &arg_vals)?;
                    return Ok(r);
                }
                // COW 快路径：v.push(x)/m.set(k,v) 在变量槽是缓冲区唯一持有者时原地修改
                //（O(1) 摊还）；存在别名时 get_mut 失败回退深拷贝路径——值语义不变。
                if let Expr::Ident { name, .. } = &**base {
                    if is_mutating_method(field) {
                        if let Some(r) = self.try_mutate_in_place(env, name, field, &arg_vals)? {
                            return Ok(r);
                        }
                    }
                }
                let base_val = self.eval(env, base)?;
                let result = self.call_method(&base_val, field, &arg_vals)?;
                if let Some(name) = receiver_name {
                    let mutable = is_mutating_method(field)
                        && (matches!(base_val, Value::Vec(_)) || matches!(base_val, Value::Map(_)));
                    if mutable {
                        env.assign(&name, result.clone());
                    }
                }
                Ok(result)
            }
            Expr::Closure { .. } => {
                let c = self.eval(env, callee)?;
                self.call_closure(&c, &arg_vals)
            }
            other => {
                let c = self.eval(env, other)?;
                self.call_closure(&c, &arg_vals)
            }
        }
    }

    fn call_named_value(&mut self, name: &str, args: &[Value], env: &mut Env) -> Result<Value, RtError> {
        match name {
            "print" => {
                let text: Vec<String> = args.iter().map(|v| v.display()).collect();
                let line = text.join(" ");
                self.output.push_str(&line);
                self.output.push('\n');
                println!("{}", line);
                Ok(Value::Unit)
            }
            "now" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                Ok(Value::Int(secs))
            }
            // Std API (冻结, docs/STD_API_SPEC.md): 逐行读 stdin; 注入缓冲优先
            "read_line" => {
                if let Some(line) = self.input_lines.pop_front() {
                    return Ok(Value::Str(line.into()));
                }
                use std::io::BufRead;
                let mut buf = String::new();
                let n = std::io::stdin().lock().read_line(&mut buf).unwrap_or(0);
                if n == 0 {
                    return Ok(Value::Str(String::new().into()));
                }
                while buf.ends_with('\n') || buf.ends_with('\r') {
                    buf.pop();
                }
                Ok(Value::Str(buf.into()))
            }
            "env" => {
                let name = args.first().map(|v| v.display()).unwrap_or_default();
                Ok(Value::Str(std::env::var(&name).unwrap_or_default().into()))
            }
            // 目录条目列表(仅名字, 不含子目录路径): file_list(dir) -> Vec<String>
            "file_list" => {
                let dir = args.first().map(|v| v.display()).unwrap_or_default();
                let mut out: Vec<Value> = Vec::new();
                match std::fs::read_dir(&dir) {
                    Ok(rd) => {
                        let mut names: Vec<String> = rd
                            .filter_map(|e| e.ok())
                            .map(|e| e.file_name().to_string_lossy().to_string())
                            .collect();
                        names.sort();
                        for n in names {
                            out.push(Value::Str(n.into()));
                        }
                    }
                    Err(_) => {}
                }
                Ok(Value::Vec(std::sync::Arc::new(out)))
            }
            "sleep" => {
                let ms = match args.first() {
                    Some(Value::Int(i)) => *i,
                    _ => 0,
                };
                std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
                Ok(Value::Unit)
            }
            "window_new" | "label_new" | "button_new" | "set_text" | "get_text" | "button_onclick" | "ui_run" | "input_new" | "listbox_new" | "listbox_add" | "listbox_clear" | "listbox_selected" => {
                Err(RtError::msg("UI 内建仅原生构建支持(aine build); 解释执行请使用编译产物"))
            }
            "json_encode" => {
                let v = args.first().cloned().unwrap_or(Value::Unit);
                match crate::json::encode(&v) {
                    Ok(s) => Ok(Value::Str(s.into())),
                    Err(e) => Err(RtError::msg(e)),
                }
            }
            "json_decode" => {
                let s = args.first().map(|v| v.display()).unwrap_or_default();
                match crate::json::decode(&s) {
                    Ok(v) => Ok(v),
                    Err(e) => Err(RtError::msg(e)),
                }
            }
            // http_request(method, url, headers: Map, body, on_chunk) -> i32
            // 逐行回调 on_chunk(闭包或函数名); 错误返回 0
            "http_request" => {
                let method = args.first().map(|v| v.display()).unwrap_or_default();
                let url = args.get(1).map(|v| v.display()).unwrap_or_default();
                let mut headers: Vec<(String, String)> = Vec::new();
                if let Some(Value::Map(entries)) = args.get(2) {
                    for (k, v) in entries {
                        headers.push((k.clone(), v.display()));
                    }
                }
                let body = args.get(3).map(|v| v.display()).unwrap_or_default();
                let cb = args.get(4).cloned();
                let mut resp = match crate::http::open(&method, &url, &headers, &body) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("{}", e);
                        return Ok(Value::Int(0));
                    }
                };
                let status = resp.status;
                use std::io::BufRead;
                loop {
                    let mut line = String::new();
                    let n = match resp.reader.read_line(&mut line) {
                        Ok(n) => n,
                        Err(_) => break,
                    };
                    if n == 0 {
                        break;
                    }
                    while line.ends_with('\n') || line.ends_with('\r') {
                        line.pop();
                    }
                    if let Some(c) = &cb {
                        let arg = Value::Str(line.clone().into());
                        match c {
                            Value::Closure { .. } => {
                                if let Err(e) = self.call_closure(c, &[arg]) {
                                    eprintln!("{}", crate::interp::rt_error_text(&e));
                                    break;
                                }
                            }
                            Value::Str(fname) => {
                                if let Some(f) = self.fns.get(&**fname).cloned() {
                                    if let Err(e) = self.call_fn(&f, &[arg]) {
                                        eprintln!("{}", crate::interp::rt_error_text(&e));
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Value::Int(status as i64))
            }
            // 断言内建：失败即 panic（确定性、不可捕获），与索引越界同一资格模型
            "assert" => {
                let cond_ok = match args.first() {
                    Some(Value::Bool(b)) => *b,
                    Some(other) => {
                        return Err(RtError::msg(format!(
                            "assert 第一个参数必须是 bool，实际是 {}（于函数 {}）",
                            other.display(),
                            self.fn_context()
                        )))
                    }
                    None => {
                        return Err(RtError::msg("assert 需要参数：assert(cond) 或 assert(cond, msg)"))
                    }
                };
                if cond_ok {
                    Ok(Value::Unit)
                } else {
                    let msg = args
                        .get(1)
                        .map(|v| v.display())
                        .unwrap_or_else(|| "条件为假".to_string());
                    Err(RtError::msg(format!("断言失败: {} （于函数 {}）", msg, self.fn_context())))
                }
            }
            "read_file" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                // 重试：Windows 杀软/索引器瞬时锁可致偶发读取失败
                let mut result: Option<Result<String, std::io::Error>> = None;
                for attempt in 0..5 {
                    match std::fs::read_to_string(&path) {
                        Ok(s) => {
                            result = Some(Ok(s));
                            break;
                        }
                        Err(e) => {
                            result = Some(Err(e));
                            if attempt < 4 {
                                std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
                            }
                        }
                    }
                }
                match result.unwrap() {
                    Ok(s) => Ok(Value::Str(s.into())),
                    Err(e) => Ok(Value::Str(format!("[read_file error: {}]", e).into())),
                }
            }
            // std/io 内建（M6-STD-2）：真实文件写入
            "write_file" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                let content = args.get(1).map(|v| v.display()).unwrap_or_default();
                match std::fs::write(&path, content) {
                    Ok(_) => Ok(Value::Bool(true)),
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            "append_file" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                let content = args.get(1).map(|v| v.display()).unwrap_or_default();
                use std::io::Write;
                match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                    Ok(mut f) => match f.write_all(content.as_bytes()) {
                        Ok(_) => Ok(Value::Bool(true)),
                        Err(_) => Ok(Value::Bool(false)),
                    },
                    Err(_) => Ok(Value::Bool(false)),
                }
            }
            "file_exists" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                Ok(Value::Bool(std::path::Path::new(&path).exists()))
            }
            "read_bytes" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        let items: Vec<Value> = bytes.iter().map(|b| Value::Int(*b as i64)).collect();
                        Ok(Value::Vec(items.into()))
                    }
                    Err(e) => Ok(Value::Str(format!("[read_bytes error: {}]", e).into())),
                }
            }
            "write_bytes" => {
                let path = args.first().map(|v| v.display()).unwrap_or_default();
                // 接受 Vec<Int>（0-255）写原始字节
                let data = args.get(1).and_then(|v| {
                    if let Value::Vec(arc) = v {
                        let mut bytes = Vec::new();
                        for item in arc.iter() {
                            if let Value::Int(i) = item {
                                bytes.push(*i as u8);
                            }
                        }
                        Some(bytes)
                    } else { None }
                });
                match data {
                    Some(bytes) => match std::fs::write(&path, &bytes) {
                        Ok(_) => Ok(Value::Bool(true)),
                        Err(_) => Ok(Value::Bool(false)),
                    },
                    None => Ok(Value::Bool(false)),
                }
            }
            // base64 编解码（传二进制到 AI 接口用）
            "base64_encode" => {
                let data = args.first().and_then(|v| {
                    if let Value::Vec(arc) = v {
                        let mut bytes = Vec::new();
                        for item in arc.iter() {
                            if let Value::Int(i) = item { bytes.push(*i as u8); }
                        }
                        Some(bytes)
                    } else if let Value::Str(st) = v {
                        Some(st.as_bytes().to_vec())
                    } else { None }
                });
                match data {
                    Some(bytes) => {
                        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                        let mut out = String::new();
                        for chunk in bytes.chunks(3) {
                            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
                            out.push(CHARS[(b[0] >> 2) as usize] as char);
                            out.push(CHARS[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
                            out.push(if chunk.len() > 1 { CHARS[(((b[1] & 0x0F) << 2) | (b[2] >> 6)) as usize] as char } else { '=' });
                            out.push(if chunk.len() > 2 { CHARS[(b[2] & 0x3F) as usize] as char } else { '=' });
                        }
                        Ok(Value::Str(out.into()))
                    }
                    None => Ok(Value::Str("".into())),
                }
            }
            "base64_decode" => {
                let input = args.first().map(|v| v.display()).unwrap_or_default();
                const REV: fn(u8) -> Option<u8> = |c| match c {
                    b'A'..=b'Z' => Some(c - b'A'),
                    b'a'..=b'z' => Some(c - b'a' + 26),
                    b'0'..=b'9' => Some(c - b'0' + 52),
                    b'+' => Some(62),
                    b'/' => Some(63),
                    _ => None,
                };
                let bytes: Vec<u8> = input.bytes().filter(|c| *c != b'=' && *c != b'\n' && *c != b'\r').collect();
                let mut out = Vec::new();
                for chunk in bytes.chunks(4) {
                    let b = [REV(chunk[0]).unwrap_or(0), REV(*chunk.get(1).unwrap_or(&b'A')).unwrap_or(0),
                             REV(*chunk.get(2).unwrap_or(&b'A')).unwrap_or(0), REV(*chunk.get(3).unwrap_or(&b'A')).unwrap_or(0)];
                    out.push((b[0] << 2) | (b[1] >> 4));
                    if chunk.len() > 2 { out.push((b[1] << 4) | (b[2] >> 2)); }
                    if chunk.len() > 3 { out.push((b[2] << 6) | b[3]); }
                }
                let items: Vec<Value> = out.iter().map(|b| Value::Int(*b as i64)).collect();
                Ok(Value::Vec(items.into()))
            }
            // 值语义内建（B5-M29）：move/clone 的解释器侧
            //（interp Vec 为持久化结构，clone 即共享引用，语义与 C 深拷贝一致）
            "al_vec_clone" => {
                match args.first() {
                    Some(Value::Vec(items)) => Ok(Value::Vec(items.clone())),
                    _ => Ok(Value::Vec(Vec::new().into())),
                }
            }
            "al_zero_vec" => Ok(Value::Vec(Vec::new().into())),
            "al_zero_map" => Ok(Value::Map(Vec::new())),
            "al_map_clone" => {
                match args.first() {
                    Some(Value::Map(entries)) => Ok(Value::Map(entries.clone())),
                    _ => Ok(Value::Map(Vec::new())),
                }
            }
            "al_strdup_lit" => {
                match args.first() {
                    Some(v @ Value::Str(_)) => Ok(v.clone()),
                    other => Ok(other.cloned().unwrap_or(Value::Str("".into()))),
                }
            }
            "al_strcat_own" => {
                let text: Vec<String> = args.iter().map(|v| v.display()).collect();
                Ok(Value::Str(text.join("").into()))
            }
            "run_ui" => {
                let comp = args.first().map(|v| v.display()).unwrap_or_default();
                let rendered = self.render_component(&comp);
                let line = format!("[ui] 组件 {} 渲染:\n{}", comp, rendered);
                self.output.push_str(&line);
                println!("{}", line);
                Ok(Value::Unit)
            }
            "Ok" => {
                let v = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::Result { ok: true, value: Box::new(v) })
            }
            "Err" => {
                let v = args.first().cloned().unwrap_or(Value::Str(String::new().into()));
                Ok(Value::Result { ok: false, value: Box::new(v) })
            }
            "Some" => {
                let v = args.first().cloned().unwrap_or(Value::Unit);
                Ok(Value::Option { some: true, value: Box::new(v) })
            }
            "None" => Ok(Value::Option { some: false, value: Box::new(Value::Unit) }),
            "Column" | "Row" | "Text" | "Button" | "TextInput" | "List" | "ListItem" | "Window"
            | "Dialog" | "Image" => Ok(Value::Nil),
            "db" => Ok(Value::Nil),
            _ => {
                if let Some((ty, arity)) = self.variants.get(name).cloned() {
                    if args.len() == arity {
                        return Ok(Value::Variant { ty, name: name.to_string(), payload: args.to_vec() });
                    }
                }
                if let Some(f) = self.fns.get(name).cloned() {
                    self.call_fn(&f, args)
                } else if let Some(v) = env.get(name) {
                    self.call_closure(&v, args)
                } else if let Some(v) = self.globals.get(name).cloned() {
                    self.call_closure(&v, args)
                } else {
                    Err(RtError::msg(format!("未定义的函数 '{}'", name)))
                }
            }
        }
    }

    /// 可变方法调用（push/pop/clear/sort/set/remove）的原地快路径：
    /// 当变量槽是缓冲区的唯一持有者时直接修改（O(1) 摊还），否则返回 None 回退
    /// 深拷贝路径。值语义不变：任何别名绑定都会使 strong_count > 1 而走回退。
    fn try_mutate_in_place(&mut self, env: &mut Env, name: &str, method: &str, args: &[Value]) -> Result<Option<Value>, RtError> {
        let Some(slot) = env.frames.iter_mut().rev().find_map(|f| f.get_mut(name)) else {
            return Ok(None);
        };
        match slot {
            Value::Vec(arc) => {
                let Some(items) = std::sync::Arc::get_mut(arc) else {
                    return Ok(None);
                };
                match method {
                    "push" => {
                        items.push(args.first().cloned().unwrap_or(Value::Unit));
                        Ok(Some(slot.clone()))
                    }
                    "pop" => Ok(Some(items.pop().unwrap_or(Value::Nil))),
                    "clear" => {
                        items.clear();
                        Ok(Some(Value::Unit))
                    }
                    "sort" => {
                        items.sort_by(|a, b| a.display().cmp(&b.display()));
                        Ok(Some(Value::Unit))
                    }
                    _ => Ok(None),
                }
            }
            Value::Map(entries) => match method {
                "set" => {
                    if let (Some(Value::Str(k)), Some(v)) = (args.first(), args.get(1)) {
                        let kstr = k.to_string();
                        if let Some(slot) = entries.iter_mut().find(|(ek, _)| *ek == kstr) {
                            slot.1 = v.clone();
                        } else {
                            entries.push((k.to_string(), v.clone()));
                        }
                    }
                    Ok(Some(Value::Unit))
                }
                "remove" => {
                    if let Some(Value::Str(k)) = args.first() {
                        let kref = k.as_ref();
                        entries.retain(|(ek, _)| ek.as_str() != kref);
                    }
                    Ok(Some(Value::Unit))
                }
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    fn call_method(&mut self, base: &Value, method: &str, args: &[Value]) -> Result<Value, RtError> {
        match base {
            Value::Str(s) => match method {
                "clone" => Ok(Value::Str(s.clone())),
                "len" => Ok(Value::Int(s.chars().count() as i64)),
                "to_f64" => match s.trim().parse::<f64>() {
                    Ok(f) => Ok(Value::Float(f)),
                    Err(_) => Err(RtError::msg(format!("无法解析为数值: '{}'", s))),
                },
                "to_i32" => match s.trim().parse::<i64>() {
                    Ok(i) => Ok(Value::Int(i)),
                    Err(_) => Err(RtError::msg(format!("无法解析为整数: '{}'", s))),
                },
                "starts_with" => {
                    let p = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| "".into());
                    Ok(Value::Bool(s.starts_with(&*p)))
                }
                "contains" => {
                    let p = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| "".into());
                    Ok(Value::Bool(s.contains(&*p)))
                }
                "ends_with" => {
                    let p = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| "".into());
                    Ok(Value::Bool(s.ends_with(&*p)))
                }
                "trim" => Ok(Value::Str(s.trim().into())),
                "split" => {
                    let sep = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| ",".into());
                    let parts: Vec<Value> = if sep.is_empty() {
                        s.chars().map(|c| Value::Str(c.to_string().into())).collect()
                    } else {
                        s.split(&*sep).map(|p| Value::Str(p.into())).collect()
                    };
                    Ok(Value::Vec(parts.into()))
                }
                "replace" => {
                    let from = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| "".into());
                    let to = args.get(1).and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None }).unwrap_or_else(|| "".into());
                    Ok(Value::Str(s.replace(&*from, &*to).into()))
                }
                _ => Ok(Value::Nil),
            },
            Value::Vec(items) => match method {
                "len" => Ok(Value::Int(items.len() as i64)),
                "push" => {
                    let mut new_items: Vec<Value> = items.iter().cloned().collect();
                    if let Some(v) = args.first() {
                        new_items.push(v.clone());
                    }
                    Ok(Value::Vec(new_items.into()))
                }
                "iter" => Ok(Value::Vec(items.clone())),
                "map" => {
                    let mut out = Vec::new();
                    for item in items.iter() {
                        if let Some(f) = args.first() {
                            out.push(self.call_closure(f, &[item.clone()])?);
                        }
                    }
                    Ok(Value::Vec(out.into()))
                }
                "sum" => {
                    let mut total = 0.0;
                    let mut is_float = false;
                    for item in items.iter() {
                        match item {
                            Value::Int(i) => total += *i as f64,
                            Value::Float(f) => {
                                total += *f;
                                is_float = true;
                            }
                            _ => {}
                        }
                    }
                    if is_float {
                        Ok(Value::Float(total))
                    } else {
                        Ok(Value::Int(total as i64))
                    }
                }
                "get" => {
                    let i = args.first().map(int_of).unwrap_or(0) as usize;
                    Ok(items.get(i).cloned().unwrap_or(Value::Nil))
                }
                "pop" => {
                    let mut new_items: Vec<Value> = items.iter().cloned().collect();
                    let last = new_items.pop();
                    Ok(last.unwrap_or(Value::Nil))
                }
                "clear" => Ok(Value::Vec(Vec::new().into())),
                "sort" => {
                    let mut new_items: Vec<Value> = items.iter().cloned().collect();
                    new_items.sort_by(|a, b| a.display().cmp(&b.display()));
                    Ok(Value::Vec(new_items.into()))
                }
                _ => Ok(Value::Nil),
            },
            Value::Map(entries) => match method {
                "set" => {
                    let mut new_entries = entries.clone();
                    if let (Some(Value::Str(k)), Some(v)) = (args.first(), args.get(1)) {
                        let kstr = k.to_string();
                        if let Some(slot) = new_entries.iter_mut().find(|(ek, _)| *ek == kstr) {
                            slot.1 = v.clone();
                        } else {
                            new_entries.push((k.to_string(), v.clone()));
                        }
                    }
                    Ok(Value::Map(new_entries))
                }
                "get" => {
                    let k = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None });
                    let v = k.and_then(|k| entries.iter().find(|(ek, _)| ek.as_str() == &*k).map(|(_, v)| v.clone()));
                    match v {
                        Some(v) => Ok(Value::Option { some: true, value: Box::new(v) }),
                        None => Ok(Value::Option { some: false, value: Box::new(Value::Unit) }),
                    }
                }
                "contains" => {
                    let k = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None });
                    Ok(Value::Bool(k.map(|k| entries.iter().any(|(ek, _)| ek.as_str() == &*k)).unwrap_or(false)))
                }
                "len" => Ok(Value::Int(entries.len() as i64)),
                "keys" => {
                    let v: Vec<Value> = entries.iter().map(|(k, _)| Value::Str(k.clone().into())).collect();
                    Ok(Value::Vec(v.into()))
                }
                "remove" => {
                    let k = args.first().and_then(|v| match v { Value::Str(x) => Some(x.clone()), _ => None });
                    let new_entries: Vec<(String, Value)> = entries
                        .iter()
                        .filter(|(ek, _)| k.as_deref().map(|k| ek.as_str() != k).unwrap_or(true))
                        .cloned()
                        .collect();
                    Ok(Value::Map(new_entries))
                }
                "values" => {
                    let v: Vec<Value> = entries.iter().map(|(_, v)| v.clone()).collect();
                    Ok(Value::Vec(v.into()))
                }
                _ => Ok(Value::Nil),
            },
            Value::MapType => match method {
                "new" => Ok(Value::Map(Vec::new())),
                _ => Ok(Value::Nil),
            },
            Value::Option { some, value } => match method {
                "unwrap_or" => {
                    let default = args.first().cloned().unwrap_or(Value::Unit);
                    Ok(if *some { *value.clone() } else { default })
                }
                "unwrap" => {
                    if *some {
                        Ok(*value.clone())
                    } else {
                        Err(RtError::Propagate(Value::Option { some: false, value: Box::new(Value::Unit) }))
                    }
                }
                "is_some" => Ok(Value::Bool(*some)),
                _ => Ok(Value::Nil),
            },
            Value::Result { ok, value } => match method {
                "unwrap_or" => {
                    let default = args.first().cloned().unwrap_or(Value::Unit);
                    Ok(if *ok { *value.clone() } else { default })
                }
                "unwrap" => {
                    if *ok {
                        Ok(*value.clone())
                    } else {
                        Err(RtError::Propagate(*value.clone()))
                    }
                }
                _ => Ok(Value::Nil),
            },
            Value::Struct { ty, .. } => {
                // user impl method: self bound as first param
                if let Some(methods) = self.impls.get(ty).cloned() {
                    if let Some(f) = methods.into_iter().find(|f| f.sig.name == method) {
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(base.clone());
                        call_args.extend_from_slice(args);
                        return self.call_fn(&f, &call_args);
                    }
                }
                Ok(Value::Nil)
            }
            Value::Nil => match method {
                "init" | "insert" | "delete" => Ok(Value::Unit),
                "query" => Ok(Value::Vec(Vec::new().into())),
                _ => Ok(Value::Nil),
            },
            _ => Ok(Value::Nil),
        }
    }

    // ---- f-string interpolation ----

    fn eval_fmt(&mut self, env: &mut Env, s: &str) -> Result<Value, RtError> {
        let mut out = String::new();
        let mut rest = s;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    let expr_src = &after[..end];
                    if let Some(expr) = crate::parser::parse_expression_str(expr_src) {
                        let v = self.eval(env, &expr)?;
                        out.push_str(&v.display());
                    } else {
                        out.push_str(&format!("{{{}}}", expr_src));
                    }
                    rest = &after[end + 1..];
                }
                None => {
                    out.push_str(&rest[start..]);
                    break;
                }
            }
        }
        out.push_str(rest);
        Ok(Value::Str(out.into()))
    }
}

/// Public text of a runtime error (for the CLI).
pub fn rt_error_text(e: &RtError) -> String {
    err_text(e)
}

/// Vec/Map methods that mutate the variable binding.
fn is_mutating_method(m: &str) -> bool {
    matches!(m, "push" | "pop" | "sort" | "clear" | "insert" | "remove" | "set")
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        Value::Float(f) => *f as i64,
        Value::Bool(b) => *b as i64,
        _ => 0,
    }
}

fn num_as_f64(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => 0.0,
    }
}

fn err_text(e: &RtError) -> String {
    match e {
        RtError::Msg(m) => m.clone(),
        RtError::Propagate(v) => format!("Err({})", v.display()),
        RtError::Return(v) => format!("return({})", v.display()),
    }
}

fn err_value(e: &RtError) -> Value {
    match e {
        RtError::Msg(m) => Value::Str(m.clone().into()),
        RtError::Propagate(v) => v.clone(),
        RtError::Return(v) => v.clone(),
    }
}


/// Value 的调试短摘要
pub fn value_short(v: &Value) -> String {
    match v {
        Value::Int(i) => format!("{}", i),
        Value::Float(f) => format!("{}", f),
        Value::Bool(b) => format!("{}", b),
        Value::Str(s) => {
            let t: String = s.chars().take(24).collect();
            format!("\"{}\"", t)
        }
        Value::Vec(items) => format!("Vec[{}]", items.len()),
        Value::Map(m) => format!("Map[{}]", m.len()),
        Value::Option { some, .. } => if *some { "Some".into() } else { "None".into() },
        Value::Result { .. } => "Result".into(),
        Value::Unit => "()".into(),
        Value::Closure { .. } => "closure".into(),
        other => format!("{:?}", other),
    }
}

// ---- db 内建模块（文件 JSONL 存储;仅解释器运行）----

fn db_value_to_json(v: &Value) -> String {
    match v {
        Value::Int(i) => format!("{}", i),
        Value::Float(f) => format!("{}", f),
        Value::Bool(b) => format!("{}", b),
        Value::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Value::Struct { fields, .. } => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(k, fv)| format!("\"{}\":{}", k, db_value_to_json(fv)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Unit => "null".into(),
        other => format!("\"{}\"", value_short(other)),
    }
}

/// JSON 对象行 → 键值字符串表（值去引号）
fn db_parse_row(line: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let n = bytes.len();
    while i < n {
        // 找 "key":
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let ks = i + 1;
        let mut ke = ks;
        while ke < n && bytes[ke] != b'"' {
            ke += 1;
        }
        let key = line[ks..ke].to_string();
        i = ke + 1;
        while i < n && (bytes[i] == b' ' || bytes[i] == b':') {
            i += 1;
        }
        // 值: 字符串(去引号)或原始
        let mut vs = i;
        let mut ve = vs;
        if i < n && bytes[i] == b'"' {
            vs = i + 1;
            i += 1;
            while i < n && bytes[i] != b'"' {
                i += 1;
            }
            ve = i;
            i += 1;
        } else {
            while i < n && bytes[i] != b',' && bytes[i] != b'}' {
                i += 1;
            }
            ve = i;
        }
        let val = line[vs..ve].to_string();
        out.push((key, val));
        while i < n && bytes[i] != b',' {
            if bytes[i] == b'}' {
                break;
            }
            i += 1;
        }
        if i < n && bytes[i] == b',' {
            i += 1;
        }
    }
    out
}

/// 解析 SQL 子集：SELECT * FROM <table> [ORDER BY <col> [DESC]]
fn db_parse_sql(sql: &str) -> (String, Option<String>, bool) {
    let mut table = String::new();
    let mut order_col = None;
    let mut desc = false;
    let upper = sql.to_uppercase();
    if let Some(fi) = upper.find("FROM ") {
        let rest = &sql[fi + 5..];
        let mut t = String::new();
        for c in rest.chars() {
            if c.is_whitespace() {
                break;
            }
            t.push(c);
        }
        table = t;
    }
    if let Some(oi) = upper.find("ORDER BY ") {
        let rest = &sql[oi + 9..];
        let mut col = String::new();
        for c in rest.chars() {
            if c.is_whitespace() {
                break;
            }
            col.push(c);
        }
        order_col = Some(col);
        desc = upper[oi..].contains("DESC");
    }
    (table, order_col, desc)
}

impl Interp {
    fn call_db_method(&mut self, method: &str, args: &[Value]) -> Result<Value, RtError> {
        match method {
            "init" => {
                let path = match args.first() {
                    Some(Value::Str(p)) => p.to_string(),
                    _ => return Err(RtError::msg("db.init 需要文件路径")),
                };
                // 初始化: 确保文件可写(不存在则创建)
                let _ = std::fs::OpenOptions::new().append(true).create(true).open(&path);
                Ok(Value::Unit)
            }
            "insert" => {
                let table = args.first().map(db_table_arg).unwrap_or_default();
                let record = args.get(1).cloned().unwrap_or(Value::Unit);
                let path = self.db_path();
                let mut line = format!("{{\"__table\":\"{}\",", table);
                match &record {
                    Value::Struct { fields, .. } => {
                        let parts: Vec<String> = fields
                            .iter()
                            .map(|(k, fv)| format!("\"{}\":{}", k, db_value_to_json(fv)))
                            .collect();
                        line.push_str(&parts.join(","));
                    }
                    other => {
                        line.push_str(&format!("\"value\":{}", db_value_to_json(other)));
                    }
                }
                line.push_str("}\n");
                let mut f = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .map_err(|e| RtError::msg(format!("db.insert: {}", e)))?;
                use std::io::Write;
                f.write_all(line.as_bytes())
                    .map_err(|e| RtError::msg(format!("db.insert: {}", e)))?;
                Ok(Value::Unit)
            }
            "query" => {
                let sql = args.first().map(|v| v.display()).unwrap_or_default();
                let (table, order_col, desc) = db_parse_sql(&sql);
                let path = self.db_path();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let mut rows: Vec<Vec<(String, String)>> = Vec::new();
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let row = db_parse_row(line);
                    let t = row.iter().find(|(k, _)| k == "__table").map(|(_, v)| v.clone()).unwrap_or_default();
                    if t == table {
                        rows.push(row);
                    }
                }
                if let Some(col) = order_col {
                    rows.sort_by(|a, b| {
                        let av = a.iter().find(|(k, _)| k == &col).map(|(_, v)| v.clone()).unwrap_or_default();
                        let bv = b.iter().find(|(k, _)| k == &col).map(|(_, v)| v.clone()).unwrap_or_default();
                        let ord = av.cmp(&bv);
                        if desc { ord.reverse() } else { ord }
                    });
                }
                // 每行 → Map<String, String>(值字符串)
                let maps: Vec<Value> = rows
                    .into_iter()
                    .map(|row| {
                        let entries: Vec<(String, Value)> = row
                            .into_iter()
                            .filter(|(k, _)| k != "__table")
                            .map(|(k, v)| (k, Value::Str(v.into())))
                            .collect();
                        Value::Map(entries)
                    })
                    .collect();
                Ok(Value::Vec(std::sync::Arc::new(maps)))
            }
            "delete" => {
                let table = args.first().map(db_table_arg).unwrap_or_default();
                let id = args.get(1).map(|v| v.display()).unwrap_or_default();
                let path = self.db_path();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let mut kept = String::new();
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let row = db_parse_row(line);
                    let t = row.iter().find(|(k, _)| k == "__table").map(|(_, v)| v.clone()).unwrap_or_default();
                    let rid = row.iter().find(|(k, _)| k == "id").map(|(_, v)| v.clone()).unwrap_or_default();
                    if !(t == table && rid == id) {
                        kept.push_str(line);
                        kept.push('\n');
                    }
                }
                let _ = std::fs::write(&path, kept);
                Ok(Value::Unit)
            }
            _ => Err(RtError::msg(format!("db 无方法 {}", method))),
        }
    }

    fn db_path(&self) -> String {
        // 最近一次 init 的路径（默认 account.db）
        "account.db".to_string()
    }
}

fn db_table_arg(v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        other => value_short(other),
    }
}

// ---- http 内建模块（std::net;仅解释器运行）----

impl Interp {
    fn call_http_method(&mut self, method: &str, args: &[Value]) -> Result<Value, RtError> {
        match method {
            "get" => {
                let url = args.first().map(|v| v.display()).unwrap_or_default();
                match http_get(&url) {
                    Ok(body) => Ok(Value::Result { ok: true, value: Box::new(Value::Str(body.into())) }),
                    Err(e) => Ok(Value::Result { ok: false, value: Box::new(Value::Str(e.into())) }),
                }
            }
            _ => Err(RtError::msg(format!("http 无方法 {}", method))),
        }
    }
}

/// HTTP GET（HTTP/1.0 连接关闭即响应尾）
fn http_get(url: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    // 解析 http://host[:port]/path
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| "仅支持 http:// URL".to_string())?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match hostport.find(':') {
        Some(i) => (
            &hostport[..i],
            hostport[i + 1..].parse::<u16>().unwrap_or(80),
        ),
        None => (hostport, 80),
    };
    let addr = format!("{}:{}", host, port);
    let mut stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("连接失败 {}: {}", addr, e))?;
    let req = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("发送失败: {}", e))?;
    let mut resp = Vec::new();
    stream
        .read_to_end(&mut resp)
        .map_err(|e| format!("读取失败: {}", e))?;
    let text = String::from_utf8_lossy(&resp).into_owned();
    // 状态行检查
    let status = text.lines().next().unwrap_or("").to_string();
    if !status.contains("200") {
        return Err(format!("HTTP {}", status));
    }
    // HTTP/1.0: 空行后为 body
    match text.find("\r\n\r\n") {
        Some(i) => Ok(text[i + 4..].to_string()),
        None => Ok(text),
    }
}
