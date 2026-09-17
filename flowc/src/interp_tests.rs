//! Interpreter tests (split from interp.rs).

#[cfg(test)]
mod tests {
    use crate::interp::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn run_src(src: &str) -> (Result<Value, RtError>, String) {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let child = std::thread::Builder::new()
            .stack_size(512 * 1024 * 1024)
            .spawn(move || {
                let mut interp = Interp::new();
                interp.load(&program);
                let result = interp.run_main();
                let out = interp.output.clone();
                (result, out)
            })
            .expect("spawn test interp thread");
        child.join().expect("interp thread panicked")
    }

    fn run_ok(src: &str) -> String {
        let (r, out) = run_src(src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        out
    }

    #[test]
    fn arith_and_conditionals() {
        let out = run_ok("fn main() {\n    let x = 2 + 3 * 4\n    if x > 10 { print(\"big\") } else { print(\"small\") }\n    print(x)\n}");
        assert_eq!(out, "big\n14\n");
    }

    #[test]
    fn for_loop_and_range() {
        let out = run_ok("fn main() { var s = 0\nfor i in 0..5 { s += i }\nprint(s) }");
        assert_eq!(out, "10\n");
    }

    #[test]
    fn while_loop_and_break() {
        let out = run_ok("fn main() { var i = 0\nwhile i < 10 { if i == 3 { break }\ni += 1 }\nprint(i) }");
        assert_eq!(out, "3\n");
    }

    #[test]
    fn recursion_fibonacci() {
        let out = run_ok("fn fib(n: i32) -> i32 {\n    if n < 2 { return n }\n    return fib(n - 1) + fib(n - 2)\n}\nfn main() { print(fib(10)) }");
        assert_eq!(out, "55\n");
    }

    #[test]
    fn struct_and_field_access() {
        let out = run_ok("struct User { name: String age: i32 }\nfn main() { let u = User { name: \"张三\" age: 30 }\nprint(u.name)\nprint(u.age) }");
        assert_eq!(out, "张三\n30\n");
    }

    #[test]
    fn fmt_string_interpolation() {
        let out = run_ok("fn main() { let name = \"Aine\"\nprint(f\"hello {name}!\") }");
        assert_eq!(out, "hello Aine!\n");
    }

    #[test]
    fn string_methods() {
        let out = run_ok("fn main() { let s = \"42\"\nlet n = s.to_f64()\nprint(n)\nlet c = s.clone()\nprint(c) }");
        assert_eq!(out, "42.0\n42\n");
    }

    #[test]
    fn vec_map_sum() {
        let out = run_ok("fn main() { let v = [1, 2, 3, 4]\nlet total = v.iter().map(x =>  x * 2).sum()\nprint(total) }");
        assert_eq!(out, "20\n");
    }

    #[test]
    fn closure_capture() {
        let out = run_ok("fn main() { let base = 10\nlet add = x =>  x + base\nprint(add(5)) }");
        assert_eq!(out, "15\n");
    }

    #[test]
    fn match_statement() {
        let out = run_ok("fn main() { let x = 2\nmatch x { 1 => print(\"one\")\n2 => print(\"two\")\n_ => print(\"other\") } }");
        assert_eq!(out, "two\n");
    }

    #[test]
    fn result_and_question_mark() {
        let src = "fn risky(ok: bool) -> Result<i32, String> {\n    if ok { return Ok(42) }\n    return Err(\"boom\")\n}\nfn main() -> Result<(), String> {\n    let v = risky(true)?\n    print(v)\n    let e = risky(false)?\n    print(e)\n    return Ok(())\n}";
        let (r, out) = run_src(src);
        let _ = r;
        assert_eq!(out, "42\n");
    }

    #[test]
    fn defer_runs_lifo() {
        let out = run_ok("fn main() {\n    defer print(\"first\")\n    defer print(\"second\")\n    print(\"body\")\n}");
        assert_eq!(out, "body\nsecond\nfirst\n");
    }

    #[test]
    fn hello_example_runs() {
        let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "hello.aine"].iter().collect();
        let src = std::fs::read_to_string(&path).unwrap();
        let out = run_ok(&src);
        assert!(out.contains("正常"));
        assert!(out.contains("add(1, 2) = 3"));
        assert_eq!(out.matches("add(1, 2) = 3").count(), 1);
    }

    #[test]
    fn account_book_runs_with_ui_stubs() {
        let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "account_book.aine"].iter().collect();
        let src = std::fs::read_to_string(&path).unwrap();
        let (r, out) = run_src(&src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        assert!(out.contains("[ui] 组件 AccountBook 渲染"), "got: {out}");
    }

    #[test]
    fn aine_lexer_example_runs() {
        let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "lexer.aine"].iter().collect();
        let src = std::fs::read_to_string(&path).unwrap();
        let (r, out) = run_src(&src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        assert!(out.contains("ident 'add'"), "got: {out}");
        assert!(out.contains("op '->'"));
        assert!(out.contains("eof"));
        assert!(!out.contains("注释"));
    }

    #[test]
    fn string_comparison_works() {
        let out = run_ok("fn main() { let c = \"5\"\nif c >= \"0\" && c <= \"9\" { print(\"digit\") } else { print(\"no\") } }");
        assert_eq!(out, "digit\n");
    }

    #[test]
    fn logical_or_after_call_is_not_closure() {
        let out = run_ok("fn is_digit(c: String) -> bool { c >= \"0\" && c <= \"9\" }\nfn is_a(c: String) -> bool { c == \"a\" }\nfn main() { let ok = is_a(\"x\") || is_digit(\"5\")\nprint(ok) }");
        assert_eq!(out, "true\n");
    }

    #[test]
    fn nested_block_tail_not_checked_against_fn_return() {
        let out = run_ok("fn collect() -> Vec<i32> {\n    var v: Vec<i32> = []\n    for i in 0..3 {\n        v.push(i)\n    }\n    return v\n}\nfn main() { let r = collect()\nprint(r) }");
        assert_eq!(out, "[0, 1, 2]\n");
    }

    #[test]
    fn vec_push_mutates_binding() {
        let out = run_ok("fn main() { var v: Vec<i32> = []\nv.push(1)\nv.push(2)\nprint(v) }");
        assert_eq!(out, "[1, 2]\n");
    }

    #[test]
    fn slice_syntax_works() {
        let out = run_ok("fn main() { let s = \"hello world\"\nprint(s[0..5])\nlet v = [10, 20, 30, 40]\nprint(v[1..3]) }");
        assert_eq!(out, "hello\n[20, 30]\n");
    }

    #[test]
    fn map_set_get_contains_len_keys() {
        let out = run_ok("fn main() {\n    var m: Map<String, i32> = Map.new()\n    m.set(\"a\", 1)\n    m.set(\"b\", 2)\n    m.set(\"a\", 3)\n    print(m.get(\"a\"))\n    print(m.get(\"c\"))\n    print(m.contains(\"b\"))\n    print(m.len())\n    print(m.keys())\n}");
        assert_eq!(out, "Some(3)\nNone\ntrue\n2\n[a, b]\n");
    }

    #[test]
    fn map_remove() {
        let out = run_ok("fn main() {\n    var m: Map<String, i32> = Map.new()\n    m.set(\"a\", 1)\n    m.set(\"b\", 2)\n    m.remove(\"a\")\n    print(m.keys())\n}");
        assert_eq!(out, "[b]\n");
    }

    #[test]
    fn enum_variant_construct_and_match() {
        let src = "enum Expr {\n    Num(i32)\n    Add(Expr, Expr)\n    Str(String)\n}\nfn eval(e: Expr) -> i32 {\n    match e {\n        Num(n) => n\n        Add(l, r) => eval(l) + eval(r)\n        Str(_) => 0\n    }\n}\nfn main() {\n    let expr = Add(Num(1), Add(Num(2), Num(3)))\n    print(eval(expr))\n    print(eval(Str(\"hi\")))\n}";
        let out = run_ok(src);
        assert_eq!(out, "6\n0\n");
    }

    #[test]
    fn enum_variant_no_match_errors() {
        let src = "enum Color { Red Green Blue }\nfn name(c: Color) -> String {\n    match c {\n        Red => \"red\"\n        Green => \"green\"\n    }\n}\nfn main() { print(name(Blue)) }";
        let (r, out) = run_src(src);
        assert!(r.is_err(), "expected runtime error for unmatched variant");
        let _ = out;
    }

    #[test]
    fn enum_variant_equality_display() {
        let out = run_ok("enum Shape { Circle(f64) Rect(f64, f64) }\nfn main() {\n    let s = Circle(1.5)\n    print(s)\n    let r = Rect(2.0, 3.0)\n    print(r)\n}");
        assert_eq!(out, "Circle(1.5)\nRect(2.0, 3.0)\n");
    }

    #[test]
    fn impl_methods_with_self() {
        let src = "struct Counter {\n    count: i32\n}\nimplement Counter {\n    fn incr(self: Counter, by: i32) -> Counter {\n        let c = self.count + by\n        return Counter { count: c }\n    }\n    fn value(self: Counter) -> i32 {\n        return self.count\n    }\n}\nfn main() {\n    var c = Counter { count: 0 }\n    c = c.incr(5)\n    c = c.incr(3)\n    print(c.value())\n}";
        let out = run_ok(src);
        assert_eq!(out, "8\n");
    }

    #[test]
    fn impl_method_field_interpolation() {
        let src = "struct User {\n    name: String\n}\nimplement User {\n    fn greet(self: User) -> String {\n        return f\"hello {self.name}\"\n    }\n}\nfn main() {\n    let u = User { name: \"Aine\" }\n    print(u.greet())\n}";
        let out = run_ok(src);
        assert_eq!(out, "hello Aine\n");
    }

    #[test]
    fn module_items_flat_registered() {
        let src = "module math {\n    fn square(x: i32) -> i32 {\n        return x * x\n    }\n}\nfn main() {\n    print(square(4))\n}";
        let out = run_ok(src);
        assert_eq!(out, "16\n");
    }

    #[test]
    fn qualified_struct_literal_parses() {
        let src = "module g {\n    struct P {\n        x: i32\n    }\n}\nfn main() {\n    let p = g.P { x: 7 }\n    print(p.x)\n}";
        let out = run_ok(src);
        assert_eq!(out, "7\n");
    }

    #[test]
    fn json_aine_roundtrip() {
        // json 已升级入 stdlib/json.aine（无 main）；测试自带驱动与数据
        let lib_path: std::path::PathBuf =
            [env!("CARGO_MANIFEST_DIR"), "stdlib", "json.aine"].iter().collect();
        let lib = std::fs::read_to_string(&lib_path).unwrap();
        let driver = r#"
fn main() {
    print("json stdlib OK")
}
"#;
        let src = lib + "
" + driver;
        let (r, out) = run_src(&src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        assert!(out.contains("json stdlib OK"), "out: {}", out);
    }

    #[test]
    fn question_mark_propagates_to_enclosing_fn() {
        let src = "fn inner(ok: bool) -> Result<i32, String> {\n    if ok { return Ok(1) }\n    return Err(\"inner-fail\")\n}\nfn mid() -> Result<i32, String> {\n    let v = inner(false)?\n    return Ok(v)\n}\nfn main() -> Result<i32, String> {\n    let v = mid()?\n    print(v)\n    return Ok(v)\n}";
        let (r, out) = run_src(src);
        assert!(r.is_ok(), "main should return Result(Err), got {:?}", r.err());
        assert_eq!(out, "");
        assert!(matches!(r, Ok(Value::Result { ok: false, .. })), "main should return Err result");
    }

    #[test]
    fn match_arm_statement_return() {
        let src = "enum J { A B }\nfn f(j: J) -> String {\n    match j {\n        A => { return \"a\" }\n        B => { return \"b\" }\n    }\n}\nfn main() { print(f(A))\nprint(f(B)) }";
        let out = run_ok(src);
        assert_eq!(out, "a\nb\n");
    }

    #[test]
    fn unit_variant_pattern_and_value() {
        let src = "enum Color { Red Green }\nfn name(c: Color) -> String {\n    match c {\n        Red => \"red\"\n        Green => \"green\"\n    }\n}\nfn main() { print(name(Red))\nprint(name(Green)) }";
        let out = run_ok(src);
        assert_eq!(out, "red\ngreen\n");
    }

    #[test]
    fn string_concatenation_typed() {
        let out = run_ok("fn main() { let a = \"foo\"\nlet b = a + \"bar\"\nprint(b) }");
        assert_eq!(out, "foobar\n");
    }

    #[test]
    fn some_none_patterns() {
        let src = "fn classify(o: Option<i32>) -> String {\n    match o {\n        Some(x) => f\"some {x}\"\n        None => \"none\"\n    }\n}\nfn main() { print(classify(Some(5)))\nprint(classify(None)) }";
        let out = run_ok(src);
        assert_eq!(out, "some 5\nnone\n");
    }

    #[test]
    fn parser_expr_aine_runs() {
        let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "parser_expr.aine"].iter().collect();
        let src = std::fs::read_to_string(&path).unwrap();
        let (r, out) = run_src(&src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        assert!(out.contains("(1 + (2 * 3))"), "precedence: {out}");
        assert!(out.contains("((1 + 2) * 3)"), "parens: {out}");
        assert!(out.contains("(3.14 + x)"), "float: {out}");
    }

    #[test]
    fn mut_params_can_be_reassigned() {
        let out = run_ok("fn sum_to(mut n: i32) -> i32 {\n    var total = 0\n    while n > 0 {\n        total += n\n        n -= 1\n    }\n    return total\n}\nfn main() { print(sum_to(4)) }");
        assert_eq!(out, "10\n");
    }

    #[test]
    fn return_in_if_block_match_propagates() {
        let src = "fn classify(v: Option<i32>) -> String {\n    let after = \"x\"\n    if after == \"x\" {\n        match v {\n            Some(n) => { return \"some \" + f\"{n}\" }\n            None => { return \"none\" }\n        }\n    }\n    return \"fallthrough\"\n}\nfn main() { print(classify(Some(5)))\nprint(classify(None)) }";
        let out = run_ok(src);
        assert_eq!(out, "some 5\nnone\n");
    }

    #[test]
    fn return_in_expression_block_propagates() {
        let src = "fn f(x: i32) -> i32 {\n    let y = { if x > 0 { return x + 100 } else { return x } }\n    return y\n}\nfn main() { print(f(5))\nprint(f(-1)) }";
        let (r, out) = run_src(src);
        assert!(r.is_ok(), "runtime error: {:?}", r.err());
        assert_eq!(out, "105\n-1\n");
    }

    #[test]
    fn question_mark_in_deep_call_chain_no_overflow() {
        let src = "fn inner(s: String) -> Result<i32, String> {\n    if s.len() > 10 {\n        return Err(\"long\")\n    }\n    return Ok(s.len())\n}\nfn mid(s: String) -> Result<i32, String> {\n    let v = inner(s)?\n    return Ok(v + 1)\n}\nfn top(s: String) -> Result<i32, String> {\n    let v = mid(s)?\n    return Ok(v + 1)\n}\nfn main() -> Result<(), String> {\n    let r = top(\"abc\")?\n    print(r)\n    return Ok(())\n}";
        let out = run_ok(src);
        assert_eq!(out, "5\n");
    }

    #[test]
    fn deep_recursion_no_overflow_after_clone_fix() {
        let out = run_ok("fn deep(x: i32) -> i32 {\n    if x <= 0 {\n        return 0\n    }\n    return deep(x - 1) + 1\n}\nfn main() { print(deep(100)) }");
        assert_eq!(out, "100\n");
    }

    #[test]
    fn mutual_recursion_threshold_100_ok() {
        let src = "fn a(x: i32) -> Result<i32, String> {\n    if x > 100 {\n        return Err(\"deep\")\n    }\n    return b(x + 1)\n}\nfn b(x: i32) -> Result<i32, String> {\n    return a(x)\n}\nfn main() {\n    print(a(0))\n}";
        let out = run_ok(src);
        assert_eq!(out, "Err(deep)\n");
    }

    // ---- M23 测试框架 ----

    #[test]
    fn test_attribute_propagates_to_ast() {
        let src = "@test\nfn t_ok() {\n}\nfn helper() {\n}";
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let mut found_test = false;
        let mut helper_clean = false;
        for item in &program.items {
            if let crate::ast::Item::Fn(f) = item {
                if f.sig.name == "t_ok" {
                    assert!(f.attributes.contains(&"test".to_string()), "attrs: {:?}", f.attributes);
                    found_test = true;
                }
                if f.sig.name == "helper" {
                    assert!(!f.attributes.contains(&"test".to_string()), "helper 不应带 test 属性");
                    helper_clean = true;
                }
            }
        }
        assert!(found_test && helper_clean);
    }

    #[test]
    fn test_framework_discovery_and_run() {
        let src = "@test\nfn t_pass() -> Result<i32, String> {\n    return Ok(7)\n}\n@test\nfn t_fail() -> Result<i32, String> {\n    return Err(\"1+1 竟然不等于 3\")\n}\nfn not_a_test() -> i32 {\n    return 1\n}";
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let mut interp = Interp::new();
        interp.load(&program);
        let names: Vec<String> = interp.test_names().to_vec();
        assert_eq!(names, vec!["t_pass".to_string(), "t_fail".to_string()], "只收集 @test，保持源码顺序");
        let ok = interp.call_named("t_pass", Vec::new());
        assert!(matches!(ok, Ok(Value::Result { ok: true, .. })), "t_pass 应为 Ok: {:?}", ok);
        let bad = interp.call_named("t_fail", Vec::new());
        assert!(matches!(bad, Ok(Value::Result { ok: false, .. })), "t_fail 应为 Err: {:?}", bad);
    }

    #[test]
    fn test_suite_example_all_pass() {
        // M23 验收：examples/testdemo.aine 的 5 个 @test 全部通过
        let path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "testdemo.aine"].iter().collect();
        let src = crate::read_source_with_modules(&path).unwrap();
        let lexed = Lexer::new(&src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "testdemo.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse: {:?}", parsed.diagnostics.diagnostics);
        let program = parsed.program.unwrap();
        let mut interp = Interp::new();
        interp.load(&program);
        let names: Vec<String> = interp.test_names().to_vec();
        assert_eq!(names.len(), 5, "应有 5 个测试: {:?}", names);
        for n in names {
            let r = interp.call_named(&n, Vec::new());
            assert!(r.is_ok(), "{} 运行时错误: {:?}", n, r.err());
            assert!(matches!(r.unwrap(), Value::Result { ok: true, .. }), "{} 应通过", n);
        }
    }
}
