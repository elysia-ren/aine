//! Parser tests (split from parser.rs).

#[cfg(test)]
mod tests {
    use crate::parser::*;
    use crate::ast::*;
    use crate::lexer::Lexer;
    fn parse_ok(src: &str) -> Program {
        let lexed = Lexer::new(src).lex();
        assert!(!lexed.diagnostics.has_errors(), "lex errors: {:?}", lexed.diagnostics.diagnostics);
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        assert!(!parsed.diagnostics.has_errors(), "parse errors: {:?}", parsed.diagnostics.diagnostics);
        parsed.program.expect("program")
    }

    fn parse_err_count(src: &str) -> usize {
        let lexed = Lexer::new(src).lex();
        let parsed = Parser::new(&lexed.tokens, "test.aine").parse_program();
        parsed.diagnostics.error_count()
    }

    #[test]
    fn parses_fn_with_params_and_return() {
        let p = parse_ok("fn add(a: i32, b: i32) -> i32 { a + b }");
        assert_eq!(p.items.len(), 1);
        let Item::Fn(f) = &p.items[0] else { panic!("expected fn") };
        assert_eq!(f.sig.name, "add");
        assert_eq!(f.sig.params.len(), 2);
        assert!(f.sig.ret.is_some());
        assert!(f.body.tail.is_some());
    }

    #[test]
    fn parses_move_and_ref_params() {
        let p = parse_ok("fn store(move user: User, s: &str) { }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert!(f.sig.params[0].is_move);
        assert!(matches!(f.sig.params[1].ty, Some(Type::Ref { mutable: false, .. })));
    }

    #[test]
    fn parses_let_with_type_and_init() {
        let p = parse_ok("fn f() { var count: i32 = 0 }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        let Stmt::Let(ls) = &f.body.stmts[0] else { panic!("expected let") };
        assert!(ls.is_mut);
        assert!(ls.ty.is_some());
        assert!(ls.init.is_some());
    }

    #[test]
    fn parses_if_else_chain() {
        let p = parse_ok("fn f() { if a > 1 { x() } else if b { y() } else { z() } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 1);
        assert!(matches!(f.body.stmts[0], Stmt::If(_)));
    }

    #[test]
    fn parses_for_and_while() {
        let p = parse_ok("fn f() { for i in 0..10 { print(i) } while x < 5 { x += 1 } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 2);
        assert!(matches!(f.body.stmts[0], Stmt::For { .. }));
        assert!(matches!(f.body.stmts[1], Stmt::While { .. }));
    }

    #[test]
    fn parses_struct_and_enum() {
        let p = parse_ok("struct User { name: String age: i32 }\nenum Result<T, E> { Ok(T) Err(E) }");
        assert_eq!(p.items.len(), 2);
        let Item::Struct(s) = &p.items[0] else { panic!() };
        assert_eq!(s.fields.len(), 2);
        let Item::Enum(e) = &p.items[1] else { panic!() };
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].payload.len(), 1);
    }

    #[test]
    fn parses_ui_def_with_state_and_render() {
        let p = parse_ok(
            "ui Counter {\n    title = \"x\"\n    size = (400, 300)\n    @state\n    var count = 0\n    render {\n        Column {\n            Text(f\"{count}\")\n        }\n    }\n}",
        );
        let Item::Ui(u) = &p.items[0] else { panic!("expected ui") };
        assert_eq!(u.props.len(), 2);
        assert_eq!(u.states.len(), 1);
        assert!(u.states[0].is_state);
        assert!(u.render.is_some());
    }

    #[test]
    fn parses_go_and_ui_ops() {
        let p = parse_ok("fn f() { let t = go { work() }\ngo! { fire() }\nui { total = x } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 2);
        assert!(f.body.tail.is_some(), "ui op becomes tail");
    }

    #[test]
    fn parses_closures_and_trailing_blocks() {
        let p = parse_ok("fn f() { List(records) r =>  { item(r) }\nButton(\"x\") { click() }\nColumn { Row { } } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 2);
        assert!(f.body.tail.is_some());
    }

    #[test]
    fn parses_struct_literal() {
        let p = parse_ok("fn f() { let r = Record { id: 0 desc: \"x\" } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        let Stmt::Let(ls) = &f.body.stmts[0] else { panic!() };
        let init = ls.init.as_ref().unwrap();
        let Expr::StructLit { name, fields } = &**init else { panic!("expected struct literal, got {:?}", init) };
        assert_eq!(name.display(), "Record");
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn parses_method_chains() {
        let p = parse_ok("fn f() { let t = records.iter().map(r =>  r.amount).sum() }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        let Stmt::Let(ls) = &f.body.stmts[0] else { panic!() };
        assert!(ls.init.is_some());
    }

    #[test]
    fn parses_named_arguments() {
        let p = parse_ok("fn f() { TextInput(placeholder = \"描述\", type = number) }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        let tail = f.body.tail.as_ref().expect("tail expression");
        let Expr::Call { args, .. } = &**tail else { panic!("expected call") };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].name.as_deref(), Some("placeholder"));
        assert_eq!(args[1].name.as_deref(), Some("type"));
    }

    #[test]
    fn parses_assignments_and_compound() {
        let p = parse_ok("fn f() { count += 1\ncount = 0\ntotal = records.iter().sum() }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 2);
        assert!(f.body.tail.is_some(), "last expression becomes tail");
    }

    #[test]
    fn parses_try_and_question() {
        let p = parse_ok("fn f() -> Result<(), E> { try { native()? } catch (e: FfiError) { } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert!(matches!(f.body.stmts[0], Stmt::TryCatch(_)));
    }

    #[test]
    fn parses_match() {
        let p = parse_ok("fn f() { match x { 1 => a\n_ => b } }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        let tail = f.body.tail.as_ref().expect("tail");
        let Expr::Match { arms, .. } = &**tail else { panic!("expected match") };
        assert_eq!(arms.len(), 2);
    }

    #[test]
    fn parses_multiline_expressions() {
        let p = parse_ok("fn f() { let t = db.query(\n\"SELECT *\"\n)?\nui {\n    records = t\n} }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert_eq!(f.body.stmts.len(), 1);
        assert!(f.body.tail.is_some(), "ui op becomes tail");
    }

    #[test]
    fn parses_imports_and_global() {
        let p = parse_ok("import sqlite\n@global\nvar settings: Config = default()");
        assert_eq!(p.items.len(), 2);
        assert!(matches!(p.items[0], Item::Import(_)));
        assert!(matches!(p.items[1], Item::GlobalState(_)));
    }

    #[test]
    fn parses_block_tail_value() {
        let p = parse_ok("fn add(a: i32, b: i32) -> i32 { a + b }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert!(f.body.tail.is_some(), "tail expression should be captured");
    }

    #[test]
    fn error_on_unexpected_token() {
        let n = parse_err_count("fn f() { = 1 }");
        assert!(n >= 1, "expected at least one parse error");
    }

    #[test]
    fn no_infinite_loop_on_garbage() {
        // must terminate (regression: parser used to hang on some inputs)
        let n = parse_err_count("ui X { @state let } @state } } }");
        let _ = n;
    }

    #[test]
    fn else_on_new_line_parses() {
        let p = parse_ok("fn f() { if a {\n x()\n} else {\n y()\n} }");
        let Item::Fn(f) = &p.items[0] else { panic!() };
        assert!(matches!(f.body.stmts[0], Stmt::If(_)));
    }
}
