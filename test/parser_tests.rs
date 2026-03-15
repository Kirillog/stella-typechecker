use crate::ast::*;
use crate::stella;

fn parse_expr(src: &str) -> Expr {
    stella::ExprParser::new().parse(src).expect("expression parse failed")
}

fn parse_type(src: &str) -> Type {
    stella::TypeParser::new().parse(src).expect("type parse failed")
}

fn parse_program(src: &str) -> Program {
    stella::ProgramParser::new().parse(src).expect("program parse failed")
}

fn parse_expr_err(src: &str) {
    assert!(stella::ExprParser::new().parse(src).is_err(), "expected parse error for: {src}");
}

// -----------------------------------------------------------------------
// Program-level tests
// -----------------------------------------------------------------------

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
    assert_eq!(prog.extensions[0].names[0], "#structural-patterns");
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

// -----------------------------------------------------------------------
// Literal expressions
// -----------------------------------------------------------------------

#[test]
fn test_const_true() {
    assert!(matches!(parse_expr("true"), Expr::ConstTrue));
}

#[test]
fn test_const_false() {
    assert!(matches!(parse_expr("false"), Expr::ConstFalse));
}

#[test]
fn test_const_unit() {
    assert!(matches!(parse_expr("unit"), Expr::ConstUnit));
}

#[test]
fn test_const_int() {
    assert!(matches!(parse_expr("42"), Expr::ConstInt(42)));
}

#[test]
fn test_var() {
    assert!(matches!(parse_expr("x"), Expr::Var(ref s) if s == "x"));
}

// -----------------------------------------------------------------------
// Arithmetic expressions
// -----------------------------------------------------------------------

#[test]
fn test_addition() {
    assert!(matches!(parse_expr("1 + 2"), Expr::Add(_, _)));
}

#[test]
fn test_subtraction() {
    assert!(matches!(parse_expr("5 - 3"), Expr::Subtract(_, _)));
}

#[test]
fn test_multiplication() {
    assert!(matches!(parse_expr("4 * 7"), Expr::Multiply(_, _)));
}

#[test]
fn test_division() {
    assert!(matches!(parse_expr("8 / 2"), Expr::Divide(_, _)));
}

#[test]
fn test_add_left_assoc() {
    // 1 + 2 + 3  should parse as  (1 + 2) + 3
    match parse_expr("1 + 2 + 3") {
        Expr::Add(lhs, _) => assert!(matches!(*lhs, Expr::Add(_, _))),
        _ => panic!("expected Add"),
    }
}

#[test]
fn test_mul_higher_prec_than_add() {
    // 1 + 2 * 3  should parse as  1 + (2 * 3)
    match parse_expr("1 + 2 * 3") {
        Expr::Add(_, rhs) => assert!(matches!(*rhs, Expr::Multiply(_, _))),
        _ => panic!("expected Add at top level"),
    }
}

// -----------------------------------------------------------------------
// Comparison expressions
// -----------------------------------------------------------------------

#[test]
fn test_less_than() {
    assert!(matches!(parse_expr("a < b"), Expr::LessThan(_, _)));
}

#[test]
fn test_equal() {
    assert!(matches!(parse_expr("a == b"), Expr::Equal(_, _)));
}

#[test]
fn test_not_equal() {
    assert!(matches!(parse_expr("a != b"), Expr::NotEqual(_, _)));
}

// -----------------------------------------------------------------------
// Boolean / logic expressions
// -----------------------------------------------------------------------

#[test]
fn test_logic_or() {
    assert!(matches!(parse_expr("a or b"), Expr::LogicOr(_, _)));
}

#[test]
fn test_logic_and() {
    assert!(matches!(parse_expr("a and b"), Expr::LogicAnd(_, _)));
}

#[test]
fn test_logic_not() {
    assert!(matches!(parse_expr("not(x)"), Expr::LogicNot(_)));
}

// -----------------------------------------------------------------------
// If / then / else
// -----------------------------------------------------------------------

#[test]
fn test_if_expr() {
    match parse_expr("if true then 1 else 0") {
        Expr::If { cond, then_, else_ } => {
            assert!(matches!(*cond,   Expr::ConstTrue));
            assert!(matches!(*then_,  Expr::ConstInt(1)));
            assert!(matches!(*else_,  Expr::ConstInt(0)));
        }
        _ => panic!("expected If"),
    }
}

// -----------------------------------------------------------------------
// Lambda abstraction
// -----------------------------------------------------------------------

#[test]
fn test_abstraction() {
    match parse_expr("fn(x : Nat) { return x }") {
        Expr::Abstraction { params, body } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert!(matches!(*body, Expr::Var(ref s) if s == "x"));
        }
        _ => panic!("expected Abstraction"),
    }
}

#[test]
fn test_abstraction_multi_param() {
    match parse_expr("fn(x : Nat, y : Nat) { return x }") {
        Expr::Abstraction { params, .. } => assert_eq!(params.len(), 2),
        _ => panic!("expected Abstraction"),
    }
}

// -----------------------------------------------------------------------
// Function application
// -----------------------------------------------------------------------

#[test]
fn test_application() {
    match parse_expr("f(x)") {
        Expr::Application { func, args } => {
            assert!(matches!(*func, Expr::Var(ref s) if s == "f"));
            assert_eq!(args.len(), 1);
        }
        _ => panic!("expected Application"),
    }
}

#[test]
fn test_application_multi_args() {
    match parse_expr("f(x, y, z)") {
        Expr::Application { args, .. } => assert_eq!(args.len(), 3),
        _ => panic!("expected Application"),
    }
}

// -----------------------------------------------------------------------
// Let expressions
// -----------------------------------------------------------------------

#[test]
fn test_let_expr() {
    match parse_expr("let x = 1 in x") {
        Expr::Let(bindings, body) => {
            assert_eq!(bindings.len(), 1);
            assert!(matches!(*body, Expr::Var(_)));
        }
        _ => panic!("expected Let"),
    }
}

// -----------------------------------------------------------------------
// Tuples and records
// -----------------------------------------------------------------------

#[test]
fn test_tuple() {
    match parse_expr("{1, 2, 3}") {
        Expr::Tuple(elems) => assert_eq!(elems.len(), 3),
        _ => panic!("expected Tuple"),
    }
}

#[test]
fn test_record() {
    match parse_expr("{x = 1, y = 2}") {
        Expr::Record(fields) => {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
        }
        _ => panic!("expected Record"),
    }
}

#[test]
fn test_dot_record() {
    match parse_expr("r.field") {
        Expr::DotRecord(_, field) => assert_eq!(field, "field"),
        _ => panic!("expected DotRecord"),
    }
}

#[test]
fn test_dot_tuple() {
    match parse_expr("t.0") {
        Expr::DotTuple(_, idx) => assert_eq!(idx, 0),
        _ => panic!("expected DotTuple"),
    }
}

// -----------------------------------------------------------------------
// Lists
// -----------------------------------------------------------------------

#[test]
fn test_empty_list() {
    assert!(matches!(parse_expr("[]"), Expr::List(ref v) if v.is_empty()));
}

#[test]
fn test_list_literals() {
    match parse_expr("[1, 2, 3]") {
        Expr::List(elems) => assert_eq!(elems.len(), 3),
        _ => panic!("expected List"),
    }
}

#[test]
fn test_cons() {
    assert!(matches!(parse_expr("cons(1, [])"), Expr::ConsList(_, _)));
}

#[test]
fn test_list_head() {
    assert!(matches!(parse_expr("List::head(xs)"), Expr::Head(_)));
}

#[test]
fn test_list_tail() {
    assert!(matches!(parse_expr("List::tail(xs)"), Expr::Tail(_)));
}

#[test]
fn test_list_isempty() {
    assert!(matches!(parse_expr("List::isempty(xs)"), Expr::IsEmpty(_)));
}

// -----------------------------------------------------------------------
// Nat built-ins
// -----------------------------------------------------------------------

#[test]
fn test_succ() {
    assert!(matches!(parse_expr("succ(0)"), Expr::Succ(_)));
}

#[test]
fn test_nat_pred() {
    assert!(matches!(parse_expr("Nat::pred(n)"), Expr::Pred(_)));
}

#[test]
fn test_nat_iszero() {
    assert!(matches!(parse_expr("Nat::iszero(n)"), Expr::IsZero(_)));
}

#[test]
fn test_nat_rec() {
    assert!(matches!(parse_expr("Nat::rec(n, 0, f)"), Expr::NatRec(_, _, _)));
}

// -----------------------------------------------------------------------
// Sum types: inl / inr
// -----------------------------------------------------------------------

#[test]
fn test_inl() {
    assert!(matches!(parse_expr("inl(x)"), Expr::Inl(_)));
}

#[test]
fn test_inr() {
    assert!(matches!(parse_expr("inr(x)"), Expr::Inr(_)));
}

// -----------------------------------------------------------------------
// Variant
// -----------------------------------------------------------------------

#[test]
fn test_variant_no_payload() {
    match parse_expr("<| none |>") {
        Expr::Variant { label, payload } => {
            assert_eq!(label, "none");
            assert!(payload.is_none());
        }
        _ => panic!("expected Variant"),
    }
}

#[test]
fn test_variant_with_payload() {
    match parse_expr("<| some = 42 |>") {
        Expr::Variant { label, payload } => {
            assert_eq!(label, "some");
            assert!(payload.is_some());
        }
        _ => panic!("expected Variant"),
    }
}

// -----------------------------------------------------------------------
// Match
// -----------------------------------------------------------------------

#[test]
fn test_match_expr() {
    match parse_expr("match x { true => 1 | false => 0 }") {
        Expr::Match { cases, .. } => assert_eq!(cases.len(), 2),
        _ => panic!("expected Match"),
    }
}

// -----------------------------------------------------------------------
// Sequence
// -----------------------------------------------------------------------

#[test]
fn test_sequence() {
    assert!(matches!(parse_expr("1; 2"), Expr::Sequence(_, _)));
}

// -----------------------------------------------------------------------
// Type annotations and casts
// -----------------------------------------------------------------------

#[test]
fn test_type_asc() {
    assert!(matches!(parse_expr("x as Nat"), Expr::TypeAsc(_, _)));
}

// -----------------------------------------------------------------------
// Fix / fold / unfold
// -----------------------------------------------------------------------

#[test]
fn test_fix() {
    assert!(matches!(parse_expr("fix(f)"), Expr::Fix(_)));
}

#[test]
fn test_fold() {
    assert!(matches!(parse_expr("fold[µ X . Nat] x"), Expr::Fold { .. }));
}

#[test]
fn test_unfold() {
    assert!(matches!(parse_expr("unfold[µ X . Nat] x"), Expr::Unfold { .. }));
}

// -----------------------------------------------------------------------
// Panic / ref / deref
// -----------------------------------------------------------------------

#[test]
fn test_panic() {
    assert!(matches!(parse_expr("panic!"), Expr::Panic));
}

#[test]
fn test_ref() {
    assert!(matches!(parse_expr("new(x)"), Expr::Ref(_)));
}

#[test]
fn test_deref() {
    // *e  is syntactic sugar for dereference
    assert!(matches!(parse_expr("*x"), Expr::Deref(_)));
}

// -----------------------------------------------------------------------
// Type parsing
// -----------------------------------------------------------------------

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
    assert!(matches!(parse_type("fn(Nat) -> Nat"), Type::Fun(params, return_type) if params == vec![Type::Nat] && *return_type == Type::Nat));
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

// -----------------------------------------------------------------------
// Error cases
// -----------------------------------------------------------------------

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
