use crate::ast::Type;

use std::fmt;

#[derive(Debug, Clone)]
pub enum TypeError {
    // 1
    MissingMain,
    // 2
    UndefinedVariable(String),
    // 3
    UnexpectedTypeForExpression {
        expected: Type,
        got: Type,
    },
    // 4
    NotAFunction(Type),
    // 5
    NotATuple(Type),
    // 6
    NotARecord(Type),
    // 7
    NotAList(Type),
    // 8
    UnexpectedLambda {
        expected: Type,
    },
    // 9
    UnexpectedTypeForParameter {
        param: String,
        expected: Type,
        got: Type,
    },
    // 10
    UnexpectedTuple {
        expected: Type,
    },
    // 11
    UnexpectedRecord {
        expected: Type,
    },
    // 12
    UnexpectedVariant {
        expected: Type,
    },
    // 13
    UnexpectedList {
        expected: Type,
    },
    // 14
    UnexpectedInjection {
        expected: Type,
    },
    // 15
    MissingRecordFields {
        missing: Vec<String>,
    },
    // 16
    UnexpectedRecordFields {
        unexpected: Vec<String>,
    },
    // 17
    UnexpectedFieldAccess {
        field: String,
        record_type: Type,
    },
    // 18
    UnexpectedVariantLabel {
        label: String,
        variant_type: Type,
    },
    // 19
    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
    },
    // 20
    UnexpectedTupleLength {
        expected: usize,
        got: usize,
    },
    // 21
    AmbiguousSumType,
    // 22
    AmbiguousVariantType,
    // 23
    AmbiguousList,
    AmbiguousTuple,
    AmbiguousFunction,
    // 24
    IllegalEmptyMatching,
    // 25
    NonexhaustiveMatchPatterns {
        missing: Vec<String>,
    },
    // 26
    UnexpectedPatternForType {
        pattern_desc: String,
        scrutinee_type: Type,
    },
    // 27
    DuplicateRecordFields {
        field: String,
    },
    // 28
    DuplicateRecordTypeFields {
        field: String,
    },
    // 29
    DuplicateVariantTypeFields {
        label: String,
    },
    // 30
    DuplicateFunctionDeclaration {
        name: String,
    },
    // 31
    IncorrectArityOfMain {
        got: usize,
    },
    // 32
    IncorrectNumberOfArguments {
        expected: usize,
        got: usize,
    },
    // 33
    UnexpectedNumberOfParametersInLambda {
        expected: usize,
        got: usize,
    },
    // 34
    DuplicateRecordPatternFields {
        field: String,
    },
    // 35
    UnexpectedDataForNullaryLabel {
        label: String,
    },
    // 36
    MissingDataForLabel {
        label: String,
    },
    // 37
    UnexpectedNonNullaryVariantPattern {
        label: String,
    },
    // 38
    UnexpectedNullaryVariantPattern {
        label: String,
    },
    // 39
    DuplicateFunctionParameter {
        name: String,
    },
    // 40
    DuplicateLetBinding {
        name: String,
    },
    // 41
    DuplicateTypeParameter {
        name: String,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::MissingMain =>
                                write!(f, "ERROR_MISSING_MAIN: no `main` function in program"),
            TypeError::UndefinedVariable(name) =>
                                write!(f, "ERROR_UNDEFINED_VARIABLE: `{name}` is not defined"),
            TypeError::UnexpectedTypeForExpression { expected, got } =>
                                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_EXPRESSION: expected {expected:?}, got {got:?}"),
            TypeError::NotAFunction(ty) =>
                                write!(f, "ERROR_NOT_A_FUNCTION: expected a function type, got {ty:?}"),
            TypeError::NotATuple(ty) =>
                                write!(f, "ERROR_NOT_A_TUPLE: expected a tuple type, got {ty:?}"),
            TypeError::NotARecord(ty) =>
                                write!(f, "ERROR_NOT_A_RECORD: expected a record type, got {ty:?}"),
            TypeError::NotAList(ty) =>
                                write!(f, "ERROR_NOT_A_LIST: expected a list type, got {ty:?}"),
            TypeError::UnexpectedLambda { expected } =>
                                write!(f, "ERROR_UNEXPECTED_LAMBDA: lambda checked against non-function type {expected:?}"),
            TypeError::UnexpectedTypeForParameter { param, expected, got } =>
                                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_PARAMETER: parameter `{param}` expected {expected:?}, got {got:?}"),
            TypeError::UnexpectedTuple { expected } =>
                                write!(f, "ERROR_UNEXPECTED_TUPLE: tuple checked against non-tuple type {expected:?}"),
            TypeError::UnexpectedRecord { expected } =>
                                write!(f, "ERROR_UNEXPECTED_RECORD: record checked against non-record type {expected:?}"),
            TypeError::UnexpectedVariant { expected } =>
                                write!(f, "ERROR_UNEXPECTED_VARIANT: variant checked against non-variant type {expected:?}"),
            TypeError::UnexpectedList { expected } =>
                                write!(f, "ERROR_UNEXPECTED_LIST: list checked against non-list type {expected:?}"),
            TypeError::UnexpectedInjection { expected } =>
                                write!(f, "ERROR_UNEXPECTED_INJECTION: injection checked against non-sum type {expected:?}"),
            TypeError::MissingRecordFields { missing } =>
                                write!(f, "ERROR_MISSING_RECORD_FIELDS: missing fields: {}", missing.join(", ")),
            TypeError::UnexpectedRecordFields { unexpected } =>
                                write!(f, "ERROR_UNEXPECTED_RECORD_FIELDS: unexpected fields: {}", unexpected.join(", ")),
            TypeError::UnexpectedFieldAccess { field, record_type } =>
                                write!(f, "ERROR_UNEXPECTED_FIELD_ACCESS: field `{field}` not found in {record_type:?}"),
            TypeError::UnexpectedVariantLabel { label, variant_type } =>
                                write!(f, "ERROR_UNEXPECTED_VARIANT_LABEL: label `{label}` not found in {variant_type:?}"),
            TypeError::TupleIndexOutOfBounds { index, length } =>
                                write!(f, "ERROR_TUPLE_INDEX_OUT_OF_BOUNDS: index {index} in tuple of length {length}"),
            TypeError::UnexpectedTupleLength { expected, got } =>
                                write!(f, "ERROR_UNEXPECTED_TUPLE_LENGTH: expected {expected} elements, got {got}"),
            TypeError::AmbiguousSumType =>
                                write!(f, "ERROR_AMBIGUOUS_SUM_TYPE: cannot determine sum type for injection"),
            TypeError::AmbiguousVariantType =>
                                write!(f, "ERROR_AMBIGUOUS_VARIANT_TYPE: cannot determine variant type"),
            TypeError::AmbiguousList =>
                                write!(f, "ERROR_AMBIGUOUS_LIST: cannot determine list element type"),
            TypeError::IllegalEmptyMatching =>
                                write!(f, "ERROR_ILLEGAL_EMPTY_MATCHING: match expression has no cases"),
            TypeError::NonexhaustiveMatchPatterns { missing } =>
                                write!(f, "ERROR_NONEXHAUSTIVE_MATCH_PATTERNS: missing patterns: {}", missing.join(", ")),
            TypeError::UnexpectedPatternForType { pattern_desc, scrutinee_type } =>
                                write!(f, "ERROR_UNEXPECTED_PATTERN_FOR_TYPE: pattern `{pattern_desc}` does not match type {scrutinee_type:?}"),
            TypeError::DuplicateRecordFields { field } =>
                                write!(f, "ERROR_DUPLICATE_RECORD_FIELDS: duplicate field `{field}` in record"),
            TypeError::DuplicateRecordTypeFields { field } =>
                                write!(f, "ERROR_DUPLICATE_RECORD_TYPE_FIELDS: duplicate field `{field}` in record type"),
            TypeError::DuplicateVariantTypeFields { label } =>
                                write!(f, "ERROR_DUPLICATE_VARIANT_TYPE_FIELDS: duplicate label `{label}` in variant type"),
            TypeError::DuplicateFunctionDeclaration { name } =>
                                write!(f, "ERROR_DUPLICATE_FUNCTION_DECLARATION: function `{name}` declared more than once"),
            TypeError::IncorrectArityOfMain { got } =>
                                write!(f, "ERROR_INCORRECT_ARITY_OF_MAIN: function `main` must have exactly 1 parameter, got {got}"),
            TypeError::IncorrectNumberOfArguments { expected, got } =>
                                write!(f, "ERROR_INCORRECT_NUMBER_OF_ARGUMENTS: expected {expected} arguments, got {got}"),
            TypeError::UnexpectedNumberOfParametersInLambda { expected, got } =>
                                write!(f, "ERROR_UNEXPECTED_NUMBER_OF_PARAMETERS_IN_LAMBDA: expected {expected} parameters, got {got}"),
            TypeError::DuplicateRecordPatternFields { field } =>
                                write!(f, "ERROR_DUPLICATE_RECORD_PATTERN_FIELDS: duplicate field `{field}` in record pattern"),
            TypeError::UnexpectedDataForNullaryLabel { label } =>
                                write!(f, "ERROR_UNEXPECTED_DATA_FOR_NULLARY_LABEL: label `{label}` is nullary and must not carry data"),
            TypeError::MissingDataForLabel { label } =>
                                write!(f, "ERROR_MISSING_DATA_FOR_LABEL: label `{label}` requires payload data"),
            TypeError::UnexpectedNonNullaryVariantPattern { label } =>
                                write!(f, "ERROR_UNEXPECTED_NON_NULLARY_VARIANT_PATTERN: label `{label}` is nullary but pattern expects payload"),
            TypeError::UnexpectedNullaryVariantPattern { label } =>
                                write!(f, "ERROR_UNEXPECTED_NULLARY_VARIANT_PATTERN: label `{label}` requires payload in pattern"),
            TypeError::DuplicateFunctionParameter { name } =>
                                write!(f, "ERROR_DUPLICATE_FUNCTION_PARAMETER: duplicate parameter `{name}` in function declaration"),
            TypeError::DuplicateLetBinding { name } =>
                                write!(f, "ERROR_DUPLICATE_LET_BINDING: duplicate let binding `{name}`"),
            TypeError::DuplicateTypeParameter { name } =>
                                write!(f, "ERROR_DUPLICATE_TYPE_PARAMETER: duplicate type parameter `{name}`"),
            TypeError::AmbiguousTuple =>
                                write!(f, "ERROR_AMBIGUOUS_TUPLE: cannot determine tuple element types"),
            TypeError::AmbiguousFunction => write!(f, "ERROR_AMBIGUOUS_FUNCTION: cannot determine function type"),
        }
    }
}
