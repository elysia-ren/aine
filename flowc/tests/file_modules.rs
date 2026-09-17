//! 集成测试：文件级模块 `mod name;`（B5-M26）

use std::path::PathBuf;

use aine::interp::Interp;
use aine::lexer::Lexer;
use aine::parser::Parser;

fn fixture(p: &str) -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "modtest", p].iter().collect()
}

#[test]
fn file_module_inline_and_run() {
    let path = fixture("main.aine");
    let src = std::fs::read_to_string(&path).unwrap();
    let lexed = Lexer::new(&src).lex();
    assert!(!lexed.diagnostics.has_errors());
    let parsed = Parser::new(&lexed.tokens, "main.aine").parse_program();
    assert!(!parsed.diagnostics.has_errors(), "{:?}", parsed.diagnostics.diagnostics);
    let mut program = parsed.program.unwrap();
    let dir = path.parent().unwrap().to_path_buf();
    aine::resolve_file_modules(&mut program, &dir).expect("module resolution");

    // strutil 模块被内联为 Mod 项且带内容
    let m = program.items.iter().find_map(|it| match it {
        aine::ast::Item::Mod(m) if m.name == "strutil" => Some(m),
        _ => None,
    });
    let m = m.expect("strutil module present");
    assert!(!m.items.is_empty(), "file module should be inlined");

    // 解释器扁平注册后可运行 main
    let mut interp = Interp::new();
    interp.load(&program);
    let _ = interp.run_main().expect("run main");
    assert!(interp.output.contains("aine!"), "output: {}", interp.output);
    assert!(interp.output.contains("ababab"), "output: {}", interp.output);
}

#[test]
fn missing_module_file_errors() {
    let src = "module nosuch;\nfn main() {}\n";
    let parsed = Parser::new(&Lexer::new(src).lex().tokens, "t.aine").parse_program();
    let mut program = parsed.program.unwrap();
    let err = aine::resolve_file_modules(&mut program, &std::env::temp_dir()).unwrap_err();
    assert!(err.contains("nosuch"), "err: {err}");
}

#[test]
fn module_cycle_detected() {
    let dir = std::env::temp_dir().join("aine_modcycle_test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.aine"), "module b;\nfn main() {}\n").unwrap();
    std::fs::write(dir.join("b.aine"), "module a;\n").unwrap();
    let parsed = Parser::new(&Lexer::new("module a;\nfn main() {}\n").lex().tokens, "root.aine").parse_program();
    let mut program = parsed.program.unwrap();
    let err = aine::resolve_file_modules(&mut program, &dir).unwrap_err();
    assert!(err.contains("环路") || err.contains("重复加载"), "err: {err}");
    let _ = std::fs::remove_dir_all(&dir);
}
