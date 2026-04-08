use stella_typechecker::parser;
use stella_typechecker::type_error::{TypeCheckError, TypeError};
use stella_typechecker::typechecker::TypeChecker;

fn typecheck(src: &str) -> Vec<TypeCheckError> {
    let prog = parser::ProgramParser::new()
        .parse(src)
        .expect("parse failed");
    TypeChecker::new().check_program(&prog, src)
}

fn has_error<F: Fn(&TypeError) -> bool>(errors: &[TypeCheckError], pred: F) -> bool {
    errors.iter().any(|e| pred(&e.error))
}

fn missing_witnesses(errors: &[TypeCheckError]) -> Option<&[String]> {
    errors.iter().find_map(|e| match &e.error {
        TypeError::NonexhaustiveMatchPatterns { missing, .. } => Some(missing.as_slice()),
        _ => None,
    })
}

#[test]
fn test_error_missing_main() {
    let errors = typecheck("language core; fn foo(n : Nat) -> Nat { return n }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::MissingMain)),
        "expected MissingMain, got: {errors:?}"
    );
}

#[test]
fn test_error_undefined_variable() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return x }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UndefinedVariable { .. }
        )),
        "expected UndefinedVariable, got: {errors:?}"
    );
}

#[test]
fn test_error_display_shows_source_excerpt() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return x }");
    let rendered = errors
        .iter()
        .find(|e| matches!(e.error, TypeError::UndefinedVariable { .. }))
        .expect("expected UndefinedVariable")
        .to_string();

    assert!(
        rendered
            .contains("\n  --> [1:49]\n  1 | language core; fn main(n : Nat) -> Nat { return x }"),
        "expected source line context header in rendered error, got: {rendered}"
    );
}

#[test]
fn test_error_unexpected_type_for_expression() {
    let errors = typecheck("language core; fn main(n : Nat) -> Bool { return n }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_function() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return n(1) }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAFunction { .. })),
        "expected NotAFunction, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_tuple() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return n.0 }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotATuple { .. })),
        "expected NotATuple, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_record() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return n.foo }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotARecord { .. })),
        "expected NotARecord, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_list() {
    let errors = typecheck("language core; fn main(n : Nat) -> Bool { return List::isempty(n) }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAList { .. })),
        "expected NotAList, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_lambda() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> Nat { return fn(x : Nat) { return x } }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedLambda { .. })),
        "expected UnexpectedLambda, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_type_for_parameter() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> fn(Bool) -> Nat { return fn(x : Nat) { return 1 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForParameter { .. }
        )),
        "expected UnexpectedTypeForParameter, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_tuple() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return {1, 2} }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedTuple { .. })),
        "expected UnexpectedTuple, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_record() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return {x = 1} }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedRecord { .. })),
        "expected UnexpectedRecord, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_variant() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return <| foo = 1 |> }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedVariant { .. }
        )),
        "expected UnexpectedVariant, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_list() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return [1, 2] }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedList { .. })),
        "expected UnexpectedList, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_injection() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return inl(1) }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedInjection { .. }
        )),
        "expected UnexpectedInjection, got: {errors:?}"
    );
}

#[test]
fn test_error_missing_record_fields() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> {x : Nat, y : Nat} { return {x = 1} }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::MissingRecordFields { .. }
        )),
        "expected MissingRecordFields, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_record_fields() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> {x : Nat} { return {x = 1, y = 2} }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedRecordFields { .. }
        )),
        "expected UnexpectedRecordFields, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_field_access() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return {x = 1}.y }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedFieldAccess { .. }
        )),
        "expected UnexpectedFieldAccess, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_variant_label() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> <| foo : Nat |> { return <| bar = 1 |> }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedVariantLabel { .. }
        )),
        "expected UnexpectedVariantLabel, got: {errors:?}"
    );
}

#[test]
fn test_error_tuple_index_out_of_bounds() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return {1, 2}.3 }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::TupleIndexOutOfBounds { .. }
        )),
        "expected TupleIndexOutOfBounds, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_tuple_length() {
    let errors = typecheck("language core; fn main(n : Nat) -> {Nat, Nat} { return {1, 2, 3} }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTupleLength { .. }
        )),
        "expected UnexpectedTupleLength, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_sum_type() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return let x = inl(0) in n }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousSumType { .. })),
        "expected AmbiguousSumType, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_variant_type() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> Nat { return let x = <| foo = 1 |> in n }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousVariantType { .. }
        )),
        "expected AmbiguousVariantType, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_list() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return let x = [] in n }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousList { .. })),
        "expected AmbiguousList, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_tuple() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return (inl(0)).0 }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousTuple { .. })),
        "expected AmbiguousTuple, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_function() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return (inl(0))(1) }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousFunction { .. }
        )),
        "expected AmbiguousFunction, got: {errors:?}"
    );
}

#[test]
fn test_error_illegal_empty_matching() {
    let errors = typecheck("language core; fn main(n : Bool) -> Nat { return match n {} }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IllegalEmptyMatching { .. }
        )),
        "expected IllegalEmptyMatching, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_patterns() {
    let errors =
        typecheck("language core; fn main(n : Bool) -> Nat { return match n { true => 1 } }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_pattern_for_type() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> Nat { return match n { true => 1 | false => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_record_fields() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> Nat { return let x = {a = 1, a = 2} in 0 }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateRecordFields { .. }
        )),
        "expected DuplicateRecordFields, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_record_type_fields() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> {x : Nat, x : Nat} { return {x = 1} }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateRecordTypeFields { .. }
        )),
        "expected DuplicateRecordTypeFields, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_variant_type_fields() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> <| foo : Nat, foo : Nat |> { return <| foo = 1 |> }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateVariantTypeFields { .. }
        )),
        "expected DuplicateVariantTypeFields, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_function_declaration() {
    let errors = typecheck(
        "language core;
         fn foo(n : Nat) -> Nat { return n }
         fn foo(n : Nat) -> Nat { return n }
         fn main(n : Nat) -> Nat { return n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateFunctionDeclaration { .. }
        )),
        "expected DuplicateFunctionDeclaration, got: {errors:?}"
    );
}

#[test]
fn test_error_incorrect_arity_of_main() {
    let errors = typecheck("language core; fn main(a : Nat, b : Nat) -> Nat { return a }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IncorrectArityOfMain { .. }
        )),
        "expected IncorrectArityOfMain, got: {errors:?}"
    );
}

#[test]
fn test_error_incorrect_number_of_arguments() {
    let errors = typecheck(
        "language core;
         fn add(a : Nat, b : Nat) -> Nat { return a }
         fn main(n : Nat) -> Nat { return add(1, 2, 3) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IncorrectNumberOfArguments { .. }
        )),
        "expected IncorrectNumberOfArguments, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_number_of_parameters_in_lambda() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> fn(Nat) -> Nat \
         { return fn(x : Nat, y : Nat) { return 1 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedNumberOfParametersInLambda { .. }
        )),
        "expected UnexpectedNumberOfParametersInLambda, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_record_pattern_fields() {
    let errors = typecheck(
        "language core; fn main(n : {x : Nat}) -> Nat { return match n { {x = a, x = b} => a } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateRecordPatternFields { .. }
        )),
        "expected DuplicateRecordPatternFields, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_data_for_nullary_label() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> <| none |> { return <| none = 1 |> }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedDataForNullaryLabel { .. }
        )),
        "expected UnexpectedDataForNullaryLabel, got: {errors:?}"
    );
}

#[test]
fn test_error_missing_data_for_label() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> <| some : Nat |> { return <| some |> }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::MissingDataForLabel { .. }
        )),
        "expected MissingDataForLabel, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_non_nullary_variant_pattern() {
    let errors = typecheck(
        "language core; fn main(n : <| none |>) -> Nat { return match n { <| none = x |> => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedNonNullaryVariantPattern { .. }
        )),
        "expected UnexpectedNonNullaryVariantPattern, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_nullary_variant_pattern() {
    let errors = typecheck(
        "language core; fn main(n : <| some : Nat |>) -> Nat { return match n { <| some |> => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedNullaryVariantPattern { .. }
        )),
        "expected UnexpectedNullaryVariantPattern, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_function_parameter() {
    let errors = typecheck(
        "language core;
         fn foo(n : Nat, n : Bool) -> Nat { return 0 }
         fn main(n : Nat) -> Nat { return 0 }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateFunctionParameter { .. }
        )),
        "expected DuplicateFunctionParameter, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_let_binding() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> Nat { return let x = 1, x = 2 in x }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateLetBinding { .. }
        )),
        "expected DuplicateLetBinding, got: {errors:?}"
    );
}

#[test]
fn test_error_letrec_inl_inner_pattern_with_sum_ascription() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> Nat { return letrec inl(x) as (Nat + Bool) = inl(0) as (Nat + Bool) in x }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveLetRecPatterns { .. }
        )),
        "expected NonexhaustiveLetRecPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_duplicate_type_parameter() {
    let errors = typecheck(
        "language core;
         generic fn foo[T, T](x : T) -> T { return x }
         fn main(n : Nat) -> Nat { return 0 }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateTypeParameter { .. }
        )),
        "expected DuplicateTypeParameter, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_sum_missing_inl() {
    let errors = typecheck(
        "language core; fn main(n : Nat + Bool) -> Nat { return match n { inr(b) => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_sum_missing_inr() {
    let errors = typecheck(
        "language core; fn main(n : Nat + Bool) -> Nat { return match n { inl(x) => x } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_variant() {
    let errors = typecheck(
        "language core; fn main(n : <| a : Nat, b : Nat |>) -> Nat \
         { return match n { <| a = x |> => x } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_nat_missing_zero() {
    let errors =
        typecheck("language core; fn main(n : Nat) -> Nat { return match n { succ(x) => x } }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_nat_missing_succ() {
    let errors = typecheck("language core; fn main(n : Nat) -> Nat { return match n { 0 => 0 } }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_list_missing_nil() {
    let errors = typecheck(
        "language core; fn main(n : [Nat]) -> Nat { return match n { cons(x, xs) => x } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_match_list_missing_cons() {
    let errors =
        typecheck("language core; fn main(n : [Nat]) -> Nat { return match n { [] => 0 } }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_error_exhaustive_match_list_patterns() {
    let errors = typecheck(
        "language core; fn main(n : [Nat]) -> Nat { return match n { [] => 0 | [a] => a | cons(a, cons(b, rest)) => b } }",
    );
    assert!(errors.is_empty(), "unexpected errors, got: {errors:?}");
}
#[test]
fn test_error_nonexhaustive_match_list_patterns() {
    let errors = typecheck(
        "language core; fn main(n : [Nat]) -> Nat { return match n { [] => 0 | [a] => a | [a, b, rest] => b | cons(a, cons(b, cons(c, rest))) => c } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_match_catchall_covers_bool() {
    let errors = typecheck("language core; fn main(n : Bool) -> Nat { return match n { x => 0 } }");
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "unexpected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_match_catchall_covers_sum() {
    let errors =
        typecheck("language core; fn main(n : Nat + Bool) -> Nat { return match n { x => 0 } }");
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "unexpected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_exhaustive_match_bool_with_both_branches() {
    let errors = typecheck(
        "language core; fn main(n : Bool) -> Nat { return match n { true => 1 | false => 0 } }",
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "unexpected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_exhaustive_match_nat_zero_and_succ() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> Nat { return match n { 0 => 0 | succ(k) => k } }",
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "unexpected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_exhaustive_match_variant_all_labels() {
    let errors = typecheck(
        "language core; fn main(n : <| none, some : Nat |>) -> Nat { return match n { <| none |> => 0 | <| some = x |> => x } }",
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "unexpected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_match_tuple_reports_missing_witness() {
    let errors = typecheck(
        "language core; fn main(n : {Bool, Bool}) -> Nat { return match n { {true, x} => 1 | {false, true} => 2 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
    let missing = missing_witnesses(&errors).expect("expected missing witness list");
    assert_eq!(
        missing,
        ["{false, false}".to_string()],
        "expected exact tuple witness, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_match_record_reports_missing_witness() {
    let errors = typecheck(
        "language core; fn main(n : {x : Bool, y : Bool}) -> Nat { return match n { {x = true, y = b} => 1 | {x = false, y = true} => 2 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns, got: {errors:?}"
    );
    let missing = missing_witnesses(&errors).expect("expected missing witness list");
    assert_eq!(
        missing,
        ["{x = false, y = false}".to_string()],
        "expected exact record witness, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_let_patterns_missing_false() {
    let errors = typecheck("language core; fn main(b : Bool) -> Nat { return let true = b in 1 }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveLetPatterns { .. }
        )),
        "expected NonexhaustiveLetPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_let_patterns_missing_inr() {
    let errors =
        typecheck("language core; fn main(s : Bool + Nat) -> Nat { return let inl(x) = s in 1 }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveLetPatterns { .. }
        )),
        "expected NonexhaustiveLetPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_let_rec_patterns_missing_false() {
    let errors =
        typecheck("language core; fn main(b : Bool) -> Nat { return letrec true = b in 1 }");
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveLetRecPatterns { .. }
        )),
        "expected NonexhaustiveLetRecPatterns, got: {errors:?}"
    );
}

#[test]
fn test_nonexhaustive_let_rec_patterns_missing_inr() {
    let errors = typecheck(
        "language core; fn main(s : Bool + Nat) -> Nat { return letrec inl(x) = s in 1 }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousPatternType { .. }
        )),
        "expected AmbiguousPatternType, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_data_for_non_none_nullary_label() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> <| done |> { return <| done = 1 |> }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedDataForNullaryLabel { .. }
        )),
        "expected UnexpectedDataForNullaryLabel for label 'done', got: {errors:?}"
    );
}

#[test]
fn test_error_missing_data_for_non_some_label() {
    let errors = typecheck(
        "language core; fn main(n : Nat) -> <| value : Nat |> { return <| value |> }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::MissingDataForLabel { .. })),
        "expected MissingDataForLabel for label 'value', got: {errors:?}"
    );
}

#[test]
fn test_error_tuple_pattern_arity_mismatch_in_let() {
    let errors = typecheck(
        "language core; fn main(triple : {Nat, Nat, Nat}) -> Nat { return let {x, y} = triple in x }",
    );
    assert!(
        !errors.is_empty(),
        "expected a type error for arity-3 tuple matched by a 2-element pattern, but got none"
    );
}

#[test]
fn test_nonexhaustive_match_despite_ambiguous_first_case_body() {
    let errors = typecheck(
        "language core; fn main(b : Bool) -> Nat { return let x = match b { true => inl(0) } in 0 }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for missing `false` branch, got: {errors:?}"
    );
}

#[test]
fn test_well_typed_mutual_recursion() {
    let errors = typecheck(
        "language core;
         fn is_even(n : Nat) -> Bool {
             return match n { 0 => true | succ(k) => is_odd(k) }
         }
         fn is_odd(n : Nat) -> Bool {
             return match n { 0 => false | succ(k) => is_even(k) }
         }
         fn main(n : Nat) -> Bool { return is_even(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_well_typed_natrec_sum() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat {
             return Nat::rec(n, 0, fn(k : Nat) { return fn(acc : Nat) { return succ(acc) } })
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_well_typed_fix_factorial() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat {
             return fix(fn(self : fn(Nat) -> Nat) {
                 return fn(k : Nat) {
                     return match k { 0 => 1 | succ(m) => self(m) }
                 }
             })(n)
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_well_typed_nested_sum_match_exhaustive() {
    let errors = typecheck(
        "language core;
         fn main(s : Nat + (Bool + Nat)) -> Nat {
             return match s {
                 inl(n)       => n
               | inr(inl(b))  => 0
               | inr(inr(m))  => m
             }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_error_nonexhaustive_nested_sum_missing_inr_inr() {
    let errors = typecheck(
        "language core;
         fn main(s : Nat + (Bool + Nat)) -> Nat {
             return match s {
                 inl(n)      => n
               | inr(inl(b)) => 0
             }
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for missing inr(inr(_)), got: {errors:?}"
    );
    let missing = missing_witnesses(&errors).expect("expected missing witness");
    assert_eq!(missing, ["inr(inr(_))"], "wrong witness: {missing:?}");
}

#[test]
fn test_error_nonexhaustive_variant_match_three_labels() {
    let errors = typecheck(
        "language core;
         fn main(v : <| a : Nat, b : Bool, c : Nat |>) -> Nat {
             return match v {
                 <| a = x |> => x
               | <| b = flag |> => 0
             }
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for missing <| c |>, got: {errors:?}"
    );
}

#[test]
fn test_error_natrec_wrong_step_type() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat {
             return Nat::rec(n, 0, fn(k : Nat) { return 0 })
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
                | TypeError::UnexpectedLambda { .. }
                | TypeError::UnexpectedTypeForParameter { .. }
        )),
        "expected type error for step fn with wrong type, got: {errors:?}"
    );
}

#[test]
fn test_error_fix_param_return_type_mismatch() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat { return fix(fn(x : Nat) { return true }) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for fix with mismatched param/return types, got: {errors:?}"
    );
}

#[test]
fn test_error_fix_wrong_arity() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat { return fix(fn(f : Nat, g : Bool) { return 0 })(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IncorrectNumberOfArguments { .. }
        )),
        "expected IncorrectNumberOfArguments for fix applied to 2-param function, got: {errors:?}"
    );
}

#[test]
fn test_well_typed_type_ascription() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat { return (succ(n) as Nat) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_error_type_ascription_wrong_type() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Bool { return (succ(n) as Bool) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for wrong type ascription, got: {errors:?}"
    );
}

#[test]
fn test_error_match_case_type_mismatch() {
    let errors = typecheck(
        "language core;
         fn main(b : Bool) -> Nat {
             return let result = match b { true => 1 | false => true } in result
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for mismatched case types, got: {errors:?}"
    );
}

#[test]
fn test_error_higher_order_wrong_return_type() {
    let errors = typecheck(
        "language core;
         fn apply(f : fn(Nat) -> Bool, x : Nat) -> Bool { return f(x) }
         fn bad(n : Nat) -> Nat { return n }
         fn main(n : Nat) -> Bool { return apply(bad, n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for higher-order function mismatch, got: {errors:?}"
    );
}

#[test]
fn test_well_typed_nested_let_shadowing() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Bool {
             return let x = n in
                    let x = (n == 0) in
                    x
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_well_typed_nat_match_specific_ints_and_catch_all_succ() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat {
             return match n {
                 0 => 0
               | 1 => 1
               | 2 => 2
               | succ(succ(succ(k))) => k
             }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_error_nonexhaustive_nat_match_specific_ints_only() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> Nat {
             return match n { 0 => 0 | 1 => 1 | 2 => 2 }
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for uncovered succ(succ(succ(_))), got: {errors:?}"
    );
}

#[test]
fn test_error_nonexhaustive_nested_variant_in_sum() {
    let errors = typecheck(
        "language core;
         fn main(s : <| a : Bool, b : Nat |> + Nat) -> Nat {
             return match s {
                 inl(<| a = flag |>) => 0
               | inr(n)             => n
             }
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for missing inl(<| b = _ |>), got: {errors:?}"
    );
}

#[test]
fn test_well_typed_exhaustive_nested_variant_in_sum() {
    let errors = typecheck(
        "language core;
         fn main(s : <| a : Bool, b : Nat |> + Nat) -> Nat {
             return match s {
                 inl(<| a = flag |>) => 0
               | inl(<| b = n |>)   => n
               | inr(m)             => m
             }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_error_cons_tail_wrong_type() {
    let errors = typecheck(
        "language core;
         fn main(n : Nat) -> [Nat] {
             return cons(n, [true, false])
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
                | TypeError::UnexpectedList { .. }
        )),
        "expected type error for cons with Bool tail on Nat list, got: {errors:?}"
    );
}
