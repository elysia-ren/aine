//! Conformance 负向诊断集 + Value Aine/Summary API 级断言（M2 · cargo test）。
//!
//! 与 conformance/*.aine（正向行为集，aine test 运行）互补：
//! 这里覆盖"错误程序必须被拒绝且报错可理解"与"值流摘要的机器可读契约"。

#[cfg(test)]
mod tests {
    use crate::diagnostics::codes;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::resolve::Resolver;
    use crate::typeck::TypeChecker;
    use crate::valueal::{size_class, ty_is_copy, SizeClass, ValueFlow};

    /// 完整前端流水线（无模块内联），返回 (词法, 解析, 类型检查)。
    fn front(src: &str) -> (crate::lexer::Lexed, crate::resolve::Resolved, crate::typeck::TypeckResult) {
        let lexed = Lexer::new(src).lex();
        let parsed = Parser::new(&lexed.tokens, "conf.aine").parse_program();
        let program = parsed.program.unwrap();
        let resolved = Resolver::new(&lexed.tokens).resolve(&program);
        let typeck = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens).check();
        (lexed, resolved, typeck)
    }

    fn codes_of(src: &str) -> Vec<String> {
        let (_, _, t) = front(src);
        t.diagnostics
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect()
    }

    // ---- 负向集: 错误程序必须被拒绝，且错误码稳定 ----

    #[test]
    fn neg_let_type_mismatch_rejected() {
        let cs = codes_of("fn main() {\n    let x: i32 = \"oops\"\n    print(x)\n}");
        assert!(cs.iter().any(|c| c == codes::LET_TYPE_MISMATCH), "codes: {:?}", cs);
    }

    #[test]
    fn neg_arg_count_mismatch_rejected() {
        let cs = codes_of("fn add(a: i32, b: i32) -> i32 {\n    return a + b\n}\nfn main() {\n    print(add(1))\n}");
        assert!(cs.iter().any(|c| c == codes::ARG_COUNT_MISMATCH), "codes: {:?}", cs);
    }

    #[test]
    fn neg_undefined_name_with_suggestion() {
        let (_, r, _) = front("fn greet() {\n    print(\"hi\")\n}\nfn main() {\n    greet2()\n}");
        let d = r
            .diagnostics
            .diagnostics
            .iter()
            .find(|d| d.code == codes::UNDEFINED_NAME)
            .expect("应有 N3001");
        let sug = d.suggestion.clone().unwrap_or_default();
        assert!(sug.contains("greet"), "建议应含 did-you-mean: {}", sug);
    }

    #[test]
    fn neg_non_bool_condition_rejected() {
        let cs = codes_of("fn main() {\n    if 1 {\n        print(\"x\")\n    }\n}");
        assert!(cs.iter().any(|c| c == codes::NON_BOOL_CONDITION), "codes: {:?}", cs);
    }

    #[test]
    fn neg_unknown_field_rejected() {
        let cs = codes_of("struct P {\n    x: i32,\n}\nfn main() {\n    let p = P { x: 1, y: 2 }\n    print(p.x)\n}");
        assert!(
            cs.iter().any(|c| c == codes::DUPLICATE_FIELD || c == codes::UNKNOWN_FIELD),
            "多余字段应报 T3006/T3009: {:?}",
            cs
        );
    }

    #[test]
    fn neg_good_program_clean() {
        let (_, _, t) = front("fn f(n: i32) -> i32 {\n    return n * 2\n}\nfn main() {\n    print(f(21))\n}");
        assert!(!t.diagnostics.has_errors(), "正确程序不应有错误: {:?}", t.diagnostics.diagnostics);
    }

    // ---- Value Aine / Summary 子集: 摘要的机器可读契约 ----

    const VF_SRC: &str = "\
fn total(v: Vec<i32>) -> i32 {
    let mut t = 0
    let mut i = 0
    while i < v.len() {
        t += v[i]
        i += 1
    }
    return t
}
fn name_of(n: i32) -> String {
    return f\"n={n}\"
}
fn main() {
    let v = [1, 2, 3]
    print(total(v))
    print(name_of(7))
}";

    #[test]
    fn va_summaries_cover_all_fns() {
        let (lexed, resolved, typeck) = front(VF_SRC);
        let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
        for name in ["total", "name_of", "main"] {
            assert!(vf.summaries.contains_key(name), "缺少 {} 的函数摘要", name);
        }
    }

    #[test]
    fn va_summary_param_types_and_purity() {
        let (lexed, resolved, typeck) = front(VF_SRC);
        let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
        let total = &vf.summaries["total"];
        assert_eq!(total.params.len(), 1);
        assert_eq!(total.params[0].ty.display(), "Vec<i32>");
        assert!(total.params[0].name == "v");
        assert!(!total.has_tasks, "纯函数不产生任务");
        assert!(!total.writes_state, "纯函数不写全局状态");
        assert!(total.ret.is_some(), "有返回值函数的摘要应带返回来源");
        let main = &vf.summaries["main"];
        assert!(main.params.is_empty(), "main 无参");
    }

    #[test]
    fn va_vir_present() {
        let (lexed, resolved, typeck) = front(VF_SRC);
        let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
        assert!(vf.vfns.len() >= 3, "每个函数应有 VIR: {:?}", vf.vfns.iter().map(|v| v.name.clone()).collect::<Vec<_>>());
        for v in &vf.vfns {
            assert!(!v.nodes.is_empty(), "{} 的 VIR 节点不应为空", v.name);
        }
    }

    #[test]
    fn va_no_ownership_errors_on_conforming_program() {
        let (lexed, resolved, typeck) = front(VF_SRC);
        let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
        assert!(!vf.diagnostics.has_errors(), "值语义合规程序不应有所有权错误: {:?}", vf.diagnostics.diagnostics);
    }

    #[test]
    fn va_copy_and_size_classification() {
        // 类型分类契约: 标量为 Copy/Cheap; Vec/String 为所有权类型
        assert!(ty_is_copy(&crate::typeck::Ty::I32));
        assert!(ty_is_copy(&crate::typeck::Ty::Bool));
        assert!(!ty_is_copy(&crate::typeck::Ty::Str));
        assert!(!ty_is_copy(&crate::typeck::Ty::Vec(Box::new(crate::typeck::Ty::I32))));
        assert_eq!(size_class(&crate::typeck::Ty::I32), SizeClass::Cheap);
    }
}
