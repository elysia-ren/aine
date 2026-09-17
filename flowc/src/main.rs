//! aine 命令行入口。
//!
//! 用法：
//!   aine lex <file>     输出词法分析结果（token 流）
//!   aine check <file>   检查源码（当前为词法 + 后续阶段占位）
//!   aine --version      输出版本
//!   aine --help         帮助

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use aine::diagnostics::{render, Severity};
use aine::lexer::Lexer;
use aine::parser::Parser;
use aine::resolve::Resolver;
use aine::typeck::TypeChecker;
use aine::valueal::ValueFlow;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() -> String {
    String::from(
        "aine — Aine 语言编译器
         
         用法:
           aine lex <file>     词法分析并输出 token 流
           aine ast <file>     词法+语法分析并输出 AST
           aine hir <file>     HIR + 符号表 + 名称解析
           aine typeck <file>  类型检查 + 类型映射
           aine fmt <file>     格式化并输出规范源码
           aine vf <file>      值流分析（VIR + 摘要 + 所有权）
           aine opt <file>     优化决策（Borrow/Materialize + 性能提示）
           aine bench <n>     大对象基准（§74 指标 + §71.4 成本报告）
           aine run <file>    解释执行 Aine 程序
           aine test <file>   运行文件内的 #[test] 函数并汇总结果
           aine build <file>  编译为原生可执行（Aine 转译器 + zig cc）
           aine run <file> [args]  运行程序，args 经全局 args 传入
           aine lsp           语言服务器（LSP over stdio）
           aine profile <file>  性能分析报告（六区 + Aine 专属）
           aine debug <file> <行号...>  断点执行并报告局部变量
           aine check <file>   完整检查（词法+语法+名称解析）
           aine --version      输出版本
           aine --help         显示帮助
",
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print!("{}", usage());
        return ExitCode::SUCCESS;
    }

    match args[0].as_str() {
        "--help" | "-h" => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        "--version" | "-V" => {
            println!("aine {}", VERSION);
            ExitCode::SUCCESS
        }
        "lex" => {
            if args.len() < 2 {
                eprintln!("error: aine lex 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_lex(&args[1])
        }
        "ast" => {
            if args.len() < 2 {
                eprintln!("error: aine ast 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_ast(&args[1])
        }
        "hir" => {
            if args.len() < 2 {
                eprintln!("error: aine hir 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_hir(&args[1])
        }
        "typeck" => {
            if args.len() < 2 {
                eprintln!("error: aine typeck 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_typeck(&args[1])
        }
        "fmt" => {
            if args.len() < 2 {
                eprintln!("error: aine fmt 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_fmt(&args[1])
        }
        "lsp" => {
            return aine::lsp::run();
        }
        "debug" => {
            if args.len() < 2 {
                eprintln!("error: aine debug 需要一个文件路径与断点行号");
                return ExitCode::FAILURE;
            }
            let lines: Vec<usize> = args[2..].iter().filter_map(|a| a.parse().ok()).collect();
            return aine::debugger::run_debug(&args[1], &lines);
        }
        "profile" => {
            if args.len() < 2 {
                eprintln!("error: aine profile 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            return aine::profile::run_profile(&args[1]);
        }
        "vf" => {
            if args.len() < 2 {
                eprintln!("error: aine vf 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_vf(&args[1])
        }
        "opt" => {
            if args.len() < 2 {
                eprintln!("error: aine opt 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_opt(&args[1])
        }
        "bench" => {
            let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
            run_bench(n)
        }
        "run" => {
            if args.len() < 2 {
                eprintln!("error: aine run 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            let prog_args: Vec<String> = args[2..].to_vec();
            run_interp(&args[1], prog_args)
        }
        "build" => {
            if args.len() < 2 {
                eprintln!("error: aine build 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            // -o 输出路径（可选）
            let out = args.iter().position(|a| a == "-o").and_then(|i| args.get(i + 1).cloned());
            run_build(&args[1], out.as_deref())
        }
        "test" => {
            if args.len() < 2 {
                eprintln!("error: aine test 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_test(&args[1])
        }
        "check" => {
            if args.len() < 2 {
                eprintln!("error: aine check 需要一个文件路径");
                return ExitCode::FAILURE;
            }
            run_check(&args[1])
        }
        other => {
            eprintln!("error: 未知子命令 '{}'", other);
            eprintln!("{}", usage());
            ExitCode::FAILURE
        }
    }
}

fn load_source(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    // 文件模块：`mod name;` 文本级内联为虚拟合并源（span 同一坐标系）
    aine::read_source_with_modules(p)
}

/// 原始读取（不做模块内联）—— fmt 用，保留用户书写的 `mod name;`
fn load_source_raw(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    fs::read_to_string(p).map_err(|e| format!("无法读取文件 {}: {}", path, e))
}

fn run_lex(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();

    // 输出诊断（若有）
    let mut failed = false;
    for d in &lexed.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            failed = true;
        }
        eprint!("{}", render(d, &source, path));
    }

    // 输出 token 流
    println!("== token 流 ({}) ==", lexed.tokens.len());
    for t in &lexed.tokens {
        let value_desc = match &t.value {
            aine::token::TokenValue::None => String::new(),
            aine::token::TokenValue::Int(v) => format!("  value={}", v),
            aine::token::TokenValue::Float(v) => format!("  value={}", v),
            aine::token::TokenValue::Str(s) => format!("  value='{}'", s),
        };
        println!(
            "{:>5}:{:<3} {:>10} '{}'{}",
            t.loc.line, t.loc.col, t.kind.describe(), t.text, value_desc
        );
    }

    if failed {
        eprintln!("lex: 发现 {} 个错误", lexed.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!("lex: OK（0 错误）");
        ExitCode::SUCCESS
    }
}

fn run_ast(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    for d in &lexed.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    if lexed.diagnostics.has_errors() {
        eprintln!("ast: 词法阶段失败，无法生成 AST");
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    for d in &parsed.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    if parsed.diagnostics.has_errors() {
        eprintln!("ast: 语法阶段失败（{} 个错误）", parsed.diagnostics.error_count());
        return ExitCode::FAILURE;
    }
    if let Some(mut program) = parsed.program {
        let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
        if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
        println!("{:#?}", program);
    }
    println!("ast: OK");
    ExitCode::SUCCESS
}

fn run_hir(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    for d in &lexed.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    if lexed.diagnostics.has_errors() {
        eprintln!("hir: 词法阶段失败");
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    for d in &parsed.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    if parsed.diagnostics.has_errors() {
        eprintln!("hir: 语法阶段失败");
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    for d in &resolved.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    println!("== 符号表 ({}) ==", resolved.program.symbols.len());
    for s in &resolved.program.symbols {
        println!("  [{}] {:?} {}", s.id, s.kind, s.name);
    }
    println!("== 解析引用 ({}) ==", resolved.resolution.idents.len());
    println!("== HIR 顶层项目 ({}) ==", resolved.program.items.len());
    println!("{:#?}", resolved.program.items);
    if resolved.diagnostics.has_errors() {
        eprintln!("hir: 语义阶段失败（{} 个错误）", resolved.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!(
            "hir: OK（{} 符号，{} 解析引用，{} 警告）",
            resolved.program.symbols.len(),
            resolved.resolution.idents.len(),
            resolved.diagnostics.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
        );
        ExitCode::SUCCESS
    }
}

fn run_typeck(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        for d in &parsed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    // print type map
    let mut entries: Vec<(usize, usize, aine::typeck::Ty)> = typeck
        .types
        .iter()
        .map(|(s, t)| (s.start, s.end, t.clone()))
        .collect();
    entries.sort_by_key(|(start, _, _)| *start);
    println!("== 类型映射 ({}) ==", entries.len());
    for (start, end, t) in entries.iter().take(60) {
        println!("  byte {}..{}: {}", start, end, t.display());
    }
    for d in &typeck.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    if typeck.diagnostics.has_errors() {
        eprintln!("typeck: 失败（{} 个类型错误）", typeck.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!("typeck: OK（0 个类型错误）");
        ExitCode::SUCCESS
    }
}

fn run_interp(path: &str, prog_args: Vec<String>) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    // full frontend must pass before executing
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        for d in &parsed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    if resolved.diagnostics.has_errors() {
        for d in &resolved.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    if typeck.diagnostics.has_errors() {
        for d in &typeck.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    // interpret (on a large-stack thread: the tree-walking interpreter
    // recurses per call, so a big stack is required for deep recursion)
    let child = std::thread::Builder::new()
        .name("aine-interp".into())
        .stack_size(512 * 1024 * 1024)
        .spawn(move || -> Result<aine::interp::Value, aine::interp::RtError> {
            let mut interp = aine::interp::Interp::new();
            interp.load(&program);
            let argv: Vec<aine::interp::Value> =
                prog_args.into_iter().map(|a| aine::interp::Value::Str(a.into())).collect();
            interp.inject_global("cli_args", aine::interp::Value::Vec(std::sync::Arc::new(argv)));
            interp.run_main()
        })
        .expect("spawn interp thread");
    match child.join() {
        Ok(Ok(v)) => {
            // main returning Err is a program failure (design: Result)
            if let aine::interp::Value::Result { ok: false, value } = &v {
                eprintln!("程序失败: Err({})", value.display());
                return ExitCode::FAILURE;
            }
            if !matches!(v, aine::interp::Value::Unit) {
                println!("{}", v.display());
            }
            ExitCode::SUCCESS
        }
        Ok(Err(e)) => {
            eprintln!("运行时错误: {}", aine::interp::rt_error_text(&e));
            ExitCode::FAILURE
        }
        Err(_) => {
            eprintln!("解释器线程失败: 线程 panic");
            ExitCode::FAILURE
        }
    }
}

/// 前端流水线：读源（含模块内联）→ 词法 → 语法 → 名称解析 → 类型检查。
/// 任一阶段有错误即返回 Err，内容为已渲染的诊断全文（直接输出即可）。
fn front_pipeline(path: &str, with_typeck: bool) -> Result<(String, aine::ast::Program), String> {
    let source = load_source(path)?;
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        let mut text = String::new();
        for d in &lexed.diagnostics.diagnostics {
            text.push_str(&render(d, &source, path));
        }
        return Err(text);
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        let mut text = String::new();
        for d in &parsed.diagnostics.diagnostics {
            text.push_str(&render(d, &source, path));
        }
        return Err(text);
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        return Err(format!("error: {}", e));
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    if resolved.diagnostics.has_errors() {
        let mut text = String::new();
        for d in &resolved.diagnostics.diagnostics {
            text.push_str(&render(d, &source, path));
        }
        return Err(text);
    }
    // 全 Aine 迁移 Phase 2: build 的类型检查由 Aine tck_program 承担(跳过 Rust typeck)
    if !with_typeck {
        return Ok((source, program));
    }
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    if typeck.diagnostics.has_errors() {
        let mut text = String::new();
        for d in &typeck.diagnostics.diagnostics {
            text.push_str(&render(d, &source, path));
        }
        return Err(text);
    }
    Ok((source, program))
}

/// 测试框架：收集文件内的 #[test] 函数，逐个在独立解释器中运行并汇总。
/// 每个测试一个全新 Interp + 大栈线程：互相隔离、互不污染，失败不影响后续测试。
fn run_test(path: &str) -> ExitCode {
    let (_source, program) = match front_pipeline(path, true) {
        Ok(t) => t,
        Err(text) => {
            eprint!("{}", text);
            return ExitCode::FAILURE;
        }
    };
    let mut probe = aine::interp::Interp::new();
    probe.load(&program);
    let names: Vec<String> = probe.test_names().to_vec();
    if names.is_empty() {
        eprintln!("没有找到 #[test] 函数。");
        eprintln!("  = 建议: 在要测试的函数前加一行 #[test]，例如:");
        eprintln!("    #[test]");
        eprintln!("    fn add_works() {{");
        eprintln!("        assert(add(1, 2) == 3)");
        eprintln!("    }}");
        return ExitCode::FAILURE;
    }
    let program = std::sync::Arc::new(program);
    println!("运行 {} 个测试 ({}):", names.len(), path);
    let mut passed = 0usize;
    let mut failed = 0usize;
    for name in &names {
        let prog = program.clone();
        let tname = name.clone();
        let child = std::thread::Builder::new()
            .name(format!("aine-test-{}", tname))
            .stack_size(512 * 1024 * 1024)
            .spawn(move || -> Result<aine::interp::Value, String> {
                let mut interp = aine::interp::Interp::new();
                interp.load(&prog);
                interp.call_named(&tname, Vec::new()).map_err(|e| aine::interp::rt_error_text(&e))
            })
            .expect("spawn test thread");
        match child.join() {
            Ok(Ok(v)) => {
                // 测试函数返回 Result 且为 Err 视为失败（design: Result 语义）
                if let aine::interp::Value::Result { ok: false, value } = &v {
                    println!("  ✗ {}", name);
                    println!("      原因: 返回 Err({})", value.display());
                    failed += 1;
                } else {
                    println!("  ✓ {}", name);
                    passed += 1;
                }
            }
            Ok(Err(msg)) => {
                println!("  ✗ {}", name);
                println!("      原因: {}", msg);
                failed += 1;
            }
            Err(_) => {
                println!("  ✗ {}", name);
                println!("      原因: 测试线程 panic（解释器内部错误，其余测试已隔离不受影响）");
                failed += 1;
            }
        }
    }
    println!("测试结果: {} 通过, {} 失败 (共 {} 个)", passed, failed, names.len());
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// 编译 Aine 文件为原生可执行：Aine 转译器（解释执行）生成 C → zig cc。
/// 实参经全局 args 传给转译器：args[0]=目标文件, args[1]=输出 C 路径。
fn run_build(path: &str, out: Option<&str>) -> ExitCode {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let compiler = root.join("examples").join("transpiler.aine");
    if !compiler.exists() {
        eprintln!("error: 找不到转译器: {}", compiler.display());
        return ExitCode::FAILURE;
    }
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| std::path::PathBuf::from(path));
    if !target.exists() {
        eprintln!("error: 目标文件不存在: {}", path);
        return ExitCode::FAILURE;
    }
    let c_out = match out {
        Some(o) => std::path::PathBuf::from(o),
        None => {
            let stem = target.file_stem().and_then(|s| s.to_str()).unwrap_or("program");
            root.join(format!("{}_gen.c", stem))
        }
    };

    // 工作目录切到 aine 根：转译器的 mod 搜索目录为 examples/（flatten_stmts 约定）
    let prev_cwd = std::env::current_dir().unwrap_or_default();
    if std::env::set_current_dir(root).is_err() {
        eprintln!("error: 无法切换工作目录到 {}", root.display());
        return ExitCode::FAILURE;
    }

    // 1) 前端检查目标文件（先给出面向用户的编译错误，而非转译期报错）
    if let Err(text) = front_pipeline(&target.to_string_lossy(), false) {
        eprint!("{}", text);
        let _ = std::env::set_current_dir(prev_cwd);
        return ExitCode::FAILURE;
    }

    // 2) 解释执行 Aine 转译器：args = [目标文件, 输出 C]
    let compiler_str = compiler.to_string_lossy().to_string();
    let c_out_str = c_out.to_string_lossy().to_string();
    let target_str = target.to_string_lossy().to_string();
    let build = std::thread::Builder::new()
        .name("aine-build".into())
        .stack_size(512 * 1024 * 1024)
        .spawn(move || -> Result<(), String> {
            let (_source, program) = match front_pipeline(&compiler_str, true) {
                Ok(t) => t,
                Err(text) => return Err(text),
            };
            let mut interp = aine::interp::Interp::new();
            interp.load(&program);
            let argv: Vec<aine::interp::Value> = vec![
                aine::interp::Value::Str(target_str.clone().into()),
                aine::interp::Value::Str(c_out_str.clone().into()),
            ];
            interp.inject_global("cli_args", aine::interp::Value::Vec(std::sync::Arc::new(argv)));
            match interp.run_main() {
                Ok(aine::interp::Value::Result { ok: false, value }) => {
                    Err(format!("转译失败: Err({})", value.display()))
                }
                Ok(_) => Ok(()),
                Err(e) => Err(aine::interp::rt_error_text(&e)),
            }
        })
        .expect("spawn build thread");
    let transpiled = match child_join(build) {
        Ok(v) => v,
        Err(code) => {
            let _ = std::env::set_current_dir(prev_cwd);
            return code;
        }
    };
    if let Err(e) = transpiled {
        eprintln!("error: {}", e);
        let _ = std::env::set_current_dir(prev_cwd);
        return ExitCode::FAILURE;
    }

    // 3) zig cc → 原生可执行
    let exe_out = c_out.with_extension("exe");
    let status = std::process::Command::new("zig")
        .args(["cc", "-std=c23", "-O2", "-luser32", "-lgdi32", "-ld2d1", "-ldwrite", "-luuid", "-lole32"])
        .arg("-o")
        .arg(&exe_out)
        .arg(&c_out)
        .status();
    let _ = std::env::set_current_dir(prev_cwd);
    match status {
        Ok(st) if st.success() => {
            println!("构建完成: {}", exe_out.display());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("error: zig cc 编译失败（C 已生成: {}）", c_out.display());
            ExitCode::FAILURE
        }
    }
}

/// 加入构建线程并整理错误
fn child_join(child: std::thread::JoinHandle<Result<(), String>>) -> Result<Result<(), String>, ExitCode> {
    match child.join() {
        Ok(r) => Ok(r),
        Err(_) => {
            eprintln!("error: 构建线程 panic");
            Err(ExitCode::FAILURE)
        }
    }
}

fn run_bench(iterations: usize) -> ExitCode {
    let src = aine::bench::generate(iterations);
    let report = aine::bench::run(&src);
    print!("{}", aine::bench::cost_report(&report));
    ExitCode::SUCCESS
}

fn run_opt(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        for d in &parsed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    if resolved.diagnostics.has_errors() {
        for d in &resolved.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    for d in &vf.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    let mut borrows = 0;
    let mut materialized = 0;
    let mut owned = 0;
    let mut copies = 0;
    println!("== 优化决策 ({}) ==", vf.decisions.len());
    for d in &vf.decisions {
        match d {
            aine::valueal::OptDecision::Borrow { .. } => borrows += 1,
            aine::valueal::OptDecision::Materialized { .. } => materialized += 1,
            aine::valueal::OptDecision::Owned => owned += 1,
            aine::valueal::OptDecision::Copy => copies += 1,
        }
    }
    println!(
        "Borrow(零拷贝视图): {}  Materialized: {}  Owned: {}  Copy: {}",
        borrows, materialized, owned, copies
    );
    if vf.diagnostics.has_errors() {
        eprintln!("opt: 失败（{} 个所有权错误）", vf.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!("opt: OK（{} 个性能提示）", materialized);
        ExitCode::SUCCESS
    }
}

fn run_vf(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        for d in &parsed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    if resolved.diagnostics.has_errors() {
        for d in &resolved.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    if typeck.diagnostics.has_errors() {
        for d in &typeck.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    for d in &vf.diagnostics.diagnostics {
        eprint!("{}", render(d, &source, path));
    }
    println!("== 函数摘要 ({}) ==", vf.summaries.len());
    let mut names: Vec<&String> = vf.summaries.keys().collect();
    names.sort();
    for n in names {
        println!("  {}", vf.summaries.get(n).unwrap().display());
    }
    println!("== VIR 节点 ==");
    for v in &vf.vfns {
        println!("-- {} ({} 节点) --", v.name, v.nodes.len());
        for (i, n) in v.nodes.iter().enumerate() {
            println!("  [{i}] {:?}", n);
        }
    }
    // §72 联合验收指标（M6 MVP：记账本端到端验收口径）
    {
        let mut dangling = 0;
        let mut races = 0;
        let mut illegal_state = 0;
        let mut transfers = 0;
        let mut unnecessary_copy = 0;
        use aine::valueal::vcodes;
        for d in &vf.diagnostics.diagnostics {
            if d.code == vcodes::USE_AFTER_MOVE {
                dangling += 1;
            }
            if d.code == vcodes::NOT_SEND || d.code == vcodes::UI_GO_INSIDE {
                races += 1;
            }
            if d.code == vcodes::STATE_WRITE_OUTSIDE_UI || d.code == vcodes::GLOBAL_WRITE_OUTSIDE_UI {
                illegal_state += 1;
            }
        }
        for v in &vf.vfns {
            for n in &v.nodes {
                if matches!(n, aine::valueal::VirNode::TaskTransfer { .. }) {
                    transfers += 1;
                }
            }
        }
        for d in &vf.decisions {
            if matches!(d, aine::valueal::OptDecision::Materialized { class, .. }
                if *class == aine::valueal::SizeClass::Large)
            {
                unnecessary_copy += 1;
            }
        }
        println!("== §72 联合验收 ==");
        println!("  dangling reference:        {}（目标 0）", dangling);
        println!("  data race:                 {}（目标 0）", races);
        println!("  illegal state access:      {}（目标 0）", illegal_state);
        println!("  ownership transfer:        {}（记账本验收：1）", transfers);
        println!("  unnecessary full copy:     {}（目标 0）", unnecessary_copy);
    }
    if vf.diagnostics.has_errors() {
        eprintln!("vf: 失败（{} 个所有权错误）", vf.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!("vf: OK（0 个所有权错误）");
        ExitCode::SUCCESS
    }
}

fn run_fmt(path: &str) -> ExitCode {
    let source = match load_source_raw(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    if parsed.diagnostics.has_errors() {
        for d in &parsed.diagnostics.diagnostics {
            eprint!("{}", render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let program = parsed.program.unwrap();
    // 格式化逻辑由 Aine 实现（examples/fmt_tool.aine,自举 M6-FMT-2 已对齐 diff=0）
    let formatted = run_fmt_aine(path);
    print!("{}", formatted);
    ExitCode::SUCCESS
}

/// 解释执行 fmt_tool.aine（Aine 原生格式化器）并提取 FMT-BEGIN/END 块
fn run_fmt_aine(path: &str) -> String {
    let tool_path: std::path::PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", "fmt_tool.aine"].iter().collect();
    let tool_src = std::fs::read_to_string(&tool_path).unwrap_or_default();
    let tool_lexed = aine::lexer::Lexer::new(&tool_src).lex();
    let mut tool_program = match aine::parser::Parser::new(&tool_lexed.tokens, "fmt_tool.aine").parse_program() {
        p if p.diagnostics.has_errors() => return "/* fmt_tool 解析失败 */".to_string(),
        p => p.program.unwrap(),
    };
    if let Err(e) = aine::resolve_file_modules(&mut tool_program, tool_path.parent().unwrap_or(std::path::Path::new("."))) {
        return format!("/* fmt_tool 模块加载失败: {} */", e);
    }
    let mut interp = aine::interp::Interp::new();
    interp.load(&tool_program);
    let argv: Vec<aine::interp::Value> = vec![aine::interp::Value::Str(path.to_string().into())];
    interp.inject_global("cli_args", aine::interp::Value::Vec(std::sync::Arc::new(argv)));
    let _ = interp.run_main();
    let out = interp.output.clone();
    let begin = "===FMT-BEGIN===
";
    let end = "===FMT-END===";
    match (out.find(begin), out.find(end)) {
        (Some(b), Some(e)) if e > b => out[b + begin.len()..e].to_string(),
        _ => out,
    }
}

fn run_check(path: &str) -> ExitCode {
    let source = match load_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    // full pipeline: lex -> parse -> hir + name resolution -> type check
    let lexed = Lexer::new(&source).lex();
    let mut failed = false;
    for d in &lexed.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            failed = true;
        }
        eprint!("{}", render(d, &source, path));
    }
    if failed {
        eprintln!("check: 失败（{} 个词法错误）", lexed.diagnostics.error_count());
        return ExitCode::FAILURE;
    }
    let parsed = Parser::new(&lexed.tokens, path).parse_program();
    let mut parse_failed = false;
    for d in &parsed.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            parse_failed = true;
        }
        eprint!("{}", render(d, &source, path));
    }
    if parse_failed {
        eprintln!("check: 失败（{} 个语法错误）", parsed.diagnostics.error_count());
        return ExitCode::FAILURE;
    }
    let mut program = parsed.program.unwrap();
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = aine::resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    let mut sem_failed = false;
    let mut warnings = 0;
    for d in &resolved.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            sem_failed = true;
        }
        if d.severity == Severity::Warning {
            warnings += 1;
        }
        eprint!("{}", render(d, &source, path));
    }
    // 用户导向：语义错误不阻断 typeck —— 一次编译看到全部问题
    let checker = TypeChecker::new(&resolved.program, &resolved.resolution, &lexed.tokens);
    let typeck = checker.check();
    let mut type_failed = false;
    for d in &typeck.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            type_failed = true;
        }
        if d.severity == Severity::Warning {
            warnings += 1;
        }
        eprint!("{}", render(d, &source, path));
    }
    if type_failed || sem_failed {
        if sem_failed {
            eprintln!("check: 失败（{} 个语义错误 + {} 个类型错误）",
                resolved.diagnostics.error_count(), typeck.diagnostics.error_count());
        } else {
            eprintln!("check: 失败（{} 个类型错误）", typeck.diagnostics.error_count());
        }
        return ExitCode::FAILURE;
    }
    // valueal: ownership + summary analysis (M2)
    let vf = ValueFlow::new(&resolved.program, &resolved.resolution, &typeck, &lexed.tokens).analyze();
    let mut va_failed = false;
    for d in &vf.diagnostics.diagnostics {
        if d.severity == Severity::Error {
            va_failed = true;
        }
        if d.severity == Severity::Warning {
            warnings += 1;
        }
        eprint!("{}", render(d, &source, path));
    }
    if va_failed {
        eprintln!("check: 失败（{} 个所有权错误）", vf.diagnostics.error_count());
        ExitCode::FAILURE
    } else {
        println!(
            "check: OK（{} 个 token，{} 个顶层项目，{} 个符号，{} 个类型，{} 个摘要，{} 个警告）",
            lexed.tokens.len(),
            program.items.len(),
            resolved.program.symbols.len(),
            typeck.types.len(),
            vf.summaries.len(),
            warnings
        );
        ExitCode::SUCCESS
    }
}
