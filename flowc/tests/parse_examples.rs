//! Integration tests: parse the real example files end-to-end.

use std::path::PathBuf;

use aine::lexer::Lexer;
use aine::parser::Parser;

fn load(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "examples", name].iter().collect();
    std::fs::read_to_string(&path).expect(&format!("missing example {name}"))
}

fn check_example(name: &str) {
    let src = load(name);
    let lexed = Lexer::new(&src).lex();
    assert!(!lexed.diagnostics.has_errors(), "lex errors in {name}: {:?}", lexed.diagnostics.diagnostics);
    let parsed = Parser::new(&lexed.tokens, name).parse_program();
    assert!(!parsed.diagnostics.has_errors(), "parse errors in {name}: {:?}", parsed.diagnostics.diagnostics);
    let program = parsed.program.expect("program");
    println!("{name}: {} items, {} tokens", program.items.len(), lexed.tokens.len());
}

#[test]
fn parses_hello_example() {
    check_example("hello.aine");
}

#[test]
fn parses_account_book_example() {
    // design doc §69 canonical example — the joint UI+Task acceptance scenario
    check_example("account_book.aine");
}

#[test]
fn example_acceptance_counts() {
    // §72 joint acceptance: the canonical example must have exactly the
    // expected top-level structure (2 imports + 1 struct + 1 ui + 1 main)
    let src = load("account_book.aine");
    let lexed = Lexer::new(&src).lex();
    let parsed = Parser::new(&lexed.tokens, "account_book.aine").parse_program();
    assert!(!parsed.diagnostics.has_errors());
    let program = parsed.program.unwrap();
    assert_eq!(program.items.len(), 5);
    let mut imports = 0;
    let mut structs = 0;
    let mut uis = 0;
    let mut fns = 0;
    for it in &program.items {
        match it {
            aine::ast::Item::Import(_) => imports += 1,
            aine::ast::Item::Struct(_) => structs += 1,
            aine::ast::Item::Ui(_) => uis += 1,
            aine::ast::Item::Fn(_) => fns += 1,
            _ => {}
        }
    }
    assert_eq!(imports, 2);
    assert_eq!(structs, 1);
    assert_eq!(uis, 1);
    assert_eq!(fns, 1);
}
