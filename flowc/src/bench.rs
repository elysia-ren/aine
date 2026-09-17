//! M3: large-object validation (design doc §71.4 / §74).
//!
//! Since aine has no codegen/runtime yet, "large objects" are validated at
//! the ANALYSIS level: synthetic programs shaped like 10MB/100MB/1GB
//! workloads exercise the Borrow/Materialize machinery, and the §74 metrics
//! (Borrow Success / Materialization Count / Large Copy Count / summary
//! count / analysis time) are collected with exact assertions.
//!
//! §71.4 acceptance: for every materialization the report must answer
//! 是否借用 / 是否物化 / 为什么 / 成本多少 / 如何进入高级优化路径.

use std::time::Instant;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::resolve::Resolver;
use crate::typeck::TypeChecker;
use crate::valueal::{OptDecision, SizeClass, ValueFlow};

/// Generate a deterministic synthetic program with the given iteration count.
/// Each block exercises: view reuse (Borrow), consuming callee (Materialize),
/// owned value (no copy), return materialization, task-capture materialization.
pub fn generate(iterations: usize) -> String {
    let mut out = String::new();
    for i in 0..iterations {
        out.push_str(&format!("struct User_{i} {{\n    name: String\n}}\n"));
        out.push_str(&format!("fn get_name_{i}(user: User_{i}) -> String {{\n    user.name\n}}\n"));
        out.push_str(&format!("fn burn_{i}(s: String) {{ }}\n"));
        out.push_str(&format!("fn stash_{i}(s: String) {{\n    go {{\n        burn_{i}(s)\n    }}\n}}\n"));
        out.push_str(&format!("fn save_{i}(s: String) {{ }}\n"));
        out.push_str(&format!("fn load_{i}() -> String {{ \"data\" }}\n"));
        out.push_str(&format!("fn use_{i}(u: User_{i}) {{\n    let a = get_name_{i}(u)\n    save_{i}(a)\n    let b = get_name_{i}(u)\n    stash_{i}(b)\n    let c = load_{i}()\n    stash_{i}(c)\n}}\n"));
    }
    out
}
/// §74 metrics collected from a benchmark run.
#[derive(Debug, Clone)]
pub struct BenchReport {
    pub iterations: usize,
    pub functions: usize,
    pub analysis_ms: f64,
    pub borrow_success: usize,
    pub materialization_count: usize,
    pub large_copy_count: usize,
    pub owned: usize,
    pub copies: usize,
    pub summaries: usize,
    /// materialization sites for the §71.4 report
    pub materialized: Vec<OptDecision>,
}

/// Run the full frontend + valueal on a source and collect §74 metrics.
pub fn run(source: &str) -> BenchReport {
    let start = Instant::now();
    let lexed = Lexer::new(source).lex();
    let parsed = Parser::new(&lexed.tokens, "bench.aine").parse_program();
    let program = parsed.program.expect("program");
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    let analysis_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut borrow_success = 0;
    let mut materialization_count = 0;
    let mut large_copy_count = 0;
    let mut owned = 0;
    let mut copies = 0;
    let mut materialized = Vec::new();
    for d in &vf.decisions {
        match d {
            OptDecision::Borrow { .. } => borrow_success += 1,
            OptDecision::Materialized { class, .. } => {
                materialization_count += 1;
                if *class == SizeClass::Large {
                    large_copy_count += 1;
                }
                materialized.push(d.clone());
            }
            OptDecision::Owned => owned += 1,
            OptDecision::Copy => copies += 1,
        }
    }
    BenchReport {
        iterations: source.matches("struct User_").count(),
        functions: count_fns(source),
        analysis_ms,
        borrow_success,
        materialization_count,
        large_copy_count,
        owned,
        copies,
        summaries: vf.summaries.len(),
        materialized,
    }
}

fn count_fns(source: &str) -> usize {
    source.matches("fn ").count()
}

/// §71.4 cost report: 是否借用 / 是否物化 / 为什么 / 成本多少 / 如何优化.
pub fn cost_report(r: &BenchReport) -> String {
    let mut out = String::new();
    out.push_str("=== Aine M3 大对象验证报告 ===\n");
    out.push_str(&format!("程序: synthetic ({} 迭代, {} 个函数)\n", r.iterations, r.functions));
    out.push_str(&format!("分析时间: {:.2} ms\n", r.analysis_ms));
    out.push_str("\n指标 (§74):\n");
    out.push_str(&format!("  Borrow Success:      {}\n", r.borrow_success));
    out.push_str(&format!("  Materialization:     {}\n", r.materialization_count));
    out.push_str(&format!("  Large Copy Count:    {}\n", r.large_copy_count));
    out.push_str(&format!("  Owned:               {}\n", r.owned));
    out.push_str(&format!("  Copy:                {}\n", r.copies));
    out.push_str(&format!("  函数摘要数:          {}\n", r.summaries));
    out.push_str("\n物化明细 (§71.4):\n");
    if r.materialized.is_empty() {
        out.push_str("  （无物化）\n");
    }
    for (i, d) in r.materialized.iter().enumerate().take(20) {
        if let OptDecision::Materialized { span, class } = d {
            out.push_str(&format!("  [{}] 位置 byte {}..{}:\n", i, span.start, span.end));
            out.push_str("    是否借用: 否（视图不可用）\n");
            out.push_str("    是否物化: 是\n");
            out.push_str(&format!("    成本: {}（类型级保守估计，运行时实测待 codegen）\n", class.display()));
            out.push_str("    为什么: 输入派生值被存储/转移，无法安全借用\n");
            out.push_str("    如何进入高级优化路径: 拆分函数 / 显式 &T 高级借用 API\n");
        }
    }
    if r.materialized.len() > 20 {
        out.push_str(&format!("  ... 其余 {} 处略\n", r.materialized.len() - 20));
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_counts_are_exact() {
        // per iteration: Borrow 7, Materialized 3 (all Large), Copy 1
        for n in [1usize, 3, 10] {
            let src = generate(n);
            let r = run(&src);
            assert_eq!(r.borrow_success, 7 * n, "borrow for n={n}");
            assert_eq!(r.materialization_count, 3 * n, "materialized for n={n}");
            assert_eq!(r.large_copy_count, 3 * n, "large copies for n={n}");
            assert_eq!(r.copies, 1 * n, "copies for n={n}");
            assert_eq!(r.summaries, 6 * n, "summaries for n={n}");
            assert_eq!(r.functions, 6 * n, "functions for n={n}");
        }
    }

    #[test]
    fn large_program_analysis_is_fast_and_scales() {
        // 1200 functions must analyze in well under a second (§75 compile time)
        let src = generate(200);
        let r = run(&src);
        assert_eq!(r.functions, 1200);
        assert_eq!(r.borrow_success, 1400);
        assert_eq!(r.materialization_count, 600);
        assert!(r.analysis_ms < 1000.0, "analysis took {:.1} ms", r.analysis_ms);
    }

    #[test]
    fn cost_report_answers_71_4_questions() {
        let r = run(&generate(2));
        let report = cost_report(&r);
        for field in [
            "是否借用",
            "是否物化",
            "成本",
            "为什么",
            "如何进入高级优化路径",
            "Borrow Success",
            "Materialization",
            "Large Copy Count",
        ] {
            assert!(report.contains(field), "missing field {field} in report");
        }
    }

    #[test]
    fn examples_bench_metrics() {
        for name in ["hello.aine", "account_book.aine"] {
            let path: std::path::PathBuf =
                [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
            let src = std::fs::read_to_string(&path).unwrap();
            let r = run(&src);
            // the canonical examples must not contain large-copy materializations
            assert_eq!(r.large_copy_count, 0, "{name} has large copies");
            assert_eq!(r.materialization_count, 0, "{name} has materializations");
        }
    }

    #[test]
    fn generated_source_is_valid() {
        let src = generate(2);
        let lexed = Lexer::new(&src).lex();
        assert!(!lexed.diagnostics.has_errors(), "{:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "gen.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "{:?}", parsed.diagnostics.diagnostics);
    }
}
