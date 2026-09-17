//! Debugger 基础（M7 §5 DoD 6）：`aine debug <file> [行号...]`
//! 断点执行：设置行号断点 → 解释器运行 → 输出每个断点命中的行号与局部变量快照。
//! （交互式暂停/单步/异步栈为后续扩展；断点+状态检查为当前交付）

use std::process::ExitCode;

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::{resolve::Resolver, resolve_file_modules};

pub fn run_debug(path: &str, break_lines: &[usize]) -> ExitCode {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    let lexed = Lexer::new(&source).lex();
    if lexed.diagnostics.has_errors() {
        for d in &lexed.diagnostics.diagnostics {
            eprint!("{}", crate::diagnostics::render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }
    let mut program = match Parser::new(&lexed.tokens, path).parse_program() {
        p if p.diagnostics.has_errors() => {
            for d in &p.diagnostics.diagnostics {
                eprint!("{}", crate::diagnostics::render(d, &source, path));
            }
            return ExitCode::FAILURE;
        }
        p => p.program.unwrap(),
    };
    let mod_dir = std::path::Path::new(path).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    if let Err(e) = resolve_file_modules(&mut program, &mod_dir) {
        eprintln!("error: {}", e);
        return ExitCode::FAILURE;
    }
    let resolver = Resolver::new(&lexed.tokens);
    let resolved = resolver.resolve(&program);
    if resolved.diagnostics.has_errors() {
        for d in &resolved.diagnostics.diagnostics {
            eprint!("{}", crate::diagnostics::render(d, &source, path));
        }
        return ExitCode::FAILURE;
    }

    let mut interp = crate::interp::Interp::new();
    interp.set_locs(&lexed.tokens);
    for l in break_lines {
        interp.breakpoints.insert(*l);
    }
    interp.load(&program);
    let result = interp.run_main();

    println!("== Aine Debugger 报告 ==");
    println!("断点: {:?}", break_lines);
    if interp.bp_hits.is_empty() {
        println!("无断点命中");
    } else {
        for (line, vars) in &interp.bp_hits {
            println!("断点命中 行 {}: {}", line, vars);
        }
    }
    println!("-- 程序输出 --");
    print!("{}", interp.output);
    match result {
        Ok(_) => {
            println!("-- 运行完成 --");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("-- 运行错误: {:?} --", e);
            ExitCode::FAILURE
        }
    }
}
