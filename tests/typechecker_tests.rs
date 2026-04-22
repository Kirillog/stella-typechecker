use stella_typechecker::parser;
use stella_typechecker::type_error::{TypeCheckError, TypeError};
use stella_typechecker::typechecker::TypeChecker;

fn run_etalon(src: &str) -> Option<Result<(), Vec<String>>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    if std::env::var("STELLA_COMPARE_ETALON").unwrap_or_default() != "1" {
        return None;
    }

    let mut child = match Command::new("docker")
        .args(["run", "--rm", "-i", "fizruk/stella", "typecheck"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("[SKIP] docker not found — etalon verification skipped");
            return None;
        }
        Err(e) => panic!("failed to spawn docker: {e}"),
        Ok(c) => c,
    };

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(src.as_bytes()).ok();
    }

    let output = child
        .wait_with_output()
        .expect("docker wait_with_output failed");

    if output.status.success() {
        Some(Ok(()))
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);

        let tags: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                if let Some(rest) = line.strip_prefix("Type Error Tag: [") {
                    return rest.strip_suffix(']').map(str::to_string);
                }
                if let Some(rest) = line.strip_prefix("Unsupported Syntax Error: ") {
                    return Some(rest.trim().to_string());
                }
                None
            })
            .collect();
        Some(Err(tags))
    }
}

fn typecheck(src: &str) -> Vec<TypeCheckError> {
    let prog = parser::ProgramParser::new()
        .parse(src)
        .expect("parse failed");
    let errors = TypeChecker::new().check_program(&prog, src);

    if let Some(etalon_result) = run_etalon(src) {
        let our_ok = errors.is_empty();
        let etalon_ok = etalon_result.is_ok();
        assert_eq!(
            our_ok,
            etalon_ok,
            "etalon disagrees: ours={} etalon={} for:\n{}",
            if our_ok { "OK" } else { "ERROR" },
            if etalon_ok { "OK" } else { "ERROR" },
            src
        );
        if let Err(etalon_tags) = &etalon_result {
            let our_tag = errors
                .first()
                .and_then(|e| e.error.to_string().split(':').next().map(str::to_string))
                .unwrap_or_default();
            assert!(
                etalon_tags.is_empty() || etalon_tags.contains(&our_tag),
                "error tag mismatch: ours={our_tag} etalon={etalon_tags:?} for:\n{src}"
            );
        }
    }

    errors
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
    let errors = typecheck(
        "language core; extend with #natural-literals; fn main(n : Nat) -> Nat { return n(1) }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAFunction { .. })),
        "expected NotAFunction, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_tuple() {
    let errors =
        typecheck("language core; extend with #tuples; fn main(n : Nat) -> Nat { return n.1 }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotATuple { .. })),
        "expected NotATuple, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_record() {
    let errors =
        typecheck("language core; extend with #records; fn main(n : Nat) -> Nat { return n.foo }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotARecord { .. })),
        "expected NotARecord, got: {errors:?}"
    );
}

#[test]
fn test_error_not_a_list() {
    let errors = typecheck(
        "language core; extend with #lists; fn main(n : Nat) -> Bool { return List::isempty(n) }",
    );
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
        "language core; extend with #natural-literals; fn main(n : Nat) -> fn(Bool) -> Nat { return fn(x : Nat) { return 1 } }",
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
    let errors = typecheck("language core; extend with #tuples, #natural-literals; fn main(n : Nat) -> Nat { return {1, 2} }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedTuple { .. })),
        "expected UnexpectedTuple, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_record() {
    let errors = typecheck("language core; extend with #records, #natural-literals; fn main(n : Nat) -> Nat { return {x = 1} }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedRecord { .. })),
        "expected UnexpectedRecord, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_variant() {
    let errors = typecheck("language core; extend with #variants, #natural-literals; fn main(n : Nat) -> Nat { return <| foo = 1 |> }");
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
    let errors = typecheck("language core; extend with #lists, #natural-literals; fn main(n : Nat) -> Nat { return [1, 2] }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedList { .. })),
        "expected UnexpectedList, got: {errors:?}"
    );
}

#[test]
fn test_error_unexpected_injection() {
    let errors = typecheck("language core; extend with #sum-types, #natural-literals; fn main(n : Nat) -> Nat { return inl(1) }");
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
        typecheck("language core; extend with #records, #natural-literals; fn main(n : Nat) -> {x : Nat, y : Nat} { return {x = 1} }");
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
        typecheck("language core; extend with #records, #natural-literals; fn main(n : Nat) -> {x : Nat} { return {x = 1, y = 2} }");
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
    let errors = typecheck("language core; extend with #records, #natural-literals; fn main(n : Nat) -> Nat { return {x = 1}.y }");
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
        typecheck("language core; extend with #variants, #natural-literals; fn main(n : Nat) -> <| foo : Nat |> { return <| bar = 1 |> }");
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
    let errors = typecheck("language core; extend with #tuples, #natural-literals; fn main(n : Nat) -> Nat { return {1, 2}.3 }");
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
    let errors = typecheck("language core; extend with #tuples, #natural-literals; fn main(n : Nat) -> {Nat, Nat} { return {1, 2, 3} }");
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
    let errors = typecheck("language core; extend with #sum-types, #let-bindings; fn main(n : Nat) -> Nat { return let x = inl(0) in n }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousSumType { .. })),
        "expected AmbiguousSumType, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_variant_type() {
    let errors =
        typecheck("language core; extend with #variants, #let-bindings, #natural-literals; fn main(n : Nat) -> Nat { return let x = <| foo = 1 |> in n }");
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
    let errors = typecheck("language core; extend with #lists, #let-bindings; fn main(n : Nat) -> Nat { return let x = [] in n }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousList { .. })),
        "expected AmbiguousList, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_tuple() {
    let errors = typecheck("language core; extend with #sum-types, #tuples; fn main(n : Nat) -> Nat { return (inl(0)).1 }");
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousTuple { .. })),
        "expected AmbiguousTuple, got: {errors:?}"
    );
}

#[test]
fn test_error_ambiguous_function() {
    let errors = typecheck("language core; extend with #sum-types, #natural-literals; fn main(n : Nat) -> Nat { return (inl(0))(1) }");
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
    let errors = typecheck(
        "language core; extend with #sum-types; fn main(n : Bool) -> Nat { return match n {} }",
    );
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
        typecheck("language core; extend with #sum-types, #structural-patterns, #natural-literals; fn main(n : Bool) -> Nat { return match n { true => 1 } }");
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
        "language core; extend with #sum-types, #structural-patterns, #natural-literals; fn main(n : Nat) -> Nat { return match n { true => 1 | false => 0 } }",
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
        typecheck("language core; extend with #records, #let-bindings, #natural-literals; fn main(n : Nat) -> Nat { return let x = {a = 1, a = 2} in 0 }");
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
        typecheck("language core; extend with #records, #natural-literals; fn main(n : Nat) -> {x : Nat, x : Nat} { return {x = 1} }");
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
        "language core; extend with #variants, #natural-literals; fn main(n : Nat) -> <| foo : Nat, foo : Nat |> { return <| foo = 1 |> }",
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
    let errors = typecheck("language core; extend with #multiparameter-functions; fn main(a : Nat, b : Nat) -> Nat { return a }");
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
        "language core; extend with #multiparameter-functions, #natural-literals;
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
        "language core; extend with #multiparameter-functions, #natural-literals; fn main(n : Nat) -> fn(Nat) -> Nat \
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
        "language core; extend with #records, #structural-patterns, #sum-types; fn main(n : {x : Nat}) -> Nat { return match n { {x = a, x = b} => a } }",
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
        typecheck("language core; extend with #variants, #nullary-variant-labels, #natural-literals; fn main(n : Nat) -> <| none |> { return <| none = 1 |> }");
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
        typecheck("language core; extend with #variants, #nullary-variant-labels; fn main(n : Nat) -> <| some : Nat |> { return <| some |> }");
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
        "language core; extend with #variants, #nullary-variant-labels, #sum-types; fn main(n : <| none |>) -> Nat { return match n { <| none = x |> => 0 } }",
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
        "language core; extend with #variants, #nullary-variant-labels, #sum-types; fn main(n : <| some : Nat |>) -> Nat { return match n { <| some |> => 0 } }",
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
        "language core; extend with #multiparameter-functions;
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
        typecheck("language core; extend with #let-bindings, #natural-literals; fn main(n : Nat) -> Nat { return let x = 1, x = 2 in x }");
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
        "language core; extend with #sum-types, #letrec-bindings, #type-ascriptions; fn main(n : Nat) -> Nat { return letrec inl(x) as (Nat + Bool) = inl(0) as (Nat + Bool) in x }",
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
        "language core; extend with #sum-types; fn main(n : Nat + Bool) -> Nat { return match n { inr(b) => 0 } }",
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
        "language core; extend with #sum-types; fn main(n : Nat + Bool) -> Nat { return match n { inl(x) => x } }",
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
        "language core; extend with #variants, #sum-types; fn main(n : <| a : Nat, b : Nat |>) -> Nat \
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
        typecheck("language core; extend with #sum-types, #structural-patterns; fn main(n : Nat) -> Nat { return match n { succ(x) => x } }");
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
    let errors = typecheck("language core; extend with #sum-types, #structural-patterns; fn main(n : Nat) -> Nat { return match n { 0 => 0 } }");
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
        "language core; extend with #lists, #sum-types, #structural-patterns; fn main(n : [Nat]) -> Nat { return match n { cons(x, xs) => x } }",
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
        typecheck("language core; extend with #lists, #sum-types, #structural-patterns; fn main(n : [Nat]) -> Nat { return match n { [] => 0 } }");
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
        "language core; extend with #lists, #sum-types, #structural-patterns; fn main(n : [Nat]) -> Nat { return match n { [] => 0 | [a] => a | cons(a, cons(b, rest)) => b } }",
    );
    assert!(errors.is_empty(), "unexpected errors, got: {errors:?}");
}
#[test]
fn test_error_nonexhaustive_match_list_patterns() {
    let errors = typecheck(
        "language core; extend with #lists, #sum-types, #structural-patterns; fn main(n : [Nat]) -> Nat { return match n { [] => 0 | [a] => a | [a, b, rest] => b | cons(a, cons(b, cons(c, rest))) => c } }",
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
    let errors = typecheck("language core; extend with #sum-types; fn main(n : Bool) -> Nat { return match n { x => 0 } }");
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
        typecheck("language core; extend with #sum-types; fn main(n : Nat + Bool) -> Nat { return match n { x => 0 } }");
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
        "language core; extend with #sum-types, #structural-patterns, #natural-literals; fn main(n : Bool) -> Nat { return match n { true => 1 | false => 0 } }",
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
        "language core; extend with #sum-types, #structural-patterns; fn main(n : Nat) -> Nat { return match n { 0 => 0 | succ(k) => k } }",
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
        "language core; extend with #variants, #nullary-variant-labels, #sum-types; fn main(n : <| none, some : Nat |>) -> Nat { return match n { <| none |> => 0 | <| some = x |> => x } }",
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
        "language core; extend with #tuples, #sum-types, #structural-patterns, #natural-literals; fn main(n : {Bool, Bool}) -> Nat { return match n { {true, x} => 1 | {false, true} => 2 } }",
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
        "language core; extend with #records, #sum-types, #structural-patterns, #natural-literals; fn main(n : {x : Bool, y : Bool}) -> Nat { return match n { {x = true, y = b} => 1 | {x = false, y = true} => 2 } }",
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
fn test_nonexhaustive_let_patterns_missing_inr() {
    let errors =
        typecheck("language core; extend with #sum-types, #let-bindings, #natural-literals; fn main(s : Bool + Nat) -> Nat { return let inl(x) = s in 1 }");
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
        typecheck("language core; extend with #letrec-bindings, #structural-patterns, #natural-literals; fn main(b : Bool) -> Nat { return letrec true = b in 1 }");
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
        "language core; extend with #sum-types, #letrec-bindings, #natural-literals; fn main(s : Bool + Nat) -> Nat { return letrec inl(x) = s in 1 }",
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
fn test_error_tuple_pattern_arity_mismatch_in_let() {
    let errors = typecheck(
        "language core; extend with #tuples, #let-bindings, #structural-patterns; fn main(triple : {Nat, Nat, Nat}) -> Nat { return let {x, y} = triple in x }",
    );
    assert!(
        !errors.is_empty(),
        "expected a type error for arity-3 tuple matched by a 2-element pattern, but got none"
    );
}

#[test]
fn test_nonexhaustive_match_despite_ambiguous_first_case_body() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns, #let-bindings; fn main(b : Bool) -> Nat { return let x = match b { true => inl(0) } in 0 }",
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
        "language core; extend with #sum-types, #structural-patterns;
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
        "language core; extend with #fixpoint-combinator, #sum-types, #structural-patterns, #natural-literals;
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
        "language core; extend with #sum-types, #structural-patterns;
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
        "language core; extend with #sum-types;
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
        "language core; extend with #variants, #sum-types;
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
        "language core; extend with #fixpoint-combinator;
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
        "language core; extend with #fixpoint-combinator, #multiparameter-functions;
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
        "language core; extend with #type-ascriptions;
         fn main(n : Nat) -> Nat { return (succ(n) as Nat) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_error_type_ascription_wrong_type() {
    let errors = typecheck(
        "language core; extend with #type-ascriptions;
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
        "language core; extend with #let-bindings, #sum-types, #structural-patterns, #natural-literals;
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
fn test_well_typed_nested_let_shadowing() {
    let errors = typecheck(
        "language core; extend with #let-bindings, #let-patterns, #comparison-operators;
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
        "language core; extend with #sum-types, #structural-patterns, #natural-literals;
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
        "language core; extend with #sum-types, #structural-patterns, #natural-literals;
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
        "language core; extend with #variants, #sum-types, #structural-patterns;
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
        "language core; extend with #variants, #sum-types, #structural-patterns;
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
        "language core; extend with #lists;
         fn main(n : Nat) -> [Nat] {
             return cons(n, [true, false])
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. } | TypeError::UnexpectedList { .. }
        )),
        "expected type error for cons with Bool tail on Nat list, got: {errors:?}"
    );
}

#[test]
fn test_sequencing_well_typed() {
    let errors = typecheck(
        "language core; extend with #sequencing, #unit-type;
         fn main(n : Nat) -> Nat { return unit; n }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_sequencing_returns_type_of_second_expr() {
    let errors = typecheck(
        "language core; extend with #sequencing, #unit-type;
         fn main(n : Nat) -> Bool { return unit; Nat::iszero(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_sequencing_first_expr_not_unit() {
    let errors = typecheck(
        "language core; extend with #sequencing;
         fn main(n : Nat) -> Nat { return n; n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for non-Unit sequencing head, got: {errors:?}"
    );
}

#[test]
fn test_sequencing_wrong_result_type() {
    let errors = typecheck(
        "language core; extend with #sequencing, #unit-type;
         fn main(n : Nat) -> Bool { return unit; n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for wrong result type in sequence, got: {errors:?}"
    );
}

#[test]
fn test_reference_new_and_deref_well_typed() {
    let errors = typecheck(
        "language core; extend with #references;
         fn main(n : Nat) -> Nat { return *new(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_reference_assign_well_typed() {
    let errors = typecheck(
        "language core; extend with #references, #unit-type;
         fn main(r : &Nat) -> Unit { return r := succ(*r) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_reference_unexpected_type_deref() {
    let errors = typecheck(
        "language core; extend with #references;
         fn main(n : Nat) -> Nat { return *n }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedTypeForExpression { .. })),
        "expected UnexpectedTypeForExpression for deref of non-reference, got: {errors:?}"
    );
}

#[test]
fn test_reference_not_a_reference_assign() {
    let errors = typecheck(
        "language core; extend with #references, #unit-type;
         fn main(n : Nat) -> Unit { return n := succ(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAReference { .. })),
        "expected NotAReference for assign to non-reference, got: {errors:?}"
    );
}

#[test]
fn test_reference_unexpected_reference() {
    let errors = typecheck(
        "language core; extend with #references;
         fn main(n : Nat) -> Nat { return new(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::UnexpectedReference { .. })),
        "expected UnexpectedReference for new(...) where non-reference type expected, got: {errors:?}"
    );
}

#[test]
fn test_panic_well_typed_as_nat() {
    let errors = typecheck(
        "language core; extend with #panic;
         fn main(n : Nat) -> Nat { return panic! }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_panic_ambiguous_type() {
    let errors = typecheck(
        "language core; extend with #panic, #let-bindings;
         fn main(n : Nat) -> Nat { return let x = panic! in n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousPanicType { .. }
        )),
        "expected AmbiguousPanicType, got: {errors:?}"
    );
}

#[test]
fn test_exceptions_throw_and_catch_well_typed() {
    let errors = typecheck(
        "language core; extend with #exceptions, #exception-type-declaration;
         exception type = Nat
         fn main(n : Nat) -> Bool {
             return try { throw(n) } catch { x => false }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_exceptions_try_with_well_typed() {
    let errors = typecheck(
        "language core; extend with #exceptions, #exception-type-declaration;
         exception type = Nat
         fn main(n : Nat) -> Nat {
             return try { throw(n) } with { succ(0) }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_exceptions_type_not_declared() {
    let errors = typecheck(
        "language core; extend with #exceptions;
         fn main(n : Nat) -> Bool { return throw(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::ExceptionTypeNotDeclared { .. }
        )),
        "expected ExceptionTypeNotDeclared, got: {errors:?}"
    );
}

#[test]
fn test_exceptions_ambiguous_throw_type() {
    let errors = typecheck(
        "language core; extend with #exceptions, #exception-type-declaration, #let-bindings;
         exception type = Nat
         fn main(n : Nat) -> Nat { return let x = throw(n) in n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousThrowType { .. }
        )),
        "expected AmbiguousThrowType, got: {errors:?}"
    );
}

#[test]
fn test_subtyping_record_extra_field_well_typed() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #records;
         fn take_x(r : {x : Nat}) -> Nat { return r.x }
         fn main(n : Nat) -> Nat { return take_x({x = n, y = true}) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_nat_not_subtype_of_bool() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping;
         fn main(n : Nat) -> Bool { return n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. }
        )),
        "expected UnexpectedSubtype, got: {errors:?}"
    );
}

#[test]
fn test_top_type_nat_is_subtype_of_top() {
    let errors = typecheck(
        "language core;
         extend with #top-type, #structural-subtyping;
         fn main(n : Nat) -> Top { return n }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_type_cast_well_typed() {
    let errors = typecheck(
        "language core;
         extend with #type-cast;
         fn main(n : Nat) -> Nat { return n cast as Nat }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_type_cast_to_top_and_back() {
    let errors = typecheck(
        "language core;
         extend with #type-cast, #top-type, #structural-subtyping;
         fn main(n : Nat) -> Nat { return (n cast as Top) cast as Nat }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_open_variant_exceptions_try_catch_well_typed() {
    let errors = typecheck(
        "language core;
         extend with #open-variant-exceptions, #exceptions, #variants;
         exception variant MyError : Nat
         fn fail(n : Nat) -> Bool { return throw(<| MyError = n |>) }
         fn main(n : Nat) -> Bool {
           return try { fail(n) } catch { <| MyError = x |> => false }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_open_variant_exceptions_wrong_label_error() {
    let errors = typecheck(
        "language core;
         extend with #open-variant-exceptions, #exceptions, #variants;
         exception variant MyError : Nat
         fn main(n : Nat) -> Bool { return throw(<| OtherError = n |>) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedVariantLabel { .. }
        )),
        "expected UnexpectedVariantLabel, got: {errors:?}"
    );
}

#[test]
fn test_duplicate_exception_type_error() {
    let errors = typecheck(
        "language core; extend with #exception-type-declaration;
         exception type = Nat
         exception type = Bool
         fn main(n : Nat) -> Nat { return n }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::DuplicateExceptionType)),
        "expected DuplicateExceptionType, got: {errors:?}"
    );
}

#[test]
fn test_duplicate_exception_variant_error() {
    let errors = typecheck(
        "language core;
         extend with #open-variant-exceptions;
         exception variant Foo : Nat
         exception variant Foo : Bool
         fn main(n : Nat) -> Nat { return n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateExceptionVariant { label } if label == "Foo"
        )),
        "expected DuplicateExceptionVariant(Foo), got: {errors:?}"
    );
}

#[test]
fn test_conflicting_exception_declarations_error() {
    let errors = typecheck(
        "language core;
         extend with #open-variant-exceptions, #exception-type-declaration;
         exception type = Nat
         exception variant Foo : Nat
         fn main(n : Nat) -> Nat { return n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::ConflictingExceptionDeclarations
        )),
        "expected ConflictingExceptionDeclarations, got: {errors:?}"
    );
}

#[test]
fn test_illegal_local_exception_type_error() {
    let errors = typecheck(
        "language core; extend with #exception-type-declaration;
         fn main(n : Nat) -> Nat {
           exception type = Nat
           return n
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IllegalLocalExceptionType
        )),
        "expected IllegalLocalExceptionType, got: {errors:?}"
    );
}

#[test]
fn test_illegal_local_open_variant_exception_error() {
    let errors = typecheck(
        "language core;
         extend with #open-variant-exceptions;
         fn main(n : Nat) -> Nat {
           exception variant Foo : Nat
           return n
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IllegalLocalOpenVariantException
        )),
        "expected IllegalLocalOpenVariantException, got: {errors:?}"
    );
}

#[test]
fn test_try_cast_as_well_typed() {
    let errors = typecheck(
        "language core;
         extend with #try-cast-as, #top-type, #type-cast;
         fn main(n : Nat) -> Nat {
           return try { n cast as Top } cast as Nat { x => x } with { 0 }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_try_cast_as_with_branch_type_mismatch() {
    let errors = typecheck(
        "language core;
         extend with #try-cast-as, #top-type, #type-cast;
         fn main(n : Nat) -> Nat {
           return try { n cast as Top } cast as Nat { x => true } with { 0 }
         }",
    );
    assert!(
        !errors.is_empty(),
        "expected a type error for mismatched branch types"
    );
}

#[test]
fn test_type_cast_patterns_well_typed() {
    let errors = typecheck(
        "language core;
         extend with #type-cast-patterns, #type-cast, #top-type, #structural-subtyping;
         fn main(n : Nat) -> Nat {
           return match (n cast as Top) {
               x cast as Nat => x
           }
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns: cast-as patterns are not catch-alls, got: {errors:?}"
    );
}

#[test]
fn test_type_cast_patterns_binding_in_let() {
    let errors = typecheck(
        "language core;
         extend with #type-cast-patterns, #type-cast, #top-type, #structural-subtyping,
                     #let-patterns, #let-bindings;
         fn main(n : Nat) -> Nat {
           return let (x cast as Nat) = (n cast as Top) in x
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveLetPatterns { .. }
        )),
        "expected NonexhaustiveLetPatterns: cast-as pattern in let is non-exhaustive, got: {errors:?}"
    );
}

#[test]
fn test_local_exception_type_throw_no_spurious_error() {
    let errors = typecheck(
        "language core; extend with #exceptions, #exception-type-declaration;
         fn main(n : Nat) -> Nat {
           exception type = Nat
           return throw(succ(0))
         }",
    );

    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::ExceptionTypeNotDeclared { .. }
        )),
        "must not produce ExceptionTypeNotDeclared alongside IllegalLocalExceptionType: {errors:?}"
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IllegalLocalExceptionType
        )),
        "expected IllegalLocalExceptionType: {errors:?}"
    );
}

#[test]
fn test_nested_function_exception_type_does_not_leak_outward() {
    let errors = typecheck(
        "language core;
         extend with #nested-function-declarations, #exceptions, #exception-type-declaration;
         fn outer(n : Nat) -> Nat {
           fn inner(m : Nat) -> Nat {
             exception type = Nat
             return m
           }
           return throw(succ(0))
         }
         fn main(n : Nat) -> Nat { return outer(n) }",
    );

    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::IllegalLocalExceptionType
        )),
        "expected IllegalLocalExceptionType: {errors:?}"
    );

    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::ExceptionTypeNotDeclared { .. } | TypeError::AmbiguousThrowType { .. }
        )),
        "expected ExceptionTypeNotDeclared or AmbiguousThrowType for outer's throw: {errors:?}"
    );
}

#[test]
fn test_subtyping_fun_wrong_return_type_error() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #multiparameter-functions;
         fn apply(f : fn(Nat) -> Bool, x : Nat) -> Bool { return f(x) }
         fn id_nat(n : Nat) -> Nat { return n }
         fn main(n : Nat) -> Bool { return apply(id_nat, n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. } | TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected type error passing fn->Nat where fn->Bool expected, got: {errors:?}"
    );
}

#[test]
fn test_subtyping_fun_wrong_param_type_error() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #multiparameter-functions;
         fn apply(f : fn(Nat) -> Nat, x : Nat) -> Nat { return f(x) }
         fn take_bool(x : Bool) -> Nat { return 0 }
         fn main(n : Nat) -> Nat { return apply(take_bool, n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. } | TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected type error passing fn(Bool)->Nat where fn(Nat)->Nat expected, got: {errors:?}"
    );
}

#[test]
fn test_subtyping_fun_arity_mismatch_not_subtype() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #multiparameter-functions;
         fn apply(f : fn(Nat) -> Nat, x : Nat) -> Nat { return f(x) }
         fn two_args(a : Nat, b : Nat) -> Nat { return a }
         fn main(n : Nat) -> Nat { return apply(two_args, n) }",
    );
    assert!(
        !errors.is_empty(),
        "expected error: 2-param fn cannot satisfy 1-param fn type, got none"
    );
}

#[test]
fn test_subtyping_tuple_element_subtype() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #tuples, #panic;
         fn take_pair(p : {Nat, Nat}) -> Nat { return p.1 }
         fn main(n : Nat) -> Nat { return take_pair({panic!, n}) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_list_element_subtype() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #lists, #panic;
         fn head_nat(xs : [Nat]) -> Nat { return List::head(xs) }
         fn main(n : Nat) -> Nat { return head_nat([panic!]) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_sum_covariant() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #sum-types, #panic;
         fn take_sum(s : Nat + Bool) -> Nat { return 0 }
         fn main(n : Nat) -> Nat { return take_sum(inl(panic!)) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_variant_structural() {
    let errors = typecheck(
        "language core;
         extend with #structural-subtyping, #variants, #top-type;
         fn take_variant(v : <| a : Top |>) -> Nat { return 0 }
         fn main(n : Nat) -> Nat { return take_variant(<| a = n |>) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_type_cast_expr() {
    let errors = typecheck(
        "language core;
         extend with #type-cast, #top-type, #let-bindings;
         fn main(n : Nat) -> Top { return let x = n cast as Top in x }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_inl_with_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #ambiguous-type-as-bottom, #sum-types;
         fn main(n : Nat) -> Nat + Bool { return inl(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_inr_with_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #ambiguous-type-as-bottom, #sum-types;
         fn main(n : Nat) -> Bool + Nat { return inr(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_variant_with_payload_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #ambiguous-type-as-bottom, #variants, #let-bindings, #let-patterns, #structural-subtyping;
         fn main(n : Nat) -> Nat { return let _ = <| foo = n |> in n }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_variant_no_payload_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #ambiguous-type-as-bottom, #variants, #nullary-variant-labels, #let-bindings, #let-patterns, #structural-subtyping;
         fn main(n : Nat) -> Nat { return let _ = <| none |> in n }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_empty_list_with_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #ambiguous-type-as-bottom, #lists, #let-bindings;
         fn main(n : Nat) -> Nat { return let _ = [] in n }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_infer_letrec_duplicate_binding() {
    let errors = typecheck(
        "language core;
         extend with #letrec-bindings, #type-ascriptions;
         fn main(n : Nat) -> Nat {
             return letrec (x as Nat) = succ(0), (x as Nat) = succ(0) in x
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::DuplicateLetBinding { .. }
        )),
        "expected DuplicateLetBinding in letrec, got: {errors:?}"
    );
}

#[test]
fn test_check_conslist_against_list_type() {
    let errors = typecheck(
        "language core; extend with #lists;
         fn main(n : Nat) -> [Nat] { return cons(n, []) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_check_conslist_wrong_type_error() {
    let errors = typecheck(
        "language core; extend with #lists;
         fn main(n : Nat) -> [Bool] { return cons(n, []) }",
    );
    assert!(
        !errors.is_empty(),
        "expected type error for cons(Nat, _) against [Bool], got none"
    );
}

#[test]
fn test_check_trycatch_no_exception_type() {
    let errors = typecheck(
        "language core; extend with #exceptions;
         fn main(n : Nat) -> Nat { return try { n } catch { x => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::ExceptionTypeNotDeclared { .. }
        )),
        "expected ExceptionTypeNotDeclared in check path, got: {errors:?}"
    );
}

#[test]
fn test_check_deref_non_reference_type() {
    let errors = typecheck(
        "language core; extend with #references;
         fn main(r : &Nat) -> Bool { return *r }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. } | TypeError::UnexpectedSubtype { .. }
        )),
        "expected type mismatch when dereffing &Nat where Bool expected, got: {errors:?}"
    );
}

#[test]
fn test_check_assign_expected_not_unit() {
    let errors = typecheck(
        "language core; extend with #references;
         fn main(r : &Nat) -> Bool { return r := 0 }",
    );
    assert!(
        !errors.is_empty(),
        "expected error: assign returns Unit, not Bool, got none"
    );
}

#[test]
fn test_check_const_memory_not_ref_expected() {
    let errors = typecheck(
        "language core;
         extend with #references;
         fn main(n : Nat) -> Nat { return <0x0> }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedMemoryAddress { .. }
        )),
        "expected UnexpectedMemoryAddress, got: {errors:?}"
    );
}

#[test]
fn test_check_application_ambiguous_function() {
    let errors = typecheck(
        "language core;
         extend with #sum-types, #natural-literals;
         fn main(n : Nat) -> Nat { return (inl(0))(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousFunction { .. }
        )),
        "expected AmbiguousFunction from check path, got: {errors:?}"
    );
}

#[test]
fn test_check_isempty_none_infer_ambiguous_list() {
    let errors = typecheck(
        "language core;
         extend with #sum-types, #lists, #natural-literals;
         fn main(n : Nat) -> Bool { return List::isempty(inl(0)) }",
    );
    assert!(
        !errors.is_empty(),
        "expected errors for isempty on ambiguous expression, got none"
    );
}

#[test]
fn test_pattern_tuple_on_non_tuple() {
    let errors = typecheck(
        "language core; extend with #tuples, #sum-types;
         fn main(n : Nat) -> Nat { return match n { {a, b} => a } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for tuple pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_record_on_non_record() {
    let errors = typecheck(
        "language core; extend with #records, #sum-types;
         fn main(n : Nat) -> Nat { return match n { {x = a} => a } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for record pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_list_on_non_list() {
    let errors = typecheck(
        "language core; extend with #lists, #sum-types;
         fn main(n : Nat) -> Nat { return match n { [] => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for list pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_cons_on_non_list() {
    let errors = typecheck(
        "language core; extend with #lists, #sum-types;
         fn main(n : Nat) -> Nat { return match n { cons(h, t) => h } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for cons pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_inl_on_non_sum() {
    let errors = typecheck(
        "language core; extend with #sum-types;
         fn main(n : Bool) -> Nat { return match n { inl(x) => 0 | inr(x) => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for inl on Bool, got: {errors:?}"
    );
}

#[test]
fn test_pattern_inr_on_non_sum() {
    let errors = typecheck(
        "language core; extend with #sum-types;
         fn main(n : Bool) -> Nat { return match n { inr(x) => 0 | inl(x) => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for inr on Bool, got: {errors:?}"
    );
}

#[test]
fn test_pattern_succ_on_non_nat() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns;
         fn main(b : Bool) -> Nat { return match b { succ(k) => k | 0 => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for succ pattern on Bool, got: {errors:?}"
    );
}

#[test]
fn test_pattern_bool_on_non_bool() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns;
         fn main(n : Nat) -> Nat { return match n { true => 1 | false => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for bool pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_unit_on_non_unit() {
    let errors = typecheck(
        "language core; extend with #sum-types, #unit-type;
         fn main(n : Nat) -> Nat { return match n { unit => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for unit pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_pattern_nat_literal_on_non_nat() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns, #natural-literals;
         fn main(b : Bool) -> Nat { return match b { 0 => 0 | succ(k) => k } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for nat literal pattern on Bool, got: {errors:?}"
    );
}

#[test]
fn test_pattern_record_field_access_no_such_field() {
    let errors = typecheck(
        "language core; extend with #records, #sum-types;
         fn main(r : {x : Nat}) -> Nat { return match r { {x = a, z = b} => a } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedFieldAccess { .. }
        )),
        "expected UnexpectedFieldAccess for unknown record field in pattern, got: {errors:?}"
    );
}

#[test]
fn test_pattern_variant_on_non_variant() {
    let errors = typecheck(
        "language core; extend with #variants, #sum-types;
         fn main(n : Nat) -> Nat { return match n { <| foo = x |> => x } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedPatternForType { .. }
        )),
        "expected UnexpectedPatternForType for variant pattern on Nat, got: {errors:?}"
    );
}

#[test]
fn test_check_inl_against_sum_type() {
    let errors = typecheck(
        "language core; extend with #sum-types;
         fn main(n : Nat) -> Nat + Bool { return inl(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_check_inr_against_sum_type() {
    let errors = typecheck(
        "language core; extend with #sum-types;
         fn main(n : Nat) -> Bool + Nat { return inr(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_check_list_literal_against_list_type() {
    let errors = typecheck(
        "language core; extend with #lists;
         fn main(n : Nat) -> [Nat] { return [n] }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_exhaustiveness_with_specific_int_patterns_covered() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns, #natural-literals;
         fn main(n : Nat) -> Nat {
             return match n {
                 0 => 0
               | 1 => 1
               | 2 => 2
               | 3 => 3
               | succ(succ(succ(succ(k)))) => k
             }
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_exhaustiveness_int_patterns_nonexhaustive() {
    let errors = typecheck(
        "language core; extend with #sum-types, #structural-patterns, #natural-literals;
         fn main(n : Nat) -> Nat { return match n { 0 => 0 | 1 => 1 | 2 => 2 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::NonexhaustiveMatchPatterns { .. }
        )),
        "expected NonexhaustiveMatchPatterns for integers-only match, got: {errors:?}"
    );
}

#[test]
fn test_infer_if_else_type_mismatch() {
    let errors = typecheck(
        "language core;
         fn main(b : Bool) -> Nat { return if b then 0 else true }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTypeForExpression { .. }
        )),
        "expected UnexpectedTypeForExpression for if/else type mismatch, got: {errors:?}"
    );
}

#[test]
fn test_check_trywith_type_mismatch() {
    let errors = typecheck(
        "language core;
         extend with #exceptions, #exception-type-declaration;
         exception type = Nat
         fn main(n : Nat) -> Nat {
             return try { n } with { true }
         }",
    );
    assert!(
        !errors.is_empty(),
        "expected error for TryWith with mismatched handler type, got none"
    );
}

#[test]
fn test_check_dottuple_out_of_bounds() {
    let errors = typecheck(
        "language core; extend with #tuples, #natural-literals;
         fn main(n : Nat) -> Nat { return {n, n}.3 }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::TupleIndexOutOfBounds { .. }
        )),
        "expected TupleIndexOutOfBounds in check path, got: {errors:?}"
    );
}

#[test]
fn test_check_dottuple_not_a_tuple() {
    let errors = typecheck(
        "language core; extend with #tuples;
         fn main(n : Nat) -> Nat { return n.1 }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotATuple { .. })),
        "expected NotATuple in check path, got: {errors:?}"
    );
}

#[test]
fn test_check_dottuple_ambiguous_tuple() {
    let errors = typecheck(
        "language core; extend with #sum-types, #tuples, #natural-literals;
         fn main(n : Nat) -> Nat { return (inl(0)).1 }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::AmbiguousTuple { .. })),
        "expected AmbiguousTuple in check path, got: {errors:?}"
    );
}

#[test]
fn test_infer_tail_not_a_list() {
    let errors = typecheck(
        "language core; extend with #lists;
         fn main(n : Nat) -> [Nat] { return List::tail(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAList { .. })),
        "expected NotAList for List::tail on Nat, got: {errors:?}"
    );
}

#[test]
fn test_infer_isempty_not_a_list() {
    let errors = typecheck(
        "language core; extend with #lists;
         fn main(n : Nat) -> Bool { return List::isempty(n) }",
    );
    assert!(
        has_error(&errors, |e| matches!(e, TypeError::NotAList { .. })),
        "expected NotAList for List::isempty on Nat, got: {errors:?}"
    );
}

#[test]
fn test_infer_const_memory_no_ambiguous_extension() {
    let errors = typecheck(
        "language core;
         extend with #references, #let-bindings;
         fn main(n : Nat) -> Nat { return let x = <0x0> in n }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::AmbiguousReferenceType { .. }
        )),
        "expected AmbiguousReferenceType, got: {errors:?}"
    );
}

#[test]
fn test_infer_trycatch_no_exception_type() {
    let errors = typecheck(
        "language core; extend with #exceptions;
         fn main(n : Nat) -> Nat { return try { n } catch { x => 0 } }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::ExceptionTypeNotDeclared { .. }
        )),
        "expected ExceptionTypeNotDeclared in infer TryCatch path, got: {errors:?}"
    );
}

#[test]
fn test_infer_trycatch_type_mismatch() {
    let errors = typecheck(
        "language core;
         extend with #exceptions, #exception-type-declaration;
         exception type = Nat
         fn main(n : Nat) -> Nat {
             return let _ = try { n } catch { x => true } in n
         }",
    );
    assert!(
        !errors.is_empty(),
        "expected type error for try/catch type mismatch, got none"
    );
}

#[test]
fn test_infer_throw_with_ambiguous_type_as_bottom() {
    let errors = typecheck(
        "language core;
         extend with #exceptions, #exception-type-declaration, #ambiguous-type-as-bottom;
         exception type = Nat
         fn main(n : Nat) -> Nat { return throw(n) }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_missing_record_fields_emits_missing_record_fields_not_unexpected_subtype() {
    let errors = typecheck(
        "language core;
         extend with #records, #let-bindings, #let-patterns, #structural-subtyping;
         fn main(n : Nat) -> { a : Nat, b : Bool } {
           return let x = { a = 0 } in x
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::MissingRecordFields { missing, .. } if missing.contains(&"b".to_string())
        )),
        "expected MissingRecordFields(b), got: {errors:?}"
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. }
        )),
        "should not emit UnexpectedSubtype when MissingRecordFields applies, got: {errors:?}"
    );
}

#[test]
fn test_subtyping_record_ok_when_extra_fields_present() {
    let errors = typecheck(
        "language core;
         extend with #records, #let-bindings, #let-patterns, #natural-literals, #structural-subtyping;
         fn main(n : Nat) -> { a : Nat } {
           return let x = { a = 1, b = 2 } in x
         }",
    );
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_subtyping_tuple_length_mismatch_emits_unexpected_tuple_length() {
    let errors = typecheck(
        "language core;
         extend with #tuples, #let-bindings, #let-patterns, #natural-literals, #structural-subtyping;
         fn main(n : Nat) -> {Nat, Nat, Nat} {
           return let x = {1, 2} in x
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedTupleLength {
                expected: 3,
                got: 2,
                ..
            }
        )),
        "expected UnexpectedTupleLength(3, 2), got: {errors:?}"
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. }
        )),
        "should not emit UnexpectedSubtype when UnexpectedTupleLength applies, got: {errors:?}"
    );
}

#[test]
fn test_subtyping_variant_extra_label_emits_unexpected_variant_label() {
    let errors = typecheck(
        "language core;
         extend with #variants, #let-bindings, #let-patterns, #structural-subtyping;
         fn main(n : Nat) -> <| a : Nat |> {
           return let x = <| b = n |> as <| a : Nat, b : Nat |> in x
         }",
    );
    assert!(
        has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedVariantLabel { label, .. } if label == "b"
        )),
        "expected UnexpectedVariantLabel(b), got: {errors:?}"
    );
    assert!(
        !has_error(&errors, |e| matches!(
            e,
            TypeError::UnexpectedSubtype { .. }
        )),
        "should not emit UnexpectedSubtype when UnexpectedVariantLabel applies, got: {errors:?}"
    );
}
