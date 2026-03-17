use crate::ast::Type;

use std::fmt;

#[derive(Debug, Clone)]
pub enum TypeError {
    MissingMain,

    UndefinedVariable(String),

    UnexpectedTypeForExpression {
        expected: Type,
        got: Type,
    },

    NotAFunction(Type),

    NotATuple(Type),

    NotARecord(Type),

    NotAList(Type),

    UnexpectedLambda {
        expected: Type,
    },

    UnexpectedTypeForParameter {
        param: String,
        expected: Type,
        got: Type,
    },

    UnexpectedTuple {
        expected: Type,
    },

    UnexpectedRecord {
        expected: Type,
    },

    UnexpectedVariant {
        expected: Type,
    },

    UnexpectedList {
        expected: Type,
    },

    UnexpectedInjection {
        expected: Type,
    },

    MissingRecordFields {
        missing: Vec<String>,
    },

    UnexpectedRecordFields {
        unexpected: Vec<String>,
    },

    UnexpectedFieldAccess {
        field: String,
        record_type: Type,
    },

    UnexpectedVariantLabel {
        label: String,
        variant_type: Type,
    },

    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
    },

    UnexpectedTupleLength {
        expected: usize,
        got: usize,
    },

    AmbiguousSumType,

    AmbiguousVariantType,

    AmbiguousList,
    AmbiguousTuple,
    AmbiguousFunction,

    IllegalEmptyMatching,

    NonexhaustiveMatchPatterns {
        missing: Vec<String>,
    },

    UnexpectedPatternForType {
        pattern_desc: String,
        scrutinee_type: Type,
    },

    DuplicateRecordFields {
        field: String,
    },

    DuplicateRecordTypeFields {
        field: String,
    },

    DuplicateVariantTypeFields {
        label: String,
    },

    DuplicateFunctionDeclaration {
        name: String,
    },

    IncorrectArityOfMain {
        got: usize,
    },

    IncorrectNumberOfArguments {
        expected: usize,
        got: usize,
    },

    UnexpectedNumberOfParametersInLambda {
        expected: usize,
        got: usize,
    },

    DuplicateRecordPatternFields {
        field: String,
    },

    UnexpectedDataForNullaryLabel {
        label: String,
    },

    MissingDataForLabel {
        label: String,
    },

    UnexpectedNonNullaryVariantPattern {
        label: String,
    },

    UnexpectedNullaryVariantPattern {
        label: String,
    },

    DuplicateFunctionParameter {
        name: String,
    },

    DuplicateLetBinding {
        name: String,
    },

    DuplicateTypeParameter {
        name: String,
    },

    NonexhaustiveLetPatterns {
        missing: Vec<String>,
    },

    NonexhaustiveLetRecPatterns {
        missing: Vec<String>,
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
            TypeError::NonexhaustiveLetPatterns { missing } =>
                                write!(f, "ERROR_NONEXHAUSTIVE_LET_PATTERNS: missing patterns: {}", missing.join(", ")),
            TypeError::NonexhaustiveLetRecPatterns { missing } =>
                                write!(f, "ERROR_NONEXHAUSTIVE_LET_REC_PATTERNS: missing patterns: {}", missing.join(", ")),
        }
    }
}
