//! Profiler（M7 §5 DoD 7）：`aine profile <file>`
//! 分析层六区（CPU=流水线耗时 / Memory=大对象物化 / Allocation=物化与拷贝决策 /
//! Borrow=视图决策）+ Aine 专属（Materialization / Large Copy / Borrow 结果）。
//! 运行时分配计数经 AINE_ALLOC_STATS（原生执行时）。

use std::time::Instant;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::{resolve::Resolver, resolve_file_modules};
use crate::typeck::TypeChecker;
use crate::valueal::{OptDecision, SizeClass, ValueFlow};

pub fn run_profile(path: &str) -> std::process::ExitCode {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return std::process::ExitCode::FAILURE;
        }
    };
    let t0 = Instant::now();
    let lexed = Lexer::new(&source).lex();
    let t1 = Instant::now();
    let mut program = match Parser::new(&lexed.tokens, path).parse_program() {
        p if p.diagnostics.has_errors() => {
            for d in &p.diagnostics.diagnostics {
                eprint!("{}", crate::diagnostics::render(d, &source, path));
            }
            return std::process::ExitCode::FAILURE;
        }
        p => p.program.unwrap(),
    };
    let t2 = Instant::now();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return std::process::ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    let t3 = Instant::now();
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    let t4 = Instant::now();
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    let t5 = Instant::now();
    for d in &vf.diagnostics.diagnostics {
        eprint!("{}", crate::diagnostics::render(d, &source, path));
    }

    let mut mat = 0usize;
    let mut large_copy = 0usize;
    let mut borrow = 0usize;
    let mut owned = 0usize;
    for d in &vf.decisions {
        match d {
            OptDecision::Materialized { class, .. } => {
                mat += 1;
                if *class == SizeClass::Large {
                    large_copy += 1;
                }
            }
            OptDecision::Borrow { .. } => borrow += 1,
            OptDecision::Owned => owned += 1,
            OptDecision::Copy => {}
        }
    }
    let tokens = lexed.tokens.len();
    let fns = vf.summaries.len();
    let vir_nodes: usize = vf.vfns.iter().map(|v| v.nodes.len()).sum();
    let errors = vf.diagnostics.error_count();
    let warnings = vf.diagnostics.diagnostics.len() - errors;

    println!("== Aine Profiler 报告 ==");
    println!("程序: {}", path);
    println!("流水线耗时(CPU 区):");
    println!("  lexer      {:>8.2} ms", t1.duration_since(t0).as_secs_f64() * 1000.0);
    println!("  parser     {:>8.2} ms", t2.duration_since(t1).as_secs_f64() * 1000.0);
    println!("  resolve    {:>8.2} ms", t3.duration_since(t2).as_secs_f64() * 1000.0);
    println!("  typeck     {:>8.2} ms", t4.duration_since(t3).as_secs_f64() * 1000.0);
    println!("  valueal    {:>8.2} ms", t5.duration_since(t4).as_secs_f64() * 1000.0);
    println!("  合计       {:>8.2} ms", t5.duration_since(t0).as_secs_f64() * 1000.0);
    println!("规模:");
    println!("  tokens={} 函数={} VIR 节点={}", tokens, fns, vir_nodes);
    println!("决策(Memory/Allocation/Borrow 区):");
    println!("  Borrow(零拷贝视图)   {}", borrow);
    println!("  Materialized         {}", mat);
    println!("  Owned                {}", owned);
    println!("Aine 专属:");
    println!("  Large Copy           {}", large_copy);
    println!("  Materialization 提示  {}", mat);
    println!("诊断: errors={} warnings={}", errors, warnings);
    if errors > 0 {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}
