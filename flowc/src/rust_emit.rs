//! D16: Aine → Rust 代码发射器(Rust 后端 PoC)。
//!
//! 前端(lex/parse/resolve/typeck)全复用;本模块把宿主 AST 发射为 Rust 源码。
//! 设计:SD-1 64 位域(wrap+panic 语义与宿主一致);SD-9 容器取值 Option;
//! 值语义 = 容器赋值/传参 .clone();Map 插入序(__Map);D15 字符语义;W34 缺臂 = rustc 原生拒绝。

use crate::ast::*;
use std::collections::HashMap as StdMap;

// 块尾模式: Stmt=语句位(恒分号) FnBody=函数体(cur_ret 决定) Expr=表达式位(infer 决定)
#[derive(Clone, Copy, PartialEq)]
enum BlockMode { Stmt, FnBody, Expr }

const PRELUDE: &str = r#"// 本文件由 Aine Rust 后端自动生成
#![allow(unused)]

fn __add(a: i64, b: i64) -> i64 { a.wrapping_add(b) }
fn __sub(a: i64, b: i64) -> i64 { a.wrapping_sub(b) }
fn __mul(a: i64, b: i64) -> i64 { a.wrapping_mul(b) }
fn __div(a: i64, b: i64) -> i64 {
    if b == 0 { eprintln!("除零错误"); std::process::exit(101); }
    if a == i64::MIN && b == -1 { eprintln!("除法溢出: i64 最小值除以 -1"); std::process::exit(101); }
    a.wrapping_div(b)
}
fn __rem(a: i64, b: i64) -> i64 {
    if b == 0 { eprintln!("除零错误"); std::process::exit(101); }
    if a == i64::MIN && b == -1 { eprintln!("取余溢出: i64 最小值对 -1"); std::process::exit(101); }
    a.wrapping_rem(b)
}
fn __neg(a: i64) -> i64 {
    if a == i64::MIN { eprintln!("取负溢出: i64 最小值"); std::process::exit(101); }
    -a
}
fn __shl(a: i64, b: i64) -> i64 {
    if b < 0 || b > 63 { eprintln!("移位量越界: {}", b); std::process::exit(101); }
    ((a as u64) << b) as i64
}
fn __shr(a: i64, b: i64) -> i64 {
    if b < 0 || b > 63 { eprintln!("移位量越界: {}", b); std::process::exit(101); }
    a >> b
}
fn __strcat(a: &str, b: &str) -> String { format!("{}{}", a, b) }
fn __fnum(v: f64) -> String {
    if v == (v as i64) as f64 && v >= -1e15 && v <= 1e15 { format!("{:.1}", v) } else { __g(v) }
}
fn __g(v: f64) -> String {
    if v == 0.0 { return "0".to_string(); }
    let mag = v.abs().log10().floor() as i32;
    if mag < -5 || mag >= 6 {
        let mut s = format!("{:.5e}", v);
        if let Some(p) = s.find('e') {
            let (m, ex) = s.split_at(p);
            let m2 = m.trim_end_matches('0').trim_end_matches('.');
            s = format!("{}{}", m2, ex);
        }
        s
    } else {
        let dec = (5 - mag).max(0) as usize;
        let mut s = format!("{:.*}", dec, v);
        if s.contains('.') { while s.ends_with('0') { s.pop(); } if s.ends_with('.') { s.pop(); } }
        s
    }
}

#[derive(Clone, PartialEq, Debug)]
struct __Map<V> { keys: Vec<String>, vals: Vec<V> }
impl<V: Clone> __Map<V> {
    fn new() -> Self { __Map { keys: Vec::new(), vals: Vec::new() } }
    fn insert(&mut self, k: String, v: V) {
        if let Some(i) = self.keys.iter().position(|x| x == &k) { self.vals[i] = v; }
        else { self.keys.push(k); self.vals.push(v); }
    }
    fn get(&self, k: &str) -> Option<&V> {
        self.keys.iter().position(|x| x == k).and_then(|i| self.vals.get(i))
    }
    fn len(&self) -> i64 { self.keys.len() as i64 }
    fn contains(&self, k: &str) -> bool { self.keys.iter().any(|x| x == k) }
    fn remove(&mut self, k: &str) {
        if let Some(i) = self.keys.iter().position(|x| x == k) { self.keys.remove(i); self.vals.remove(i); }
    }
    fn keys(&self) -> Vec<String> { self.keys.clone() }
    fn values(&self) -> Vec<V> { self.vals.clone() }
}

trait __Disp { fn __disp(&self) -> String; }
impl __Disp for () { fn __disp(&self) -> String { "()".to_string() } }
impl __Disp for i64 { fn __disp(&self) -> String { self.to_string() } }
impl __Disp for f64 { fn __disp(&self) -> String { __fnum(*self) } }
impl __Disp for bool { fn __disp(&self) -> String { if *self { "true".into() } else { "false".into() } } }
impl __Disp for String { fn __disp(&self) -> String { self.clone() } }
impl<T: __Disp> __Disp for Vec<T> {
    fn __disp(&self) -> String {
        let parts: Vec<String> = self.iter().map(|x| x.__disp()).collect();
        format!("[{}]", parts.join(", "))
    }
}
impl<V: __Disp> __Disp for __Map<V> {
    fn __disp(&self) -> String {
        if self.keys.is_empty() { return "{  }".to_string(); }
        let parts: Vec<String> = (0..self.keys.len())
            .map(|i| format!("{}: {}", self.keys[i], self.vals[i].__disp())).collect();
        format!("{{ {} }}", parts.join(", "))
    }
}
impl<T: __Disp> __Disp for Option<T> {
    fn __disp(&self) -> String {
        match self { Some(x) => format!("Some({})", x.__disp()), None => "None".into() }
    }
}
impl<T: __Disp, E: __Disp> __Disp for Result<T, E> {
    fn __disp(&self) -> String {
        match self {
            Ok(x) => format!("Ok({})", x.__disp()),
            Err(e) => format!("Err({})", e.__disp()),
        }
    }
}

fn __cli_args() -> Vec<String> { std::env::args().skip(1).collect() }
fn __now() -> i64 { use std::time::{SystemTime, UNIX_EPOCH}; SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0) }
fn __now_ms() -> i64 { use std::time::{SystemTime, UNIX_EPOCH}; SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) }
fn __read_line() -> String { let mut b = String::new(); use std::io::BufRead; std::io::stdin().lock().read_line(&mut b).unwrap_or(0); b.trim_end_matches('\n').trim_end_matches('\r').to_string() }
fn __write_file(p: &str, c: &str) -> bool { std::fs::write(p, c).is_ok() }
fn __append_file(p: &str, c: &str) -> bool { use std::io::Write; if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) { f.write_all(c.as_bytes()).is_ok() } else { false } }
fn __file_exists(p: &str) -> bool { std::path::Path::new(p).exists() }
fn __b64_encode_bytes(v: &Vec<i64>) -> String {
    let bytes: Vec<u8> = v.iter().map(|x| *x as u8).collect();
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        out.push(CHARS[(b[0] >> 2) as usize] as char);
        out.push(CHARS[(((b[0] & 0x03) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[(((b[1] & 0x0F) << 2) | (b[2] >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(b[2] & 0x3F) as usize] as char } else { '=' });
    }
    out
}
fn __b64_encode_str(s: &str) -> String { let b: Vec<i64> = s.bytes().map(|x| x as i64).collect(); __b64_encode_bytes(&b) }
fn __b64_decode(s: &str) -> Vec<i64> {
    let rev = |c: u8| -> Option<u8> { match c { b'A'..=b'Z' => Some(c - b'A'), b'a'..=b'z' => Some(c - b'a' + 26), b'0'..=b'9' => Some(c - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None } };
    let bytes: Vec<u8> = s.bytes().filter(|c| *c != b'=' && *c != b'\n' && *c != b'\r').collect();
    let mut out: Vec<i64> = Vec::new();
    for chunk in bytes.chunks(4) {
        let b = [rev(chunk[0]).unwrap_or(0), rev(*chunk.get(1).unwrap_or(&b'A')).unwrap_or(0), rev(*chunk.get(2).unwrap_or(&b'A')).unwrap_or(0), rev(*chunk.get(3).unwrap_or(&b'A')).unwrap_or(0)];
        out.push(((b[0] << 2) | (b[1] >> 4)) as i64);
        if chunk.len() > 2 { out.push(((b[1] << 4) | (b[2] >> 2)) as i64); }
        if chunk.len() > 3 { out.push(((b[2] << 6) | b[3]) as i64); }
    }
    out
}
fn __to_bytes(s: &str) -> Vec<i64> { s.bytes().map(|x| x as i64).collect() }
impl<V: Clone> Default for __Map<V> {
    fn default() -> Self { __Map::new() }
}
fn __read_bytes(p: &str) -> Vec<i64> {
    match std::fs::read(p) {
        Ok(b) => b.iter().map(|x| *x as i64).collect(),
        Err(e) => { eprintln!("[read_bytes error: {}]", e); Vec::new() }
    }
}
fn __write_bytes(p: &str, data: &Vec<i64>) -> bool {
    let bytes: Vec<u8> = data.iter().map(|x| *x as u8).collect();
    std::fs::write(p, bytes).is_ok()
}
fn __file_list(p: &str) -> Vec<String> { std::fs::read_dir(p).map(|rd| rd.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string())).collect()).unwrap_or_default() }
fn __sleep(ms: i64) { std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64)); }
fn __env(k: &str) -> String { std::env::var(k).unwrap_or_default() }
fn __vslice<T: Clone>(v: &Vec<T>, a: i64, b: i64) -> Vec<T> {
    let s = (a.max(0) as usize).min(v.len());
    let e = (b.max(0) as usize).min(v.len());
    if s >= e { Vec::new() } else { v[s..e].to_vec() }
}
fn __try_opt<T>(o: Option<T>) -> T {
    match o { Some(v) => v, None => { eprintln!("错误: None 传播到顶层"); std::process::exit(101); } }
}
fn __try_res<T, E: std::fmt::Display>(r: Result<T, E>) -> T {
    match r { Ok(v) => v, Err(e) => { eprintln!("错误: {}", e); std::process::exit(101); } }
}
fn __len_s(s: &String) -> i64 { s.chars().count() as i64 }
fn __char_at(s: &String, i: i64) -> String {
    match s.chars().nth(i as usize) {
        Some(c) => c.to_string(),
        None => { eprintln!("字符串索引越界: idx={}", i); std::process::exit(101); }
    }
}
fn __substr(s: &String, a: i64, b: i64) -> String {
    s.chars().skip(a.max(0) as usize).take((b - a).max(0) as usize).collect()
}
fn __push<T: Clone>(v: &mut Vec<T>, x: T) -> Vec<T> { v.push(x); v.clone() }
fn __pop<T: Clone>(v: &mut Vec<T>) -> Option<T> { v.pop() }
fn __vset<T: Clone>(v: &mut Vec<T>, i: i64, x: T) -> Vec<T> {
    if i < 0 || i as usize >= v.len() { eprintln!("索引越界: idx={} len={}", i, v.len()); std::process::exit(101); }
    v[i as usize] = x; v.clone()
}
fn __vinsert<T: Clone>(v: &mut Vec<T>, i: i64, x: T) -> Vec<T> {
    let p = if i < 0 { 0usize } else { (i as usize).min(v.len()) };
    v.insert(p, x); v.clone()
}
fn __vremove<T: Clone>(v: &mut Vec<T>, i: i64) -> Vec<T> {
    if i < 0 || i as usize >= v.len() { eprintln!("索引越界: idx={} len={}", i, v.len()); std::process::exit(101); }
    v.remove(i as usize); v.clone()
}
fn __vconcat<T: Clone>(a: &Vec<T>, b: &Vec<T>) -> Vec<T> {
    let mut r = a.clone(); r.extend(b.iter().cloned()); r
}
fn __vsort<T: __Disp>(v: &mut Vec<T>) { v.sort_by(|a, b| a.__disp().cmp(&b.__disp())); }
fn __read_file(p: &str) -> String {
    match std::fs::read_to_string(p) { Ok(s) => s, Err(e) => format!("[read_file error: {}]", e) }
}
fn __map_get_i(m: &__Map<i64>, k: &str) -> i64 {
    match m.get(k) { Some(v) => *v, None => { eprintln!("Map 中不存在键: {}", k); std::process::exit(101); } }
}
fn __map_get_s(m: &__Map<String>, k: &str) -> String {
    match m.get(k) { Some(v) => v.clone(), None => { eprintln!("Map 中不存在键: {}", k); std::process::exit(101); } }
}
fn __mget<V: Clone>(m: &__Map<V>, k: &str) -> V {
    match m.get(k) { Some(v) => v.clone(), None => { eprintln!("Map 中不存在键: {}", k); std::process::exit(101); } }
}
fn __bytes_to_string(bytes: &Vec<i64>) -> String {
    let mut r = String::new();
    for b in bytes { r.push(*b as u8 as char); }
    r
}
"#;

#[derive(Clone, PartialEq, Debug)]
pub enum RTy {
    I64,
    F64,
    Bool,
    Str,
    Vec(Box<RTy>),
    Map(Box<RTy>),
    Opt(Box<RTy>),
    Res(Box<RTy>, Box<RTy>),
    Named(String),
    /// 递归类型的 Rust 侧指针包装(Box<T>) — 宿主语义里它就是 T 本身
    BoxedT(Box<RTy>),
    /// 统一函数型 fn(A,B)->R (fn 指针; fn item 调用位自动强转)
    FnPtr(Vec<RTy>, Box<RTy>),
    Unit,
    Unknown,
}

impl RTy {
    fn rust_str(&self) -> String {
        match self {
            RTy::I64 => "i64".into(),
            RTy::F64 => "f64".into(),
            RTy::Bool => "bool".into(),
            RTy::Str => "String".into(),
            RTy::Vec(t) => format!("Vec<{}>", t.rust_str()),
            RTy::Map(v) => format!("__Map<{}>", v.rust_str()),
            RTy::Opt(t) => format!("Option<{}>", t.rust_str()),
            RTy::Res(a, b) => format!("Result<{}, {}>", a.rust_str(), b.rust_str()),
            RTy::Named(n) => n.clone(),
            RTy::BoxedT(t) => format!("Box<{}>", t.rust_str()),
            RTy::FnPtr(ps, r) => format!("fn({}) -> {}", ps.iter().map(|p| p.rust_str()).collect::<Vec<_>>().join(", "), r.rust_str()),
            RTy::Unit => "()".into(),
            RTy::Unknown => "_".into(),
        }
    }
    fn is_container(&self) -> bool {
        matches!(self, RTy::Vec(_) | RTy::Map(_))
    }
    // 剥掉 BoxedT 包装(宿主语义: Boxed 值就是 T 值)
    fn unboxed(&self) -> RTy {
        match self {
            RTy::BoxedT(t) => (**t).clone(),
            other => other.clone(),
        }
    }
}

fn esc_name(n: &str) -> String {
    const RUST_KW: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
        "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become",
        "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual",
        "yield", "try", "gen",
    ];
    if RUST_KW.contains(&n) { format!("{}_", n) } else { n.to_string() }
}

fn rust_str_lit(s: &str) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn default_value(t: &RTy) -> String {
    match t {
        RTy::I64 => "0".into(),
        RTy::F64 => "0.0".into(),
        RTy::Bool => "false".into(),
        RTy::Str => "String::new()".into(),
        RTy::Vec(_) => "Vec::new()".into(),
        RTy::Map(_) => "__Map::new()".into(),
        RTy::Opt(_) => "None".into(),
        RTy::Res(a, _) => format!("Ok({})", default_value(a)),
        RTy::Named(n) => format!("{}::default()", n),
        _ => "()".into(),
    }
}

struct Emitter {
    fns: StdMap<String, Vec<RTy>>,
    fn_ret: StdMap<String, RTy>,
    structs: StdMap<String, Vec<(String, RTy)>>,
    enums: StdMap<String, Vec<(String, usize)>>,
    variant_types: StdMap<String, Vec<RTy>>,
    /// 递归直边: owner 类型 -> 需 Box 的目标类型名集合
    boxed_edges: StdMap<String, Vec<String>>,
    /// @global 注册表: 名 -> 具体类型
    globals: StdMap<String, RTy>,
    /// @global 定义暂存(两遍: 先全签名,再推全局类型)
    global_defs: Vec<crate::ast::GlobalStateDef>,
    /// 模式解构临时绑定计数
    tmp_n: usize,
    loop_depth: usize,
    vars: Vec<(String, RTy)>,
    cur_ret: RTy,
    cur_is_main: bool,
}

impl Emitter {
    fn new() -> Self {
        Emitter {
            fns: StdMap::new(),
            fn_ret: StdMap::new(),
            structs: StdMap::new(),
            enums: StdMap::new(),
            variant_types: StdMap::new(),
            boxed_edges: StdMap::new(),
            globals: StdMap::new(),
            global_defs: Vec::new(),
            tmp_n: 0,
            loop_depth: 0,
            vars: Vec::new(),
            cur_ret: RTy::Unit,
            cur_is_main: false,
        }
    }

    fn collect(&mut self, p: &Program) {
        for item in &p.items {
            match item {
                Item::Fn(f) => {
                    let pts: Vec<RTy> = f.sig.params.iter().map(|p| self.ty_of_opt(&p.ty)).collect();
                    let ret = f.sig.ret.as_ref().map(|t| self.ty_of(t)).unwrap_or(RTy::Unit);
                    self.fns.insert(f.sig.name.clone(), pts);
                    self.fn_ret.insert(f.sig.name.clone(), ret);
                }
                Item::Struct(sd) => {
                    let fields: Vec<(String, RTy)> = sd
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.ty_of(&f.ty)))
                        .collect();
                    self.structs.insert(sd.name.clone(), fields);
                }
                Item::Enum(ed) => {
                    let vs: Vec<(String, usize)> = ed.variants.iter().map(|v| (v.name.clone(), v.payload.len())).collect();
                    self.enums.insert(ed.name.clone(), vs);
                    for v in &ed.variants {
                        let pts: Vec<RTy> = v.payload.iter().map(|t| self.ty_of(t)).collect();
                        self.variant_types.insert(v.name.clone(), pts);
                    }
                }
                Item::GlobalState(g) => {
                    self.global_defs.push(g.clone());
                }
                _ => {}
            }
        }
        self.detect_recursive();
    }
    /// 直边递归检测: Vec/Map 后面的载荷不算边(堆间接); Option 直传算边
    fn detect_recursive(&mut self) {
        let mut edges: Vec<(String, String)> = Vec::new();
        for (sn, fields) in &self.structs {
            for (_fn2, ft) in fields {
                if let RTy::Named(n) = ft { edges.push((sn.clone(), n.clone())); }
            }
        }
        for (en, vs) in &self.enums {
            for (vn, _ar) in vs {
                if let Some(pts) = self.variant_types.get(vn) {
                    for pt in pts {
                        let mut tgt = pt;
                        while let RTy::Opt(t) = tgt { tgt = t; }
                        if let RTy::Named(n) = tgt { edges.push((en.clone(), n.clone())); }
                    }
                }
            }
        }
        // 可达性(直接边) — 只有自己能回到自己的类型在环上
        let mut names: Vec<String> = Vec::new();
        for (sn, _f) in &self.structs { names.push(sn.clone()); }
        for (en, _v) in &self.enums { names.push(en.clone()); }
        let reaches = |from: &str, to: &str| -> bool {
            // BFS
            let mut seen: Vec<String> = Vec::new();
            let mut work: Vec<String> = vec![from.to_string()];
            while let Some(cur) = work.pop() {
                if cur == to && cur != from { return true; }
                if seen.contains(&cur) { continue; }
                seen.push(cur.clone());
                for (a, b) in &edges {
                    if *a == cur { work.push(b.clone()); }
                }
            }
            // from == to 的自环检查: 从 from 出发能否回到 from
            let mut seen2: Vec<String> = Vec::new();
            let mut work2: Vec<String> = Vec::new();
            for (a, b) in &edges { if *a == from { work2.push(b.clone()); } }
            while let Some(cur) = work2.pop() {
                if *cur == *from { return true; }
                if seen2.contains(&cur) { continue; }
                seen2.push(cur.clone());
                for (a, b) in &edges {
                    if *a == cur { work2.push(b.clone()); }
                }
            }
            false
        };
        let mut boxed: Vec<(String, String)> = Vec::new();
        // 同环判定: a↔b 双向可达才 box(Stmt->Expr 单向不 box,Expr 的有限性靠自环 box 保证)
        for (a, b) in &edges {
            if reaches(a, b) && reaches(b, a) { boxed.push((a.clone(), b.clone())); }
        }
        for (a, b) in boxed {
            self.boxed_edges.entry(a).or_default().push(b);
        }
    }
    fn is_boxed_edge(&self, owner: &str, target: &str) -> bool {
        self.boxed_edges.get(owner).map(|v| v.iter().any(|x| x == target)).unwrap_or(false)
    }

    fn ty_of(&self, t: &Type) -> RTy {
        match t {
            Type::Path(p) => {
                let name = p.segments.join("::");
                if let Some(g) = p.generics.first() {
                    let inner: Vec<RTy> = g.iter().map(|t| self.ty_of(t)).collect();
                    return match name.as_str() {
                        "Vec" => RTy::Vec(Box::new(inner.into_iter().next().unwrap_or(RTy::Unknown))),
                        "Map" => RTy::Map(Box::new(inner.get(1).cloned().unwrap_or(RTy::Unknown))),
                        "Option" => RTy::Opt(Box::new(inner.into_iter().next().unwrap_or(RTy::Unknown))),
                        "Result" => RTy::Res(
                            Box::new(inner.get(0).cloned().unwrap_or(RTy::Unit)),
                            Box::new(inner.get(1).cloned().unwrap_or(RTy::Unit)),
                        ),
                        _ => RTy::Named(name),
                    };
                }
                match name.as_str() {
                    "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize" => RTy::I64,
                    "f32" | "f64" => RTy::F64,
                    "bool" => RTy::Bool,
                    "char" | "String" => RTy::Str,
                    _ => RTy::Named(name),
                }
            }
            Type::Ref { ty, .. } => self.ty_of(ty),
            Type::Tuple(ts) => {
                if ts.is_empty() { RTy::Unit } else { self.ty_of(&ts[0]) }
            }
            Type::Fn { params, ret } => RTy::FnPtr(
                params.iter().map(|t| self.ty_of(t)).collect(),
                Box::new(ret.as_ref().map(|t| self.ty_of(t)).unwrap_or(RTy::Unit)),
            ),
            Type::Infer => RTy::Unknown,
        }
    }

    fn ty_of_opt(&self, t: &Option<Type>) -> RTy {
        t.as_ref().map(|t| self.ty_of(t)).unwrap_or(RTy::Unknown)
    }

    fn var_ty(&self, name: &str) -> RTy {
        for (n, t) in self.vars.iter().rev() {
            if n == name {
                return t.clone();
            }
        }
        if let Some(r) = self.fn_ret.get(name) {
            if !matches!(r, RTy::Unit) {
                return r.clone();
            }
        }
        // cli_args 注入全局: Vec<String>
        if name == "cli_args" { return RTy::Vec(Box::new(RTy::Str)); }
        // @global 回退(局部遮蔽全局)
        if let Some(t) = self.globals.get(name) {
            return t.clone();
        }
        RTy::Unknown
    }

    fn set_var(&mut self, name: &str, ty: RTy) {
        self.vars.push((name.to_string(), ty));
    }

    // 变体名反查: 变体名 -> (枚举名, 载荷数)
    fn variant_of(&self, name: &str) -> Option<(String, usize)> {
        for (en, vs) in &self.enums {
            for (vn, ar) in vs {
                if vn == name { return Some((en.clone(), *ar)); }
            }
        }
        None
    }

    // 裸标识符发射: cli_args 全局注入 + 零载荷变体作值
    fn ident_emit(&self, name: &str) -> String {
        if name == "cli_args" { return "__cli_args()".into(); }
        // @global: 经互斥守卫解引用位(读/写/方法变异/字段统一形态)
        if self.globals.contains_key(name) {
            return format!("(*__g_{}())", esc_name(name));
        }
        if let Some((en, ar)) = self.variant_of(name) {
            if ar == 0 { return format!("{}::{}", en, esc_name(name)); }
        }
        esc_name(name)
    }

    // 局部类型定型: 内层 Unknown 以 i64 具体化(Rust 拒绝 Result<_> 等占位)
    fn concrete(&self, t: &RTy) -> RTy {
        match t {
            RTy::Unknown => RTy::I64,
            RTy::Vec(v) => RTy::Vec(Box::new(self.concrete(v))),
            RTy::Opt(v) => RTy::Opt(Box::new(self.concrete(v))),
            RTy::Res(a, b) => RTy::Res(Box::new(self.concrete(a)), Box::new(self.concrete(b))),
            RTy::BoxedT(v) => RTy::BoxedT(Box::new(self.concrete(v))),
            other => other.clone(),
        }
    }

    // 内层 Unknown 检测(仅容器构造内部的占位;顶层 Unknown 不算)
    fn has_inner_unknown(t: &RTy) -> bool {
        match t {
            RTy::Unknown => true,
            RTy::Vec(v) | RTy::Opt(v) => Emitter::has_inner_unknown(v),
            RTy::Res(a, b) => Emitter::has_inner_unknown(a) || Emitter::has_inner_unknown(b),
            RTy::BoxedT(v) => Emitter::has_inner_unknown(v),
            RTy::FnPtr(ps, r) => ps.iter().any(|p| Emitter::has_inner_unknown(p)) || Emitter::has_inner_unknown(r),
            _ => false,
        }
    }

    fn clone_if_container(&mut self, e: &Expr, t: &RTy) -> String {
        let s = self.expr(e);
        if t.is_container() || matches!(t, RTy::Str) {
            if matches!(e, Expr::Ident { .. }) {
                return format!("{}.clone()", s);
            }
        }
        s
    }

    pub fn emit_program(&mut self, p: &Program) -> String {
        self.collect(p);
        let mut out = String::from(PRELUDE);
        out.push_str("\n// ---- 用户代码 ----\n");
        for item in &p.items {
            match item {
                Item::Struct(sd) => out.push_str(&self.emit_struct(sd)),
                Item::Enum(ed) => out.push_str(&self.emit_enum(ed)),
                _ => {}
            }
        }
        // @global: 类型推断(签名已齐) -> 注册表 -> 访问器发射(W54-c)
        let defs = self.global_defs.clone();
        let mut g_acc = String::new();
        for g in &defs {
            let gt = match &g.ty {
                Some(t) => {
                    let t0 = self.ty_of(t);
                    self.concrete(&t0)
                }
                None => match &g.init {
                    Some(e) => {
                        let t0 = self.infer(e);
                        self.concrete(&t0)
                    }
                    None => RTy::Unit,
                },
            };
            self.globals.insert(g.name.clone(), gt.clone());
        }
        for g in &defs {
            let gt = self.globals.get(&g.name).cloned().unwrap_or(RTy::Unit);
            let n = esc_name(&g.name);
            let tstr = gt.rust_str();
            let init_s = match &g.init {
                Some(e) => {
                    let t0 = self.infer(e);
                    self.clone_if_container(e, &t0)
                }
                None => default_value(&gt),
            };
            g_acc.push_str("fn __g_");
            g_acc.push_str(&n);
            g_acc.push_str("() -> std::sync::MutexGuard<'static, ");
            g_acc.push_str(&tstr);
            g_acc.push_str("> {\n    static __S_");
            g_acc.push_str(&n);
            g_acc.push_str(": std::sync::OnceLock<std::sync::Mutex<");
            g_acc.push_str(&tstr);
            g_acc.push_str(">> = std::sync::OnceLock::new();\n    __S_");
            g_acc.push_str(&n);
            g_acc.push_str(".get_or_init(|| std::sync::Mutex::new(");
            g_acc.push_str(&init_s);
            g_acc.push_str(")).lock().unwrap()\n}\n\n");
        }
        out.push_str(&g_acc);
        for item in &p.items {
            match item {
                Item::Fn(f) => out.push_str(&self.emit_fn(f)),
                _ => {}
            }
        }
        out
    }

    fn emit_struct(&self, sd: &StructDef) -> String {
        let mut s = format!("#[derive(Clone, PartialEq, Debug)]\nstruct {} {{\n", sd.name);
        for f in &sd.fields {
            let ft = self.ty_of(&f.ty);
            let ft = if matches!(ft, RTy::Unknown) { RTy::I64 } else { ft };
            s.push_str(&format!("    {}: {},\n", f.name, ft.rust_str()));
        }
        s.push_str("}\n\n");
        // impl Default: ::default() 构造位(alinterp Stmt::default 等)
        s.push_str(&format!("impl Default for {} {{\n    fn default() -> Self {{ {} {{\n", sd.name, sd.name));
        for f in &sd.fields {
            let ft = self.ty_of(&f.ty);
            s.push_str(&format!("            {}: {},\n", f.name, default_value(&ft)));
        }
        s.push_str("        }\n    }\n}\n\n");
        s
    }

    fn emit_enum(&self, ed: &EnumDef) -> String {
        let mut s = format!("#[derive(Clone, PartialEq, Debug)]\nenum {} {{\n", ed.name);
        for v in &ed.variants {
            if v.payload.is_empty() {
                s.push_str(&format!("    {},\n", v.name));
            } else {
                // 递归直边载荷: Box 间接(W54-a)
                let parts: Vec<String> = v.payload.iter().map(|t| {
                    let rt = self.ty_of(t);
                    match &rt {
                        RTy::Named(n) if self.is_boxed_edge(&ed.name, n) => format!("Box<{}>", n.clone()),
                        _ => rt.rust_str(),
                    }
                }).collect();
                s.push_str(&format!("    {}({}),\n", v.name, parts.join(", ")));
            }
        }
        s.push_str("}\n\n");
        // impl Default: 首变体 + 载荷默认值(Box 载荷槽需包装)
        let dparts: Vec<String> = ed.variants.first().map(|v0| {
            v0.payload.iter().map(|t| {
                let rt = self.ty_of(t);
                match &rt {
                    RTy::Named(n2) if self.is_boxed_edge(&ed.name, n2) => format!("Box::new({})", default_value(&rt)),
                    RTy::BoxedT(_) => format!("Box::new({})", default_value(&rt)),
                    _ => default_value(&rt),
                }
            }).collect()
        }).unwrap_or_default();
        let dcall = match ed.variants.first() {
            Some(v0) if v0.payload.is_empty() => format!("{}::{}", ed.name, v0.name),
            Some(v0) => format!("{}::{}({})", ed.name, v0.name, dparts.join(", ")),
            None => format!("{}::EUnit", ed.name),
        };
        s.push_str(&format!("impl Default for {} {{\n    fn default() -> Self {{ {} }}\n}}\n\n", ed.name, dcall));
        s
    }

    fn emit_fn(&mut self, f: &FnDef) -> String {
        let name = esc_name(&f.sig.name);
        self.cur_is_main = f.sig.name == "main";
        self.cur_ret = f.sig.ret.as_ref().map(|t| self.ty_of(t)).unwrap_or(RTy::Unit);
        self.vars.clear();
        // W54-e: 被当函数调用的形参 -> Box<dyn Fn>(宿主: 形参注解可为谎言)
        let pnames: Vec<String> = f.sig.params.iter().map(|p| p.name.clone()).collect();
        let mut calls: Vec<(String, Vec<Expr>)> = Vec::new();
        self.scan_called_params_expr(&Expr::Block(Box::new(f.body.clone())), &pnames, &mut calls);
        let params: Vec<String> = f
            .sig
            .params
            .iter()
            .map(|p| {
                let t = { let t0 = self.ty_of_opt(&p.ty); if matches!(t0, RTy::Unknown) { RTy::I64 } else { t0 } };
                // 诚实函数型注解直用 fn 指针(fn item 调用位自动强转)
                if matches!(&t, RTy::FnPtr(_, _)) {
                    self.set_var(&p.name, t.clone());
                    return format!("mut {}: {}", esc_name(&p.name), t.rust_str());
                }
                // 谎言兜底仅对原始型注解生效(容器/枚举注解诚实, 恒按注解)
                let primitive_lie = matches!(t, RTy::I64 | RTy::F64 | RTy::Bool | RTy::Str);
                let call_hit = calls.iter().find(|(cn, _)| *cn == p.name).map(|(_, cargs)| cargs.clone());
                if primitive_lie && call_hit.is_some() {
                    let cargs = call_hit.unwrap();
                    // 首个调用位的实参类型 -> 动态 Fn 签名; 返回值取外层函数返回型
                    let ats: Vec<String> = cargs.iter().map(|a| {
                        let t0 = self.infer(a);
                        let t0 = if matches!(t0, RTy::Unknown) { RTy::I64 } else { t0.unboxed() };
                        t0.rust_str()
                    }).collect();
                    let ret_t = match &self.cur_ret { r if !matches!(r, RTy::Unit) => r.rust_str(), _ => "i64".to_string() };
                    let fty = format!("Box<dyn Fn({}) -> {}>", ats.join(", "), ret_t);
                    self.set_var(&p.name, RTy::Named(fty.clone()));
                    return format!("mut {}: {}", esc_name(&p.name), fty);
                }
                self.set_var(&p.name, t.clone());
                format!("mut {}: {}", esc_name(&p.name), t.rust_str())
            })
            .collect();
        let ret = match &f.sig.ret {
            Some(t) => {
                let rt = self.ty_of(t);
                let rt = if matches!(rt, RTy::Unknown) { RTy::I64 } else { rt };
                self.cur_ret = rt.clone();
                format!(" -> {}", rt.rust_str())
            }
            None => String::new(),
        };
        if f.sig.name == "main" && !matches!(self.cur_ret, RTy::Unit) {
            // 宿主 CLI 行为: main 返回值(如 Result)会被打印
            let mut out = format!("fn __aine_main(){} {{\n", ret);
            out.push_str(&self.block(&f.body, 1, BlockMode::FnBody));
            // 主包装内同样补缺省尾(该分支提前 return, 走不到通用插入点)
            if f.body.tail.is_none() && !matches!(f.body.stmts.last(), Some(Stmt::Return(_))) {
                out.push_str("    return ");
                out.push_str(&default_value(&self.cur_ret));
                out.push_str(";\n");
            }
            out.push_str("}\n\n");
            // 宿主 CLI: 仅当 main 有显式非 Unit 返回才打印(裸落空 = Unit 不打印)
            if self.body_has_nontrivial_return(&f.body) {
                out.push_str("fn main() {\n    println!(\"{}\", __aine_main().__disp());\n}\n\n");
            } else {
                out.push_str("fn main() {\n    __aine_main();\n}\n\n");
            }
            return out;
        }
        let mut out = format!("fn {}({}){} {{\n", name, params.join(", "), ret);
        out.push_str(&self.block(&f.body, 1, BlockMode::FnBody));
        // 无尾即补缺省: return 先行的路径永不触达(不可达告警可接受), 落空路径对齐类型要求
        if !matches!(self.cur_ret, RTy::Unit)
            && f.body.tail.is_none()
            && !matches!(f.body.stmts.last(), Some(Stmt::Return(_)))
        {
            // 无尾且末句非 return: 补缺省尾(lift-return 形式的块尾已自带返回)
            out.push_str("    return ");
            out.push_str(&default_value(&self.cur_ret));
            out.push_str(";\n");
        }
        out.push_str("}\n\n");
        out
    }

    // 块尾模式: Stmt=语句位(恒分号) FnBody=函数体(cur_ret 决定) Expr=表达式位(infer 决定)
    fn block(&mut self, b: &Block, depth: usize, mode: BlockMode) -> String {
        let mut out = String::new();
        let ind = "    ".repeat(depth);
        // 尾部 return 语句提升: return X 作为块尾表达式(never)在语句/表达式位都合法
        let mut lifted_ret: Option<&Stmt> = None;
        let mut lifted_if: Option<&IfExpr> = None;
        let stmts: &[Stmt] = if b.tail.is_none() {
            match b.stmts.last() {
                Some(s @ Stmt::Return(_)) => {
                    lifted_ret = Some(s);
                    &b.stmts[..b.stmts.len() - 1]
                }
                Some(Stmt::If(i)) if mode == BlockMode::Expr => {
                    // 值位块尾的 if: 提升为 if 表达式(臂型统一需要)
                    lifted_if = Some(i);
                    &b.stmts[..b.stmts.len() - 1]
                }
                _ => &b.stmts[..],
            }
        } else {
            &b.stmts[..]
        };
        for st in stmts {
            out.push_str(&ind);
            out.push_str(&self.stmt(st, depth));
            out.push('\n');
        }
        if let Some(Stmt::Return(re)) = lifted_ret {
            out.push_str(&ind);
            match re {
                Some(e) => {
                    let t = self.infer(e);
                    let rs = if self.is_fresh_expr(e) {
                        self.clone_if_container(e, &t)
                    } else {
                        format!("{}.clone()", self.clone_if_container(e, &t))
                    };
                    out.push_str(&format!("return {}", rs));
                }
                None => out.push_str("return"),
            }
            out.push('\n');
        }
        if let Some(i) = lifted_if {
            out.push_str(&ind);
            out.push_str(&self.if_expr(i));
            out.push('\n');
        }
        if let Some(t) = &b.tail {
            out.push_str(&ind);
            // Stmt 位块尾的 match = 语句位 match(臂值归 (),防臂型不统一)
            let ts = match (&**t, mode) {
                (Expr::Match { scrutinee, arms, .. }, BlockMode::Stmt) => {
                    self.match_expr(scrutinee, arms, true)
                }
                _ => self.expr(t),
            };
            let semi = match mode {
                BlockMode::Stmt => true,
                BlockMode::FnBody => matches!(self.cur_ret, RTy::Unit),
                BlockMode::Expr => matches!(self.infer(t), RTy::Unit),
            };
            if semi {
                out.push_str(&format!("{};\n", ts));
            } else {
                out.push_str(&ts);
                out.push('\n');
            }
        }
        out
    }

    fn stmt(&mut self, st: &Stmt, _depth: usize) -> String {
        match st {
            Stmt::Let(l) => {
                let name = match &l.pat {
                    Pattern::Ident { name, .. } => esc_name(name),
                    _ => "_".into(),
                };
                let ann = self.ty_of_opt(&l.ty);
                match &l.init {
                    Some(e) => {
                        let it0 = self.infer(e);
                        let it = self.concrete(&it0);
                        // 变量表登记: 注解优先(空数组字面量的推断会具体化成错误类型)
                        if matches!(ann, RTy::Unknown) {
                            self.set_var(&name, it.clone());
                        } else {
                            self.set_var(&name, ann.clone());
                        }
                        let rhs = if matches!(&**e, Expr::Closure { .. }) || self.is_fresh_expr(e) {
                            self.clone_if_container(e, &it)
                        } else {
                            // 值语义: let 初始化 = 拷贝(仅别名可能时; 新造临时免拷)
                            format!("{}.clone()", self.clone_if_container(e, &it))
                        };
                        let tstr = if matches!(ann, RTy::Unknown) {
                            it.rust_str()
                        } else {
                            ann.rust_str()
                        };
                        // 宿主语义: let 绑定的容器也可原位变异(push/set),Rust 侧需 mut
                        let needs_mut = l.is_mut || it.is_container();
                        let kw = if needs_mut { "let mut" } else { "let" };
                        // 仅当容器内层有 Unknown(如 Result<_, String>)时注解,其余交给 rustc 推断
                        // 仅 Opt/Res 构造初始化需要注解(Ok/Err/None 的载荷类型无约束点);
                        // 其余容器推断交给 rustc 从使用点推断(注解反而可能与之冲突)
                        if (matches!(it0, RTy::Opt(_)) || matches!(it0, RTy::Res(_, _))) && Emitter::has_inner_unknown(&it0) {
                            format!("{} {}: {} = {};", kw, name, it.rust_str(), rhs)
                        } else if matches!(ann, RTy::Unknown) {
                            format!("{} {} = {};", kw, name, rhs)
                        } else {
                            format!("{} {}: {} = {};", kw, name, tstr, rhs)
                        }
                    }
                    None => {
                        let tstr = if matches!(ann, RTy::Unknown) { "_".into() } else { ann.rust_str() };
                        self.set_var(&name, ann.clone());
                        format!(
                            "{} {}: {} = {};",
                            if l.is_mut { "let mut" } else { "let" },
                            name,
                            tstr,
                            default_value(&ann)
                        )
                    }
                }
            }
            // 表达式语句: match 走语句位(臂值归 (),防臂型不统一)
            Stmt::Expr(e) => {
                if let Expr::Match { scrutinee, arms, .. } = &**e {
                    return format!("{};", self.match_expr(scrutinee, arms, true));
                }
                format!("{};", self.expr(e))
            }
            // 表达式语句一律补分号(println! 宏不带分号也合法,带分号更稳)
            Stmt::Expr(e) => format!("{};", self.expr(e)),
            Stmt::Return(e) => match e {
                Some(e) => {
                    let t = self.infer(e);
                    format!("return {};", self.clone_if_container(e, &t))
                }
                None => "return;".into(),
            },
            // 语句位 if 恒补分号(块尾无分号 if 会被 Rust 当尾表达式, 与非Unit返回型冲突)
            Stmt::If(i) => format!("{};", self.emit_if(i, 0)),
            Stmt::While { cond, body, .. } => {
                self.loop_depth += 1;
                // while true 在新 rustc 中被拒,发射 loop
                let head = if matches!(&**cond, Expr::Bool(true)) { "loop".to_string() } else { format!("while {}", self.expr(cond)) };
                let s = format!("{} {{\n{}}}", head, self.block(body, 1, BlockMode::Stmt));
                self.loop_depth -= 1;
                s
            }
            Stmt::For { pat, iter, body, .. } => {
                let name = match pat {
                    Pattern::Ident { name, .. } => esc_name(name),
                    _ => "_".into(),
                };
                let it_ty = self.infer(iter);
                eprintln!("[dbg] for-var {} it_ty={:?} iter={:?}", name, &it_ty, iter);
                let saved = self.vars.clone();
                let it_s = self.expr(iter);
                let head = match &it_ty {
                    RTy::Str => format!("for {} in {}.chars().map(|__c| __c.to_string())", name, it_s),
                    RTy::Vec(_) => format!("for {} in {}.iter().cloned()", name, it_s),
                    _ => format!("for {} in ({}).clone()", name, it_s),
                };
                if let RTy::Vec(t) = &it_ty {
                    self.set_var(&name, (**t).clone());
                } else if matches!(it_ty, RTy::Str) {
                    self.set_var(&name, RTy::Str);
                } else {
                    self.set_var(&name, RTy::I64);
                }
                self.loop_depth += 1;
                let body_s = self.block(body, 1, BlockMode::Stmt);
                self.loop_depth -= 1;
                self.vars = saved;
                format!("{} {{\n{}}}", head, body_s)
            }
            // 宿主语义: 循环外的 break 终止当前函数
            Stmt::Break(_) => if self.loop_depth > 0 { "break;".into() } else { "return;".into() },
            Stmt::Continue(_) => if self.loop_depth > 0 { "continue;".into() } else { "return;".into() },
            Stmt::Defer(_) => "/* defer: PoC 未覆盖 */".into(),
            Stmt::TryCatch(t) => self.block(&t.try_block, 0, BlockMode::Stmt),
            Stmt::Block(b) => self.block(b, 0, BlockMode::Stmt),
        }
    }

    fn emit_if(&mut self, i: &IfExpr, depth: usize) -> String {
        let ind = "    ".repeat(depth);
        let mut out = format!("if {} {{\n", self.expr(&i.cond));
        out.push_str(&self.block(&i.then_branch, depth + 1, BlockMode::Stmt));
        out.push_str(&ind);
        if let Some(e) = &i.else_branch {
            match &**e {
                Expr::Block(b) => {
                    out.push_str("} else {\n");
                    out.push_str(&self.block(b, depth + 1, BlockMode::Stmt));
                    out.push_str(&ind);
                    out.push_str("}\n");
                }
                Expr::If(inner) => {
                    out.push_str("} else ");
                    out.push_str(&self.emit_if(inner, depth).trim_start());
                }
                other => {
                    out.push_str("} else {\n");
                    out.push_str(&format!("{}    {};\n", ind, self.expr(other)));
                    out.push_str(&ind);
                    out.push_str("}\n");
                }
            }
        } else {
            out.push_str("}\n");
        }
        out
    }

    fn infer(&mut self, e: &Expr) -> RTy {
        match e {
            Expr::Int(_) => RTy::I64,
            Expr::Float(_) => RTy::F64,
            Expr::Bool(_) => RTy::Bool,
            Expr::Str(_) | Expr::FmtStr(_) => RTy::Str,
            Expr::Ident { name, .. } => {
                if let Some((en, ar)) = self.variant_of(name) {
                    if ar == 0 { return RTy::Named(en); }
                }
                self.var_ty(name)
            }
            Expr::Path(p) => self.var_ty(&p.segments.join("::")),
            Expr::Array(items) => RTy::Vec(Box::new(
                items.first().map(|x| self.infer(x)).unwrap_or(RTy::Unknown),
            )),
            Expr::StructLit { name, .. } => RTy::Named(name.segments.join("::")),
            Expr::Unary { op: UnOp::Neg, expr } => {
                let t = self.infer(expr);
                if matches!(t, RTy::F64) { RTy::F64 } else { RTy::I64 }
            }
            Expr::Unary { expr, .. } => self.infer(expr),
            Expr::Binary { op, lhs, rhs } => {
                let lt = self.infer(lhs);
                let rt = self.infer(rhs);
                match op {
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                    | BinOp::And | BinOp::Or => RTy::Bool,
                    BinOp::Add if matches!(lt, RTy::Str) || matches!(rt, RTy::Str) => RTy::Str,
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        if matches!(lt, RTy::F64) || matches!(rt, RTy::F64) { RTy::F64 } else { RTy::I64 }
                    }
                    _ => RTy::I64,
                }
            }
            Expr::Assign { value, .. } => self.infer(value),
            Expr::Call { callee, args, .. } => {
                // Map.new() 构造表达式
                if let Expr::Field { base, field, .. } = &**callee {
                    if field == "new" {
                        if let Expr::Ident { name, .. } = &**base {
                            if name == "Map" {
                                return RTy::Map(Box::new(RTy::Unknown));
                            }
                        }
                    }
                }
                // 方法链推断(宿主迭代器管道): iter 透传, map 取闭包体类型, sum 恒 i64
                if let Expr::Field { base, field, .. } = &**callee {
                    match field.as_str() {
                        "iter" | "filter" | "cloned" | "collect" => return self.infer(base),
                        // Map 取值族: 按接收者载荷类型回填(get -> Opt(elem), 模式绑定下游类型自洽)
                        "get" => {
                            return match self.infer(base) {
                                RTy::Map(v) => RTy::Opt(Box::new((*v).clone())),
                                RTy::Vec(e) => RTy::Opt(Box::new((*e).clone())),
                                _ => RTy::Unknown,
                            };
                        }
                        "set" | "insert" => {
                            return match self.infer(base) {
                                RTy::Map(v) => *v,
                                _ => RTy::Unknown,
                            };
                        }
                        "remove" | "pop" => {
                            return match self.infer(base) {
                                RTy::Map(v) => RTy::Opt(Box::new((*v).clone())),
                                RTy::Vec(e) => RTy::Opt(Box::new((*e).clone())),
                                _ => RTy::Unknown,
                            };
                        }
                        "to_i32" | "to_i64" | "len" | "codepoint_at" => return RTy::I64,
                        "to_f64" => return RTy::F64,
                        "trim" | "replace" => return RTy::Str,
                        "split" => return RTy::Vec(Box::new(RTy::Str)),
                        "contains" | "starts_with" | "ends_with" => return RTy::Bool,
                        "keys" => return RTy::Vec(Box::new(RTy::Str)),
                        "values" => return self.infer(base),
                        "map" => {
                            for a in args { self.infer(&a.expr); }
                            let base_t = self.infer(base);
                            let base_elem = match &base_t {
                                RTy::Vec(e) => (**e).clone(),
                                _ => RTy::Unknown,
                            };
                            let elem = match args.first() {
                                Some(a) => match &a.expr {
                                    Expr::Closure { body, .. } => { let t = self.infer(body); if matches!(t, RTy::Unknown) { base_elem } else { t } }
                                    _ => base_elem,
                                },
                                None => base_elem,
                            };
                            return RTy::Vec(Box::new(elem));
                        }
                        "sum" => {
                            for a in args { self.infer(&a.expr); }
                            return match self.infer(base) {
                                RTy::Vec(e) if matches!(*e, RTy::F64) => RTy::F64,
                                _ => RTy::I64,
                            };
                        }
                        _ => {}
                    }
                }
                let name = match &**callee {
                    Expr::Ident { name, .. } => Some(name.clone()),
                    _ => None,
                };
                if let Some(n) = &name {
                    if n == "read_file" || n == "input" { return RTy::Str; }
                    if n == "read_bytes" || n == "base64_decode" || n == "to_bytes" { return RTy::Vec(Box::new(RTy::I64)); }
                    if n == "base64_encode" { return RTy::Str; }
                    if n == "write_file" || n == "append_file" || n == "write_bytes" || n == "file_exists" { return RTy::Bool; }
                    if n == "read_line" { return RTy::Str; }
                    if n == "now" || n == "now_ms" { return RTy::I64; }
                    if n == "file_list" { return RTy::Vec(Box::new(RTy::Str)); }
                    if n == "sleep" || n == "print" { return RTy::Unit; }
                }
                if let Some(n) = &name {
                    if let Some((en, _)) = self.variant_of(n) {
                        return RTy::Named(en);
                    }
                    if n == "None" { return RTy::Opt(Box::new(RTy::Unknown)); }
                    if n == "Some" {
                        let t = args.first().map(|a| self.infer(&a.expr)).unwrap_or(RTy::Unknown);
                        return RTy::Opt(Box::new(t));
                    }
                    if n == "Ok" {
                        let t = args.first().map(|a| self.infer(&a.expr)).unwrap_or(RTy::Unknown);
                        return RTy::Res(Box::new(t), Box::new(RTy::Unknown));
                    }
                    if n == "Err" {
                        let t = args.first().map(|a| self.infer(&a.expr)).unwrap_or(RTy::Unknown);
                        return RTy::Res(Box::new(RTy::Unknown), Box::new(t));
                    }
                }
                for a in args {
                    self.infer(&a.expr);
                }
                match name {
                    Some(n) if n == "print" => RTy::Unit,
                    Some(n) => self.fn_ret.get(&n).cloned().unwrap_or(RTy::Unknown),
                    _ => RTy::Unknown,
                }
            }
            Expr::Field { base, field, .. } => {
                let bt = self.infer(base);
                if field == "len" {
                    return RTy::I64;
                }
                match bt {
                    RTy::Named(sn) => {
                        if let Some(fields) = self.structs.get(&sn).cloned() {
                            for (fn2, ft) in fields {
                                if fn2 == *field {
                                    return ft;
                                }
                            }
                        }
                        RTy::Unknown
                    }
                    _ => {
                        // 闭包形参无类型: 程序中唯一持有该字段名的结构体兜底
                        let mut hits: Vec<RTy> = Vec::new();
                        for (_sn, fields) in &self.structs {
                            for (fn2, ft) in fields {
                                if fn2 == field { hits.push(ft.clone()); }
                            }
                        }
                        if hits.len() == 1 { return hits.pop().unwrap(); }
                        RTy::Unknown
                    }
                }
            }
            Expr::Index { base, .. } => {
                let bt = self.infer(base);
                match bt {
                    RTy::Vec(t) => *t,
                    RTy::Map(v) => *v,
                    RTy::Str => RTy::Str,
                    _ => RTy::Unknown,
                }
            }
            Expr::Try(x) => {
                let t = self.infer(x);
                match t {
                    RTy::Opt(t) => *t,
                    RTy::Res(a, _) => *a,
                    _ => RTy::Unknown,
                }
            }
            Expr::Range { .. } => RTy::I64,
            Expr::Block(b) => {
                if b.tail.is_none() && matches!(b.stmts.last(), Some(Stmt::Return(_))) {
                    // 全 return 块 = never 位(非 Unit,防尾分号)
                    RTy::Unknown
                } else if b.tail.is_none() {
                    match b.stmts.last() {
                        // 尾随 If 提升为值位: 用 If 值型(分歧让位)
                        Some(Stmt::If(i)) => self.infer(&Expr::If(Box::new((**i).clone()))),
                        _ => RTy::Unit,
                    }
                } else {
                    b.tail.as_ref().map(|t| self.infer(t)).unwrap_or(RTy::Unit)
                }
            }
            Expr::If(i) => {
                // 分歧侧让位: else 为全 return 块(never)时取 then 型,反之亦然
                let tt = i
                    .then_branch
                    .tail
                    .as_ref()
                    .map(|t| self.infer(t))
                    .unwrap_or(RTy::Unit);
                match &i.else_branch {
                    Some(e) => {
                        let et = self.infer(e);
                        match (&tt, &et) {
                            (_, RTy::Unknown) => tt,
                            (RTy::Unknown, _) => et,
                            _ => et,
                        }
                    }
                    None => tt,
                }
            }
            Expr::Match { arms, .. } => arms
                .first()
                .map(|a| self.infer(&a.value))
                .unwrap_or(RTy::Unknown),
            _ => RTy::Unknown,
        }
    }

    fn expr(&mut self, e: &Expr) -> String {
        match e {
            Expr::Int(n) => format!("{}i64", n),
            Expr::Float(f) => format!("{}f64", f),
            Expr::Bool(b) => format!("{}", b),
            Expr::Str(s) => format!("{}.to_string()", rust_str_lit(s)),
            Expr::FmtStr(s) => self.fstr(s),
            Expr::Ident { name, .. } => {
                let n = self.ident_emit(name);
                let t = self.var_ty(name);
                match t {
                    // 递归指针绑定: 宿主读 = T 值拷贝
                    RTy::BoxedT(_) => format!("(*{}).clone()", n),
                    t2 if t2.is_container() && !matches!(t2, RTy::Str) => format!("{}.clone()", n),
                    _ => n,
                }
            }
            Expr::Path(p) => esc_name(&p.segments.join("::")),
            Expr::Array(items) => {
                if items.is_empty() {
                    return "Vec::new()".into();
                }
                let parts: Vec<String> = items.iter().map(|x| self.expr(x)).collect();
                format!("vec![{}]", parts.join(", "))
            }
            Expr::StructLit { name, fields } => {
                let mut s = format!("{} {{", name.segments.join("::"));
                let mut provided: Vec<String> = Vec::new();
                for (fn2, fe) in fields {
                    // 值语义: 字段初始化 = 拷贝(防源绑定被 move)
                    s.push_str(&format!(" {}: {}.clone(),", fn2, self.expr(fe)));
                    provided.push(fn2.clone());
                }
                // 宿主语义: 省略字段 = 缺省值(Nil 对应空默认)
                let sn = name.segments.join("::");
                if let Some(declared) = self.structs.get(&sn).cloned() {
                    for (dfn, dft) in declared {
                        if !provided.contains(&dfn) {
                            s.push_str(&format!(" {}: {},", dfn, default_value(&dft)));
                        }
                    }
                }
                s.push_str(" }");
                s
            }
            Expr::Unary { op: UnOp::Neg, expr } => {
                if matches!(self.infer(expr), RTy::F64) { format!("(-{})", self.expr(expr)) }
                else { format!("__neg({})", self.expr(expr)) }
            }
            Expr::Unary { op: UnOp::Not, expr } => format!("!({})", self.expr(expr)),
            Expr::Unary { op: UnOp::Ref, expr } => format!("&({})", self.expr(expr)),
            Expr::Unary { op: UnOp::Deref, expr } => format!("*({})", self.expr(expr)),
            Expr::Binary { op, lhs, rhs } => self.binop(op, lhs, rhs),
            Expr::Assign { op, target, value } => self.assign_emit(op, target, value),
            Expr::Call { callee, args, .. } => self.call(callee, args),
            Expr::Field { base, field, .. } => {
                let bt = self.infer(base);
                if field == "len" {
                    return self.method_call(base, "len", &[]);
                }
                if let RTy::Named(sn) = &bt {
                    if self.structs.contains_key(sn) {
                        return format!("{}.{}.clone()", self.expr(base), field);
                    }
                }
                format!("{}.{}", self.expr(base), field)
            }
            Expr::Index { base, index } => {
                let bt = self.infer(base);
                let s = self.index_emit(base, index);
                match bt {
                    // 容器元素读 = 拷贝(值语义); Map/Str 助手已返回所有值
                    RTy::Map(_) | RTy::Str => s,
                    _ => format!("{}.clone()", s),
                }
            }
            Expr::Try(x) => {
                // 宿主: 失败向上传播,未捕获则 CLI 报错退出; 发射为解包助手(失败 stderr+exit)
                let t = self.infer(x);
                let s = self.expr(x);
                match t {
                    RTy::Opt(_) => format!("__try_opt({})", s),
                    _ => format!("__try_res({})", s),
                }
            }
            Expr::Range { start, end } => {
                let a = start.as_ref().map(|x| self.expr(x)).unwrap_or_default();
                let b = end.as_ref().map(|x| self.expr(x)).unwrap_or_default();
                format!("{}..{}", a, b)
            }
            Expr::Block(b) => {
                let inner = self.block(b, 0, BlockMode::Expr);
                format!("{{ {} }}", inner.trim_end())
            }
            Expr::If(i) => self.if_expr(i),
            Expr::Match { scrutinee, arms, .. } => self.match_expr(scrutinee, arms, false),
            Expr::Closure { params, body, .. } => {
                let ps: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if let Some(t) = &p.ty {
                            format!("{}: {}", esc_name(&p.name), self.ty_of(t).rust_str())
                        } else {
                            esc_name(&p.name)
                        }
                    })
                    .collect();
                format!("move |{}| {}", ps.join(", "), self.expr(body))
            }
            Expr::Go { .. } => "/* go: 占位 */".into(),
            Expr::UiOp { .. } => "/* ui: 占位 */".into(),
            Expr::Paren(x) => format!("({})", self.expr(x)),
            Expr::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(|x| self.expr(x)).collect();
                format!("({})", parts.join(", "))
            }
        }
    }

    fn binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) -> String {
        let lt = self.infer(lhs);
        match op {
            BinOp::Add if matches!(lt, RTy::Str) => {
                let a = self.expr(lhs);
                let b = self.expr(rhs);
                return format!("__strcat(&{}, &{})", a, b);
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                let l_is_f = matches!(lt, RTy::F64);
                let r_is_f = matches!(self.infer(rhs), RTy::F64);
                if l_is_f || r_is_f {
                    // 混合 int/f64: int 侧 as f64(宿主 f64 吸收) — 含 Div/Rem(f64 除法)
                    let le = if l_is_f { self.expr(lhs) } else { format!("({}) as f64", self.expr(lhs)) };
                    let re = if r_is_f { self.expr(rhs) } else { format!("({}) as f64", self.expr(rhs)) };
                    return format!("{} {} {}", le, binop_str(op), re);
                }
                let op_s = match op { BinOp::Add => "__add", BinOp::Sub => "__sub", BinOp::Mul => "__mul", BinOp::Div => "__div", _ => "__rem" };
                return format!("{}({}, {})", op_s, self.expr(lhs), self.expr(rhs));
            }
            BinOp::Shl => return format!("__shl({}, {})", self.expr(lhs), self.expr(rhs)),
            BinOp::Shr => return format!("__shr({}, {})", self.expr(lhs), self.expr(rhs)),
            _ => {}
        }
        format!("{} {} {}", self.expr(lhs), binop_str(op), self.expr(rhs))
    }

    fn assign_emit(&mut self, op: &AssignOp, target: &Expr, value: &Expr) -> String {
        let _ = op;
        let ts = self.expr_no_clone(target);
        match op {
            AssignOp::Assign => {
                // Map 索引赋值: m[k] = v -> insert
                if let Expr::Index { base, index } = target {
                    if matches!(self.infer(base), RTy::Map(_)) {
                        let bs = self.expr_no_clone(base);
                        let ks = self.expr(index);
                        let vs = self.expr(value);
                        return format!("{}.insert({}, {})", bs, ks, vs);
                    }
                }
                let t = self.infer(target);
                let vs = {
                    let vt = self.infer(value);
                    if vt.is_container() && !matches!(vt, RTy::Str) && matches!(value, Expr::Ident { .. }) {
                        format!("{}.clone()", self.expr(value))
                    } else if matches!(value, Expr::Closure { .. }) || self.is_fresh_expr(value) {
                        self.expr(value)
                    } else {
                        // 值语义: 赋值 = 拷贝(仅别名可能时; 新造临时免拷)
                        format!("{}.clone()", self.expr(value))
                    }
                };
                format!("{} = {}", ts, vs)
            }
            AssignOp::AddAssign | AssignOp::SubAssign | AssignOp::MulAssign | AssignOp::DivAssign | AssignOp::RemAssign => {
                let bin_op = match op {
                    AssignOp::AddAssign => BinOp::Add,
                    AssignOp::SubAssign => BinOp::Sub,
                    AssignOp::MulAssign => BinOp::Mul,
                    AssignOp::DivAssign => BinOp::Div,
                    AssignOp::RemAssign => BinOp::Rem,
                    _ => unreachable!("Assign 由上方独立处理"),
                };
                format!("{} = {}", ts, self.binop(&bin_op, target, value))
            }
        }
    }

    fn index_emit(&mut self, base: &Expr, index: &Expr) -> String {
        let bt = self.infer(base);
        let bs = self.expr_no_clone(base);
        // 括号下标 p[(0..4)] 解包(否则 Range 臂接不住)
        let index = match index {
            Expr::Paren(x) => x,
            _ => index,
        };
        if let Expr::Range { start, end } = index {
            let s0 = start.as_ref().map(|e| self.expr(e)).unwrap_or("0i64".into());
            match &bt {
                // Vec 切片 = 子向量拷贝
                RTy::Vec(_) => {
                    let e0 = match end {
                        Some(e) => self.expr(e),
                        None => format!("(({}).len() as i64)", bs),
                    };
                    return format!("__vslice(&{}, {}, {})", bs, s0, e0);
                }
                RTy::Str | RTy::Unknown => {
                    let e0 = match end {
                        Some(e) => self.expr(e),
                        None => format!("__len_s(&{})", bs),
                    };
                    return format!("__substr(&{}, {}, {})", bs, s0, e0);
                }
                _ => {}
            }
        }
        let is = self.expr(index);
        match &bt {
            RTy::Map(_) => format!("__mget(&{}, &{})", bs, is),
            RTy::Str => format!("__char_at(&{}, {})", bs, is),
            _ => format!("{}[({}) as usize]", bs, is),
        }
    }

    fn call(&mut self, callee: &Expr, args: &[CallArg]) -> String {
        // 方法调用: x.push(1) 解析为 Call { callee: Field { x, push } }
        if let Expr::Field { base, field, .. } = callee {
            let bt = self.infer(base);
            let _ = &bt;
            return self.method_call(base, field, args);
        }
        // 内建函数
        let callee_s = match callee {
            Expr::Ident { name, .. } => {
                if let Some((en, ar)) = self.variant_of(name) {
                    if ar > 0 {
                        // 递归直边载荷实参 -> Box::new; 其余实参值语义拷贝
                        let pts = self.variant_types.get(name).cloned().unwrap_or_default();
                        let mut vs: Vec<String> = Vec::new();
                        for (i, a) in args.iter().enumerate() {
                            let es0 = self.expr(&a.expr);
                            let es = if self.is_fresh_expr(&a.expr) || es0.ends_with(".clone()") {
                                es0
                            } else {
                                format!("{}.clone()", es0)
                            };
                            let boxed = pts.get(i).map(|pt| match pt {
                                RTy::Named(n) => self.is_boxed_edge(&en, n),
                                _ => false,
                            }).unwrap_or(false);
                            if boxed { vs.push(format!("Box::new({})", es)); } else { vs.push(es); }
                        }
                        return format!("{}::{}({})", en, esc_name(name), vs.join(", "));
                    }
                    return format!("{}::{}", en, esc_name(name));
                }
                if name == "assert" {
                    let c = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                    return format!("assert!({})", c);
                }
                match name.as_str() {
                    "print" => {
                        // print(a, b) -> println!("{}", a.__disp()) 多实参空格连接(与宿主 interp 一致)
                        let mut lit = String::new();
                        let mut ps: Vec<String> = Vec::new();
                        for a in args {
                            if !ps.is_empty() { lit.push(' '); }
                            ps.push(format!("({}).__disp()", self.expr(&a.expr)));
                            lit.push_str("{}");
                        }
                        let mut line = format!("println!({}, ", rust_str_lit(&lit));
                        line.push_str(&ps.join(", "));
                        line.push(')');
                        return line;
                    }
                    "base64_encode" => {
                        let a0 = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        let at = args.first().map(|a| self.infer(&a.expr)).unwrap_or(RTy::Unknown);
                        if matches!(at, RTy::Vec(_)) {
                            return format!("__b64_encode_bytes(&{})", a0);
                        }
                        return format!("__b64_encode_str(&{})", a0);
                    }
                    "base64_decode" => {
                        let a0 = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__b64_decode(&{})", a0);
                    }
                    "to_bytes" => {
                        let a0 = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__to_bytes(&{})", a0);
                    }
                    "bytes_to_string" => {
                        let b = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__bytes_to_string(&{})", b);
                    }
                    "read_file" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__read_file(&{})", p);
                    }
                    "read_line" => return "__read_line()".into(),
                    "now" => return "__now()".into(),
                    "now_ms" => return "__now_ms()".into(),
                    "write_file" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        let c = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__write_file(&{}, &{})", p, c);
                    }
                    "append_file" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        let c = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__append_file(&{}, &{})", p, c);
                    }
                    "file_exists" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__file_exists(&{})", p);
                    }
                    "read_bytes" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__read_bytes(&{})", p);
                    }
                    "write_bytes" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        let d = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__write_bytes(&{}, &{})", p, d);
                    }
                    "file_list" => {
                        let p = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__file_list(&{})", p);
                    }
                    "sleep" => {
                        let m2 = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__sleep({})", m2);
                    }
                    "env" => {
                        let k = args.first().map(|a| self.expr(&a.expr)).unwrap_or_default();
                        return format!("__env(&{})", k);
                    }
                    _ => esc_name(name),
                }
            }
            Expr::Path(p) => p.segments.iter().map(|s| esc_name(s)).collect::<Vec<_>>().join("::"),
            other => self.expr(other),
        };
        let as_: Vec<String> = args.iter()
            .map(|a| {
                let t = self.infer(&a.expr);
                let base = self.clone_if_container(&a.expr, &t);
                // 值语义: 实参恒拷贝(闭包传指针, 字面量已自持)
                if self.is_fresh_expr(&a.expr) {
                    base
                } else if base.ends_with(".clone()") {
                    base
                } else {
                    format!("{}.clone()", base)
                }
            })
            .collect();
        format!("{}({})", callee_s, as_.join(", "))
    }

    // 主函数显式返回检测(宿主 CLI: main 有非 Unit 返回值才打印; 裸 return 不算)
    fn body_has_nontrivial_return(&self, b: &Block) -> bool {
        for st in &b.stmts {
            if self.stmt_has_nontrivial_return(st) { return true; }
        }
        if let Some(t) = &b.tail {
            return self.expr_has_nontrivial_return(t);
        }
        false
    }

    fn body_has_any_return(&self, b: &Block) -> bool {
        for st in &b.stmts {
            let hit = match st {
                Stmt::Return(_) => true,
                Stmt::If(i) => self.expr_has_any_return(&i.cond) || self.body_has_any_return(&i.then_branch) || i.else_branch.as_ref().map(|e| self.expr_has_any_return(e)).unwrap_or(false),
                Stmt::While { cond, body, .. } => self.expr_has_any_return(cond) || self.body_has_any_return(body),
                Stmt::For { iter, body, .. } => self.expr_has_any_return(iter) || self.body_has_any_return(body),
                Stmt::Expr(e) | Stmt::Defer(e) => self.expr_has_any_return(e),
                Stmt::TryCatch(t) => self.body_has_any_return(&t.try_block) || self.body_has_any_return(&t.catch_block),
                Stmt::Block(b2) => self.body_has_any_return(b2),
                _ => false,
            };
            if hit { return true; }
        }
        if let Some(t) = &b.tail { return self.expr_has_any_return(t); }
        false
    }

    fn expr_has_any_return(&self, e: &Expr) -> bool {
        match e {
            Expr::Block(b) => self.body_has_any_return(b),
            Expr::If(i) => self.expr_has_any_return(&i.cond) || self.body_has_any_return(&i.then_branch) || i.else_branch.as_ref().map(|el| self.expr_has_any_return(el)).unwrap_or(false),
            Expr::Match { scrutinee, arms, .. } => self.expr_has_any_return(scrutinee) || arms.iter().any(|a| self.expr_has_any_return(&a.value)),
            Expr::Binary { lhs, rhs, .. } => self.expr_has_any_return(lhs) || self.expr_has_any_return(rhs),
            Expr::Assign { target, value, .. } => self.expr_has_any_return(target) || self.expr_has_any_return(value),
            Expr::Call { callee, args, .. } => self.expr_has_any_return(callee) || args.iter().any(|a| self.expr_has_any_return(&a.expr)),
            Expr::Field { base, .. } | Expr::Index { base, .. } | Expr::Unary { expr: base, .. } | Expr::Try(base) | Expr::Paren(base) => self.expr_has_any_return(base),
            Expr::Closure { body, .. } => self.expr_has_any_return(body),
            _ => false,
        }
    }

    fn stmt_has_nontrivial_return(&self, st: &Stmt) -> bool {
        match st {
            // Return(Some) 本身即显式值返回(嵌套表达式扫描无意义)
            Stmt::Return(Some(_)) => true,
            Stmt::Return(None) => false,
            Stmt::If(i) => {
                if self.expr_has_nontrivial_return(&i.cond) { return true; }
                if self.body_has_nontrivial_return(&i.then_branch) { return true; }
                if let Some(el) = &i.else_branch { return self.expr_has_nontrivial_return(el); }
                false
            }
            Stmt::While { cond, body, .. } => self.expr_has_nontrivial_return(cond) || self.body_has_nontrivial_return(body),
            Stmt::For { iter, body, .. } => self.expr_has_nontrivial_return(iter) || self.body_has_nontrivial_return(body),
            Stmt::Expr(e) => self.expr_has_nontrivial_return(e),
            Stmt::TryCatch(t) => self.body_has_nontrivial_return(&t.try_block) || self.body_has_nontrivial_return(&t.catch_block),
            Stmt::Block(b) => self.body_has_nontrivial_return(b),
            _ => false,
        }
    }

    fn expr_has_nontrivial_return(&self, e: &Expr) -> bool {
        match e {
            Expr::Block(b) => self.body_has_nontrivial_return(b),
            Expr::If(i) => {
                if self.expr_has_nontrivial_return(&i.cond) { return true; }
                if self.body_has_nontrivial_return(&i.then_branch) { return true; }
                if let Some(el) = &i.else_branch { return self.expr_has_nontrivial_return(el); }
                false
            }
            Expr::Match { scrutinee, arms, .. } => {
                if self.expr_has_nontrivial_return(scrutinee) { return true; }
                arms.iter().any(|a| self.expr_has_nontrivial_return(&a.value))
            }
            Expr::Binary { lhs, rhs, .. } => self.expr_has_nontrivial_return(lhs) || self.expr_has_nontrivial_return(rhs),
            Expr::Assign { target, value, .. } => self.expr_has_nontrivial_return(target) || self.expr_has_nontrivial_return(value),
            Expr::Call { callee, args, .. } => self.expr_has_nontrivial_return(callee) || args.iter().any(|a| self.expr_has_nontrivial_return(&a.expr)),
            Expr::Field { base, .. } | Expr::Index { base, .. } | Expr::Unary { expr: base, .. } | Expr::Try(base) | Expr::Paren(base) => self.expr_has_nontrivial_return(base),
            Expr::Array(items) | Expr::Tuple(items) => items.iter().any(|x| self.expr_has_nontrivial_return(x)),
            Expr::Closure { body, .. } => self.expr_has_nontrivial_return(body),
            _ => false,
        }
    }

    // W54-e: 形参作函数值调用 — 扫描体内 Call{callee: Ident(形参)} 收集 (形参名, 实参列表)
    fn scan_called_params_expr(&self, e: &Expr, pnames: &[String], out: &mut Vec<(String, Vec<Expr>)>) {
        match e {
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident { name, .. } = &**callee {
                    if pnames.iter().any(|p| p == name) {
                        out.push((name.clone(), args.iter().map(|a| a.expr.clone()).collect()));
                    }
                }
                self.scan_called_params_expr(callee, pnames, out);
                for a in args { self.scan_called_params_expr(&a.expr, pnames, out); }
            }
            Expr::Array(items) => for x in items { self.scan_called_params_expr(x, pnames, out); },
            Expr::StructLit { fields, .. } => for (_f2, fe) in fields { self.scan_called_params_expr(fe, pnames, out); },
            Expr::Unary { expr, .. } => self.scan_called_params_expr(expr, pnames, out),
            Expr::Binary { lhs, rhs, .. } => { self.scan_called_params_expr(lhs, pnames, out); self.scan_called_params_expr(rhs, pnames, out); }
            Expr::Assign { target, value, .. } => { self.scan_called_params_expr(target, pnames, out); self.scan_called_params_expr(value, pnames, out); }
            Expr::Field { base, .. } => self.scan_called_params_expr(base, pnames, out),
            Expr::Index { base, index } => { self.scan_called_params_expr(base, pnames, out); self.scan_called_params_expr(index, pnames, out); }
            Expr::Try(x) => self.scan_called_params_expr(x, pnames, out),
            Expr::Range { start, end } => for x in [start, end].into_iter().flatten() { self.scan_called_params_expr(x, pnames, out); },
            Expr::Block(b) => self.scan_called_params_block(b, pnames, out),
            Expr::If(i) => self.scan_called_params_if(i, pnames, out),
            Expr::Match { scrutinee, arms, .. } => {
                self.scan_called_params_expr(scrutinee, pnames, out);
                for arm in arms { self.scan_called_params_expr(&arm.value, pnames, out); }
            }
            Expr::Closure { body, .. } => self.scan_called_params_expr(body, pnames, out),
            Expr::Paren(x) => self.scan_called_params_expr(x, pnames, out),
            Expr::Tuple(items) => for x in items { self.scan_called_params_expr(x, pnames, out); },
            _ => {}
        }
    }

    fn scan_called_params_block(&self, b: &Block, pnames: &[String], out: &mut Vec<(String, Vec<Expr>)>) {
        for st in &b.stmts { self.scan_called_params_stmt(st, pnames, out); }
        if let Some(t) = &b.tail { self.scan_called_params_expr(t, pnames, out); }
    }

    fn scan_called_params_if(&self, i: &IfExpr, pnames: &[String], out: &mut Vec<(String, Vec<Expr>)>) {
        self.scan_called_params_expr(&i.cond, pnames, out);
        self.scan_called_params_block(&i.then_branch, pnames, out);
        if let Some(el) = &i.else_branch { self.scan_called_params_expr(el, pnames, out); }
    }

    fn scan_called_params_stmt(&self, st: &Stmt, pnames: &[String], out: &mut Vec<(String, Vec<Expr>)>) {
        match st {
            Stmt::Let(l) => if let Some(e) = &l.init { self.scan_called_params_expr(e, pnames, out); },
            Stmt::Expr(e) => self.scan_called_params_expr(e, pnames, out),
            Stmt::Return(Some(e)) => self.scan_called_params_expr(e, pnames, out),
            Stmt::If(i) => self.scan_called_params_if(i, pnames, out),
            Stmt::While { cond, body, .. } => { self.scan_called_params_expr(cond, pnames, out); self.scan_called_params_block(body, pnames, out); }
            Stmt::For { iter, body, .. } => { self.scan_called_params_expr(iter, pnames, out); self.scan_called_params_block(body, pnames, out); }
            Stmt::Defer(e) => self.scan_called_params_expr(e, pnames, out),
            Stmt::TryCatch(t) => { self.scan_called_params_block(&t.try_block, pnames, out); self.scan_called_params_block(&t.catch_block, pnames, out); }
            Stmt::Block(b) => self.scan_called_params_block(b, pnames, out),
            _ => {}
        }
    }

    // 方法调用 base 不做值语义 clone: 变异方法(push/set/insert...)必须作用于绑定本身
    fn expr_no_clone(&mut self, e: &Expr) -> String {
        match e {
            Expr::Ident { name, .. } => self.ident_emit(name),
            // 赋值目标: 索引/字段访问必须保持裸形式(可赋值位)
            Expr::Index { base, index } => self.index_emit(base, index),
            Expr::Field { base, field, .. } => format!("{}.{}", self.expr_no_clone(base), field),
            other => self.expr(other),
        }
    }

    fn method_call(&mut self, base: &Expr, mname: &str, args: &[CallArg]) -> String {
        let bt = self.infer(base);
        let bs = self.expr_no_clone(base);
        let a0 = args.first().map(|a| self.expr(&a.expr));
        match (&bt, mname) {
            (_, "len") => match &bt {
                RTy::Str => format!("__len_s(&{})", bs),
                RTy::Vec(_) => format!("(({}).len() as i64)", bs),
                _ => format!("(({}).len() as i64)", bs),
            },
            // 迭代器管道求和: turbofish 消歧(Item 恒 i64)
            (_, "sum") => match &bt {
                RTy::Vec(e) if matches!(**e, RTy::F64) => format!("({}).sum::<f64>()", bs),
                _ => format!("({}).sum::<i64>()", bs),
            },
            (RTy::Vec(_), "get") => format!("{}.get(({}) as usize).cloned()", bs, a0.unwrap_or_default()),
            // 实参先求值再借用(显式 &mut 无 two-phase)
            (RTy::Vec(_), "push") => format!("{{ let __pv = ({}).clone(); __push(&mut {}, __pv) }}", a0.unwrap_or_default(), bs),
            (RTy::Vec(_), "pop") => format!("__pop(&mut {})", bs),
            (RTy::Vec(_), "set") => {
                let i_s = self.expr(&args[0].expr);
                let v_s = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                format!("{{ let __si = ({}).clone(); let __sv = ({}).clone(); __vset(&mut {}, __si, __sv) }}", i_s, v_s, bs)
            }
            (RTy::Vec(_), "insert") => {
                let i_s = self.expr(&args[0].expr);
                let v_s = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                format!("{{ let __ii = ({}).clone(); let __iv = ({}).clone(); __vinsert(&mut {}, __ii, __iv) }}", i_s, v_s, bs)
            }
            (RTy::Vec(_), "remove") => {
                let i_s = self.expr(&args[0].expr);
                format!("{{ let __ri = ({}).clone(); __vremove(&mut {}, __ri) }}", i_s, bs)
            }
            (RTy::Vec(_), "contains") => format!("{}.contains(&{})", bs, a0.unwrap_or_default()),
            (RTy::Vec(_), "concat") => format!("__vconcat(&{}, &{})", bs, a0.unwrap_or_default()),
            (RTy::Vec(_), "clear") => format!("{{ let mut __v = {}; __v.clear(); __v }}", bs),
            (RTy::Vec(_), "sort") => format!("{{ let mut __v = {}; __vsort(&mut __v); __v }}", bs),
            (RTy::Vec(_), "iter") => format!("{}.iter().cloned()", bs),
            (RTy::Map(_), "new") => "__Map::new()".into(),
            // Map.new() 的 base 推断常为 Unknown(Ident{Map} 无绑定),在此兜住
            (_, "new") => "__Map::new()".into(),
            (RTy::Map(_), "set") => {
                let k = a0.unwrap_or_default();
                let v = args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default();
                format!("{}.insert({}.clone(), ({}).clone())", bs, k, v)
            }
            (RTy::Map(_), "get") => format!("{}.get(&{}).cloned()", bs, a0.unwrap_or_default()),
            (RTy::Map(_), "contains") => format!("{}.contains(&{})", bs, a0.unwrap_or_default()),
            (RTy::Map(_), "remove") => format!("{}.remove(&{})", bs, a0.unwrap_or_default()),
            (RTy::Map(_), "keys") => format!("{}.keys()", bs),
            (RTy::Map(_), "values") => format!("{}.values()", bs),
            (RTy::Str, "trim") => format!("{}.trim().to_string()", bs),
            (RTy::Str, "replace") => format!(
                "{}.replace(&{}, &{})",
                bs,
                a0.unwrap_or_default(),
                args.get(1).map(|a| self.expr(&a.expr)).unwrap_or_default()
            ),
            (RTy::Str, "split") => format!(
                "{}.split(&{}).map(String::from).collect::<Vec<String>>()",
                bs,
                a0.unwrap_or_default()
            ),
            (RTy::Str, "contains") => format!("{}.contains(&{})", bs, a0.unwrap_or_default()),
            (RTy::Str, "starts_with") => format!("{}.starts_with(&{})", bs, a0.unwrap_or_default()),
            (RTy::Str, "ends_with") => format!("{}.ends_with(&{})", bs, a0.unwrap_or_default()),
            (RTy::Str, "to_i32") | (RTy::Str, "to_i64") => {
                format!("{}.trim().parse::<i64>().unwrap_or(0)", bs)
            }
            (RTy::I64, "to_f64") => format!("(({}) as f64)", bs),
            (RTy::I64, "to_i32") | (RTy::I64, "to_i64") => format!("({})", bs),
            (RTy::F64, "to_i32") | (RTy::F64, "to_i64") => format!("(({}) as i64)", bs),
            (RTy::F64, "to_f64") => format!("{}", bs),
            (RTy::F64, "len") => format!("(({}).to_string().len() as i64)", bs),
            (RTy::Str, "to_f64") => format!("{}.trim().parse::<f64>().unwrap_or(0.0)", bs),
            (RTy::Str, "codepoint_at") => format!(
                "{}.chars().nth(({}) as usize).map(|c| c as i64).unwrap_or(0)",
                bs,
                a0.unwrap_or_default()
            ),
            (_, "clone") => format!("{}.clone()", bs),
            _ => {
                let arg_s: Vec<String> = args.iter().map(|a| self.expr(&a.expr)).collect();
                format!("{}.{}({})", bs, mname, arg_s.join(", "))
            }
        }
    }

    fn fstr(&mut self, s: &str) -> String {
        // f-string 插值: {expr} -> format!("lit", e1.__disp(), ...) 与宿主 eval_fmt 同规则
        // raw_lit: 未转义字面量(无插值时用); esc_lit: 花括号已转义(有插值时用),占位符保持裸 {}
        let mut raw_lit = String::new();
        let mut esc_lit = String::new();
        let mut parts: Vec<String> = Vec::new();
        let mut rest = s;
        while let Some(start) = rest.find('{') {
            let chunk = &rest[..start];
            raw_lit.push_str(chunk);
            esc_lit.push_str(&chunk.replace('{', "{{").replace('}', "}}"));
            let after = &rest[start + 1..];
            match after.find('}') {
                Some(end) => {
                    let expr_src = &after[..end];
                    if let Some(expr) = crate::parser::parse_expression_str(expr_src) {
                        parts.push(format!("({}).__disp()", self.expr(&expr)));
                        raw_lit.push('{');
                        raw_lit.push_str(expr_src);
                        raw_lit.push('}');
                        esc_lit.push_str("{}");
                    } else {
                        // 解析失败按字面量保留(与宿主一致)
                        raw_lit.push('{');
                        raw_lit.push_str(expr_src);
                        raw_lit.push('}');
                        esc_lit.push_str("{{");
                        esc_lit.push_str(expr_src);
                        esc_lit.push_str("}}");
                    }
                    rest = &after[end + 1..];
                }
                None => {
                    let chunk = &rest[start..];
                    raw_lit.push_str(chunk);
                    esc_lit.push_str(&chunk.replace('{', "{{").replace('}', "}}"));
                    rest = "";
                    break;
                }
            }
        }
        raw_lit.push_str(rest);
        esc_lit.push_str(&rest.replace('{', "{{").replace('}', "}}"));
        if parts.is_empty() {
            return format!("{}.to_string()", rust_str_lit(&raw_lit));
        }
        format!("format!({}, {})", rust_str_lit(&esc_lit), parts.join(", "))
    }

    fn if_expr(&mut self, i: &IfExpr) -> String {
        let mut out = format!("if {} {{ ", self.expr(&i.cond));
        for st in &i.then_branch.stmts {
            out.push_str(&self.stmt(st, 0));
            out.push(' ');
        }
        if let Some(t) = &i.then_branch.tail {
            out.push_str(&self.expr(t));
            out.push(' ');
        }
        out.push('}');
        if let Some(e) = &i.else_branch {
            out.push_str(" else ");
            match &**e {
                Expr::Block(b) => {
                    out.push_str("{ ");
                    for st in &b.stmts {
                        out.push_str(&self.stmt(st, 0));
                        out.push(' ');
                    }
                    if let Some(t) = &b.tail {
                        out.push_str(&self.expr(t));
                        out.push(' ');
                    }
                    out.push('}');
                }
                Expr::If(inner) => out.push_str(&self.if_expr(inner)),
                other => out.push_str(&self.expr(other)),
            }
        }
        out
    }

    // 新造临时判定: Call/Binary/字面量/构造等结果无别名, 拷贝纯属浪费(性能)
    fn is_fresh_expr(&self, e: &Expr) -> bool {
        match e {
            Expr::Call { .. } | Expr::Binary { .. } | Expr::Unary { .. } | Expr::StructLit { .. }
            | Expr::FmtStr(_) | Expr::Str(_) | Expr::Int(_) | Expr::Float(_) | Expr::Bool(_)
            | Expr::Array(_) | Expr::Tuple(_) | Expr::Closure { .. } | Expr::Range { .. } => true,
            Expr::Paren(x) => self.is_fresh_expr(x),
            _ => false,
        }
    }

    // match 检察体发射: 容器/枚举/结构体一律拷贝(值语义), 其他裸发
    fn match_scrutinee(&mut self, scrutinee: &Expr, st: &RTy) -> String {
        let base = self.clone_if_container(scrutinee, st);
        if base.ends_with(".clone()") {
            base
        } else if matches!(st, RTy::Vec(_) | RTy::Map(_) | RTy::Named(_) | RTy::Str | RTy::Opt(_) | RTy::Res(_, _)) {
            format!("({}).clone()", base)
        } else {
            base
        }
    }

    // stmt_mode: match 作语句(值丢弃) — 臂体块走 Stmt 位,裸表达式臂包成分号块
    fn match_expr(&mut self, scrutinee: &Expr, arms: &[MatchArm], stmt_mode: bool) -> String {
        let st = self.infer(scrutinee).unboxed();
        let enum_name = match &st {
            RTy::Named(n) => n.clone(),
            _ => String::new(),
        };
        // 宿主语义: match 检察体 = 值拷贝(枚举臂内解构 move 不影响源)
        let mut out = format!("match {} {{", self.match_scrutinee(scrutinee, &st));
        for arm in arms {
            // 模式绑定类型注册(Some(c)/Ok(v)/具名变体/结构体解构); BoxedT 剥箱
            self.register_pattern_binds(&arm.pat, &st.unboxed());
            let (pat, pat_lets) = self.pattern2(&arm.pat, &enum_name);
            if let Some(_dead) = pat.strip_prefix("__DEAD::") {
                // 假臂: 字段绑定以 String 注入(永不匹配,但臂体可编译); 语句位同样适用
                let bnames = match &arm.pat {
                    crate::ast::Pattern::Variant { fields, .. } => fields.iter().filter_map(|f| match f {
                        crate::ast::Pattern::Ident { name, .. } if name != "_" => Some(name.clone()),
                        _ => None,
                    }).collect::<Vec<String>>(),
                    _ => Vec::new(),
                };
                let mut body = self.expr(&arm.value);
                let lets: Vec<String> = bnames.iter().map(|b| format!("let {}: String = String::new();", esc_name(b))).collect();
                body = format!("{{ {} {} {} }}", pat_lets.join(" "), lets.join(" "), body);
                out.push_str(&format!(" _ if false => {},", body));
                continue;
            }
            if stmt_mode {
                // 语句位 match: 臂值归 ()(宿主语句位 match 丢弃臂值,臂间可异型)
                let body = match &arm.value {
                    Expr::Block(b) => {
                        let inner = self.block(b, 0, BlockMode::Stmt);
                        format!("{{ {} {} }}", pat_lets.join(" "), inner.trim_end())
                    }
                    other => format!("{{ {} {}; }}", pat_lets.join(" "), self.expr(other)),
                };
                out.push_str(&format!(" {} => {},", pat, body));
                continue;
            }
            let body = self.expr(&arm.value);
            if pat_lets.is_empty() {
                out.push_str(&format!(" {} => {},", pat, body));
            } else {
                out.push_str(&format!(" {} => {{ {} {} }},", pat, pat_lets.join(" "), body));
            }
        }
            // W34 缺臂语义对齐: 用户枚举无通配且未覆盖全部变体 -> 补 panic 臂(宿主/C 路同款运行时 panic)
            if let RTy::Named(en2) = &st {
                if let Some(variants) = self.enums.get(en2).cloned() {
                    let mut has_wild = false;
                    let mut covered: Vec<String> = Vec::new();
                    for arm in arms {
                        match &arm.pat {
                            crate::ast::Pattern::Wildcard => has_wild = true,
                            crate::ast::Pattern::Ident { name, .. } => { if name == "_" { has_wild = true; } else { covered.push(name.clone()); } }
                            crate::ast::Pattern::Variant { name, .. } => covered.push(name.clone()),
                            _ => {}
                        }
                    }
                    if !has_wild {
                        let missing: Vec<String> = variants.iter().filter(|(v, _)| !covered.contains(v)).map(|(v, _)| v.clone()).collect();
                        if !missing.is_empty() {
                            out.push_str(&format!(" _ => {{ eprintln!(\"match \u{7f3a}\u{81c2}: {}::{}\"); std::process::exit(101); }}, ", en2, missing.join(",")));
                        }
                    }
                }
            }
        out.push_str(" }");
        out
    }

    // 模式绑定的类型注册: 让臂体内的绑定拥有正确推断类型
    fn register_pattern_binds(&mut self, p: &Pattern, scrutinee_ty: &RTy) {
        match p {
            Pattern::Ident { name, .. } => {
                if name != "_" { self.set_var(name, scrutinee_ty.clone()); }
            }
            Pattern::Variant { name, fields } => {
                // std 容器变体
                if name == "Some" || name == "Ok" {
                    let elem = match scrutinee_ty {
                        RTy::Opt(t) => (**t).clone(),
                        RTy::Res(a, _) => (**a).clone(),
                        _ => RTy::Unknown,
                    };
                    if let Some(f0) = fields.first() { self.register_pattern_binds(f0, &elem); }
                    return;
                }
                if name == "Err" {
                    let elem = match scrutinee_ty {
                        RTy::Res(_, b) => (**b).clone(),
                        _ => RTy::Unknown,
                    };
                    if let Some(f0) = fields.first() { self.register_pattern_binds(f0, &elem); }
                    return;
                }
                // 具名变体: 载荷类型注册表; 递归直边载荷 -> BoxedT(Rust 侧绑定是指针)
                if let Some(pts) = self.variant_types.get(name).cloned() {
                    let owner = self.variant_of(name).map(|(e2, _)| e2).unwrap_or_default();
                    for (f, t) in fields.iter().zip(pts) {
                        let t2 = match &t {
                            RTy::Named(n) if self.is_boxed_edge(&owner, n) => RTy::BoxedT(Box::new(t.clone())),
                            _ => t,
                        };
                        self.register_pattern_binds(f, &t2);
                    }
                    return;
                }
                // 结构体位置解构
                if let Some(fs) = self.structs.get(name).cloned() {
                    for (f, (_fn2, ft)) in fields.iter().zip(fs) {
                        self.register_pattern_binds(f, &ft);
                    }
                }
            }
            Pattern::Path(_) | Pattern::Wildcard | Pattern::Lit(_) => {}
        }
    }

    fn pattern(&mut self, p: &Pattern, enum_name: &str) -> String {
        let (s, lets) = self.pattern2(p, enum_name);
        debug_assert!(lets.is_empty(), "顶层 pattern() 不应产生解构链");
        s
    }

    // 模式发射 v2: 返回 (模式串, 前置 let-else 解构语句)。
    // Box 载荷槽上的复杂子模式(Box 无法在模式位解引用) -> 绑定 __bxN + (*x).clone() let-else 链(W54-a)
    fn pattern2(&mut self, p: &Pattern, enum_name: &str) -> (String, Vec<String>) {
        match p {
            Pattern::Wildcard => ("_".into(), Vec::new()),
            Pattern::Ident { name, .. } => {
                if name == "_" {
                    ("_".into(), Vec::new())
                } else if enum_name.is_empty() {
                    match self.variant_of(name) {
                        Some((en, _)) => (format!("{}::{}", en, esc_name(name)), Vec::new()),
                        None => (esc_name(name), Vec::new()),
                    }
                } else {
                    (format!("{}::{}", enum_name, esc_name(name)), Vec::new())
                }
            }
            Pattern::Variant { name, fields } => {
                // 结构体位置解构: AEntry(k, v) -> AEntry { key: k, value: v }
                if (enum_name.is_empty() || enum_name == name) && self.structs.contains_key(name) {
                    let fnames: Vec<String> = self.structs[name].iter().map(|(n, _)| n.clone()).collect();
                    if fnames.len() == fields.len() {
                        let ps: Vec<String> = fields.iter().zip(fnames)
                            .map(|(f, n)| format!("{}: {}", n, self.pattern2(f, "").0)).collect();
                        return (format!("{} {{ {} }}", esc_name(name), ps.join(", ")), Vec::new());
                    }
                }
                // std 容器变体模式直通(绑定字段必须保留)
                if matches!(name.as_str(), "Some" | "None" | "Ok" | "Err") {
                    if name == "None" || fields.is_empty() {
                        return (esc_name(name), Vec::new());
                    }
                    // String 载荷槽的字符串字面量模式 -> 绑定+臂级守卫(Rust 无 String 字面量模式)
                    let mut ps: Vec<String> = Vec::new();
                    let mut guards: Vec<String> = Vec::new();
                    for f in fields {
                        if let Pattern::Lit(Expr::Str(sv)) = f {
                            let bind = format!("__sl{}", self.tmp_n);
                            self.tmp_n += 1;
                            ps.push(bind.clone());
                            guards.push(format!("{} == {}", bind, rust_str_lit(sv)));
                        } else {
                            ps.push(self.pattern2(f, "").0);
                        }
                    }
                    let pat_s = format!("{}({})", esc_name(name), ps.join(", "));
                    if guards.is_empty() {
                        return (pat_s, Vec::new());
                    }
                    return (format!("{} if {}", pat_s, guards.join(" && ")), Vec::new());
                }
                let known = match self.variant_of(name) {
                    Some((e2, _)) => format!("{}::", e2),
                    None => if enum_name.is_empty() { String::new() } else { format!("{}::", enum_name) },
                };
                if known.is_empty() {
                    return (format!("__DEAD::{}", name), Vec::new());
                }
                if fields.is_empty() {
                    (format!("{}{}", known, esc_name(name)), Vec::new())
                } else {
                    // 载荷类型(BoxedT = 递归直边槽)
                    let pts = self.variant_types.get(name).cloned().unwrap_or_default();
                    let owner = self.variant_of(name).map(|(e2, _)| e2).unwrap_or_default();
                    let mut parts: Vec<String> = Vec::new();
                    let mut lets: Vec<String> = Vec::new();
                    let mut guards: Vec<String> = Vec::new();
                    for (i, (f, pt)) in fields.iter().zip(pts.iter()).enumerate() {
                        // String 载荷槽的字符串字面量模式 -> 绑定+臂级守卫
                        if let Pattern::Lit(Expr::Str(sv)) = f {
                            let bind = format!("__sl{}", self.tmp_n);
                            self.tmp_n += 1;
                            parts.push(bind.clone());
                            guards.push(format!("{} == {}", bind, rust_str_lit(sv)));
                            continue;
                        }
                        let boxed = match pt {
                            RTy::Named(n2) => self.is_boxed_edge(&owner, n2),
                            _ => false,
                        };
                        let complex = !matches!(f, Pattern::Ident { .. } | Pattern::Wildcard);
                        if boxed && complex {
                            // Box 槽位复杂子模式: 绑定 + let-else 解构(值语义 clone)
                            let bind = format!("__bx{}", self.tmp_n);
                            self.tmp_n += 1;
                            parts.push(bind.clone());
                            let sub_en = match pt { RTy::Named(n2) => n2.clone(), _ => String::new() };
                            let (sub_pat, sub_lets) = self.pattern2(f, &sub_en);
                            lets.push(format!("let {} = (*{}).clone() else {{ unreachable!() }};", sub_pat, bind));
                            lets.extend(sub_lets);
                        } else {
                            let (s, l) = self.pattern2(f, "");
                            parts.push(s);
                            lets.extend(l);
                        }
                    }
                    let pat_s = format!("{}{}({})", known, esc_name(name), parts.join(", "));
                    if guards.is_empty() {
                        (pat_s, lets)
                    } else {
                        (format!("{} if {}", pat_s, guards.join(" && ")), lets)
                    }
                }
            }
            Pattern::Lit(e) => {
                match e {
                    // 字符串模式位必须裸字面量(.to_string() 是表达式)
                    Expr::Str(s2) => (rust_str_lit(s2), Vec::new()),
                    Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) => (self.expr(e), Vec::new()),
                    // 复杂模式(宿主运行时求值) -> 绑定+相等守卫
                    _ => (format!("__pl if __pl == {}", self.expr(e)), Vec::new()),
                }
            }
            Pattern::Path(p) => (format!("{}::{}", enum_name, p.segments.join("::")), Vec::new()),
        }
    }
}
fn binop_str(op: &BinOp) -> &'static str {
    match op {
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
    }
}

// 自由包装器:main.rs 的 rust 子命令入口(与同名方法共存,命名空间不同)
pub fn emit_program(p: &Program) -> String {
    let mut e = Emitter::new();
    e.emit_program(p)
}
