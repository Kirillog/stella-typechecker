use stella_typechecker::ast::*;
use stella_typechecker::parser;

fn parse_expr(src: &str) -> Expr {
    parser::ExprParser::new()
        .parse(src)
        .expect("expression parse failed")
}

fn parse_type(src: &str) -> Type {
    parser::TypeParser::new()
        .parse(src)
        .expect("type parse failed")
}

fn parse_program(src: &str) -> Program {
    parser::ProgramParser::new()
        .parse(src)
        .expect("program parse failed")
}

fn parse_expr_err(src: &str) {
    assert!(
        parser::ExprParser::new().parse(src).is_err(),
        "expected parse error for: {src}"
    );
}

#[test]
fn test_minimal_program() {
    let prog = parse_program("language core; fn main(n : Nat) -> Nat { return n }");
    assert_eq!(prog.decls.len(), 1);
    match &prog.decls[0] {
        Decl::Fun(f) => assert_eq!(f.name, "main"),
        _ => panic!("expected DeclFun"),
    }
}

#[test]
fn test_program_with_extension() {
    let prog = parse_program(
        "language core; extend with #structural-patterns; fn id(x : Nat) -> Nat { return x }",
    );
    assert_eq!(prog.extensions.len(), 1);
    assert_eq!(
        prog.extensions[0].features[0],
        StellaExtension::StructuralPatterns
    );
}

#[test]
fn test_program_multiple_fns() {
    let prog = parse_program(
        "language core;
         fn f(x : Nat) -> Nat { return x }
         fn g(x : Nat) -> Nat { return x }",
    );
    assert_eq!(prog.decls.len(), 2);
}

#[test]
fn test_const_true() {
    assert!(matches!(parse_expr("true").node, ExprKind::ConstTrue));
}

#[test]
fn test_const_false() {
    assert!(matches!(parse_expr("false").node, ExprKind::ConstFalse));
}

#[test]
fn test_const_unit() {
    assert!(matches!(parse_expr("unit").node, ExprKind::ConstUnit));
}

#[test]
fn test_const_int() {
    assert!(matches!(parse_expr("42").node, ExprKind::ConstInt(42)));
}

#[test]
fn test_var() {
    assert!(matches!(parse_expr("x").node, ExprKind::Var(ref s) if s == "x"));
}

#[test]
fn test_addition() {
    assert!(matches!(parse_expr("1 + 2").node, ExprKind::Add(_, _)));
}

#[test]
fn test_subtraction() {
    assert!(matches!(parse_expr("5 - 3").node, ExprKind::Subtract(_, _)));
}

#[test]
fn test_multiplication() {
    assert!(matches!(parse_expr("4 * 7").node, ExprKind::Multiply(_, _)));
}

#[test]
fn test_division() {
    assert!(matches!(parse_expr("8 / 2").node, ExprKind::Divide(_, _)));
}

#[test]
fn test_add_left_assoc() {
    match parse_expr("1 + 2 + 3").node {
        ExprKind::Add(lhs, _) => assert!(matches!(lhs.node, ExprKind::Add(_, _))),
        _ => panic!("expected Add"),
    }
}

#[test]
fn test_mul_higher_prec_than_add() {
    match parse_expr("1 + 2 * 3").node {
        ExprKind::Add(_, rhs) => assert!(matches!(rhs.node, ExprKind::Multiply(_, _))),
        _ => panic!("expected Add at top level"),
    }
}

#[test]
fn test_less_than() {
    assert!(matches!(parse_expr("a < b").node, ExprKind::LessThan(_, _)));
}

#[test]
fn test_equal() {
    assert!(matches!(parse_expr("a == b").node, ExprKind::Equal(_, _)));
}

#[test]
fn test_not_equal() {
    assert!(matches!(
        parse_expr("a != b").node,
        ExprKind::NotEqual(_, _)
    ));
}

#[test]
fn test_logic_or() {
    assert!(matches!(parse_expr("a or b").node, ExprKind::LogicOr(_, _)));
}

#[test]
fn test_logic_and() {
    assert!(matches!(
        parse_expr("a and b").node,
        ExprKind::LogicAnd(_, _)
    ));
}

#[test]
fn test_logic_not() {
    assert!(matches!(parse_expr("not(x)").node, ExprKind::LogicNot(_)));
}

#[test]
fn test_if_expr() {
    match parse_expr("if true then 1 else 0").node {
        ExprKind::If { cond, then_, else_ } => {
            assert!(matches!(cond.node, ExprKind::ConstTrue));
            assert!(matches!(then_.node, ExprKind::ConstInt(1)));
            assert!(matches!(else_.node, ExprKind::ConstInt(0)));
        }
        _ => panic!("expected If"),
    }
}

#[test]
fn test_abstraction() {
    match parse_expr("fn(x : Nat) { return x }").node {
        ExprKind::Abstraction { params, body } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert!(matches!(body.node, ExprKind::Var(ref s) if s == "x"));
        }
        _ => panic!("expected Abstraction"),
    }
}

#[test]
fn test_abstraction_multi_param() {
    match parse_expr("fn(x : Nat, y : Nat) { return x }").node {
        ExprKind::Abstraction { params, .. } => assert_eq!(params.len(), 2),
        _ => panic!("expected Abstraction"),
    }
}

#[test]
fn test_application() {
    match parse_expr("f(x)").node {
        ExprKind::Application { func, args } => {
            assert!(matches!(func.node, ExprKind::Var(ref s) if s == "f"));
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected Application"),
    }
}

#[test]
fn test_application_multi_args() {
    match parse_expr("f(x, y, z)").node {
        ExprKind::Application { args, .. } => assert_eq!(args.len(), 3),
        _ => panic!("expected Application"),
    }
}

#[test]
fn test_let_expr() {
    match parse_expr("let x = 1 in x").node {
        ExprKind::Let(bindings, body) => {
            assert_eq!(bindings.len(), 1);
            assert!(matches!(body.node, ExprKind::Var(_)));
        }
        _ => panic!("expected Let"),
    }
}

#[test]
fn test_tuple() {
    match parse_expr("{1, 2, 3}").node {
        ExprKind::Tuple(elems) => assert_eq!(elems.len(), 3),
        _ => panic!("expected Tuple"),
    }
}

#[test]
fn test_record() {
    match parse_expr("{x = 1, y = 2}").node {
        ExprKind::Record(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
        }
        _ => panic!("expected Record"),
    }
}

#[test]
fn test_dot_record() {
    match parse_expr("r.field").node {
        ExprKind::DotRecord(_, field) => assert_eq!(field, "field"),
        _ => panic!("expected DotRecord"),
    }
}

#[test]
fn test_dot_tuple() {
    match parse_expr("t.0").node {
        ExprKind::DotTuple(_, idx) => assert_eq!(idx, 0),
        _ => panic!("expected DotTuple"),
    }
}

#[test]
fn test_empty_list() {
    assert!(matches!(parse_expr("[]").node, ExprKind::List(ref v) if v.is_empty()));
}

#[test]
fn test_list_literals() {
    match parse_expr("[1, 2, 3]").node {
        ExprKind::List(elems) => assert_eq!(elems.len(), 3),
        _ => panic!("expected List"),
    }
}

#[test]
fn test_cons() {
    assert!(matches!(
        parse_expr("cons(1, [])").node,
        ExprKind::ConsList(_, _)
    ));
}

#[test]
fn test_list_head() {
    assert!(matches!(
        parse_expr("List::head(xs)").node,
        ExprKind::Head(_)
    ));
}

#[test]
fn test_list_tail() {
    assert!(matches!(
        parse_expr("List::tail(xs)").node,
        ExprKind::Tail(_)
    ));
}

#[test]
fn test_list_isempty() {
    assert!(matches!(
        parse_expr("List::isempty(xs)").node,
        ExprKind::IsEmpty(_)
    ));
}

#[test]
fn test_succ() {
    assert!(matches!(parse_expr("succ(0)").node, ExprKind::Succ(_)));
}

#[test]
fn test_nat_pred() {
    assert!(matches!(parse_expr("Nat::pred(n)").node, ExprKind::Pred(_)));
}

#[test]
fn test_nat_iszero() {
    assert!(matches!(
        parse_expr("Nat::iszero(n)").node,
        ExprKind::IsZero(_)
    ));
}

#[test]
fn test_nat_rec() {
    assert!(matches!(
        parse_expr("Nat::rec(n, 0, f)").node,
        ExprKind::NatRec(_, _, _)
    ));
}

#[test]
fn test_inl() {
    assert!(matches!(parse_expr("inl(x)").node, ExprKind::Inl(_)));
}

#[test]
fn test_inr() {
    assert!(matches!(parse_expr("inr(x)").node, ExprKind::Inr(_)));
}

#[test]
fn test_variant_no_payload() {
    match parse_expr("<| none |>").node {
        ExprKind::Variant { label, payload } => {
            assert_eq!(label, "none");
            assert!(payload.is_none());
        }
        _ => panic!("expected Variant"),
    }
}

#[test]
fn test_variant_with_payload() {
    match parse_expr("<| some = 42 |>").node {
        ExprKind::Variant { label, payload } => {
            assert_eq!(label, "some");
            assert!(payload.is_some());
        }
        _ => panic!("expected Variant"),
    }
}

#[test]
fn test_match_expr() {
    match parse_expr("match x { true => 1 | false => 0 }").node {
        ExprKind::Match { cases, .. } => assert_eq!(cases.len(), 2),
        _ => panic!("expected Match"),
    }
}

#[test]
fn test_sequence() {
    assert!(matches!(parse_expr("1; 2").node, ExprKind::Sequence(_, _)));
}

#[test]
fn test_type_asc() {
    assert!(matches!(
        parse_expr("x as Nat").node,
        ExprKind::TypeAsc(_, _)
    ));
}

#[test]
fn test_fix() {
    assert!(matches!(parse_expr("fix(f)").node, ExprKind::Fix(_)));
}

#[test]
fn test_fold() {
    assert!(matches!(
        parse_expr("fold[µ X . Nat] x").node,
        ExprKind::Fold { .. }
    ));
}

#[test]
fn test_unfold() {
    assert!(matches!(
        parse_expr("unfold[µ X . Nat] x").node,
        ExprKind::Unfold { .. }
    ));
}

#[test]
fn test_panic() {
    assert!(matches!(parse_expr("panic!").node, ExprKind::Panic));
}

#[test]
fn test_ref() {
    assert!(matches!(parse_expr("new(x)").node, ExprKind::Ref(_)));
}

#[test]
fn test_deref() {
    assert!(matches!(parse_expr("*x").node, ExprKind::Deref(_)));
}

#[test]
fn test_type_nat() {
    assert!(matches!(parse_type("Nat"), Type::Nat));
}

#[test]
fn test_type_bool() {
    assert!(matches!(parse_type("Bool"), Type::Bool));
}

#[test]
fn test_type_unit() {
    assert!(matches!(parse_type("Unit"), Type::Unit));
}

#[test]
fn test_type_fun() {
    assert!(
        matches!(parse_type("fn(Nat) -> Nat"), Type::Fun(params, return_type) if params == vec![Type::Nat] && *return_type == Type::Nat)
    );
}

#[test]
fn test_type_fun_multi_param() {
    match parse_type("fn(Nat, Bool) -> Unit") {
        Type::Fun(params, _) => assert_eq!(params.len(), 2),
        _ => panic!("expected Fun type"),
    }
}

#[test]
fn test_type_sum() {
    assert!(matches!(parse_type("Nat + Bool"), Type::Sum(_, _)));
}

#[test]
fn test_type_tuple() {
    match parse_type("{Nat, Bool}") {
        Type::Tuple(ts) => assert_eq!(ts.len(), 2),
        _ => panic!("expected Tuple type"),
    }
}

#[test]
fn test_type_record() {
    match parse_type("{x : Nat, y : Bool}") {
        Type::Record(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
        }
        _ => panic!("expected Record type"),
    }
}

#[test]
fn test_type_list() {
    assert!(matches!(parse_type("[Nat]"), Type::List(_)));
}

#[test]
fn test_type_ref() {
    assert!(matches!(parse_type("&Nat"), Type::Ref(_)));
}

#[test]
fn test_type_forall() {
    assert!(matches!(parse_type("forall T . T"), Type::ForAll(_, _)));
}

#[test]
fn test_type_rec() {
    assert!(matches!(parse_type("µ X . X"), Type::Rec(_, _)));
}

#[test]
fn test_type_var() {
    assert!(matches!(parse_type("T"), Type::Var(ref s) if s == "T"));
}

#[test]
fn test_error_empty_input() {
    parse_expr_err("");
}

#[test]
fn test_error_unbalanced_paren() {
    parse_expr_err("(1 + 2");
}

#[test]
fn test_error_missing_operand() {
    parse_expr_err("1 +");
}

#[test]
fn test_error_consecutive_operators() {
    parse_expr_err("1 + + 2");
}
