use crate::ast::{Expr, Type};

use std::fmt;

#[derive(Debug, Clone)]
pub struct TypeCheckError {
    pub error: TypeError,
    pub in_function: Option<String>,
}

impl fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)?;
        if let Some(func) = &self.in_function {
            write!(f, "\nin function `{}`", func)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum TypeError {
    MissingMain,

    UndefinedVariable(String),

    UnexpectedTypeForExpression {
        expected: Type,
        got: Type,
        expr: Option<Box<Expr>>,
    },

    NotAFunction {
        ty: Type,
        expr: Box<Expr>,
    },

    NotATuple {
        ty: Type,
        expr: Box<Expr>,
    },

    NotARecord {
        ty: Type,
        expr: Box<Expr>,
    },

    NotAList {
        ty: Type,
        expr: Box<Expr>,
    },

    UnexpectedLambda {
        expected: Type,
        expr: Box<Expr>,
    },

    UnexpectedTypeForParameter {
        param: String,
        expected: Type,
        got: Type,
    },

    UnexpectedTuple {
        expected: Type,
        expr: Box<Expr>,
    },

    UnexpectedRecord {
        expected: Type,
        expr: Box<Expr>,
    },

    UnexpectedVariant {
        expected: Type,
        expr: Box<Expr>,
    },

    UnexpectedList {
        expected: Type,
        expr: Box<Expr>,
    },

    UnexpectedInjection {
        expected: Type,
        expr: Box<Expr>,
    },

    MissingRecordFields {
        missing: Vec<String>,
        expr: Box<Expr>,
    },

    UnexpectedRecordFields {
        unexpected: Vec<String>,
        expr: Box<Expr>,
    },

    UnexpectedFieldAccess {
        field: String,
        record_type: Type,
        expr: Option<Box<Expr>>,
    },

    UnexpectedVariantLabel {
        label: String,
        variant_type: Type,
        expr: Option<Box<Expr>>,
    },

    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
        expr: Box<Expr>,
    },

    UnexpectedTupleLength {
        expected: usize,
        got: usize,
    },

    AmbiguousSumType {
        expr: Box<Expr>,
    },

    AmbiguousVariantType {
        expr: Box<Expr>,
    },

    AmbiguousList {
        expr: Box<Expr>,
    },
    AmbiguousTuple {
        expr: Box<Expr>,
    },
    AmbiguousFunction {
        expr: Box<Expr>,
    },

    IllegalEmptyMatching {
        expr: Box<Expr>,
    },

    NonexhaustiveMatchPatterns {
        missing: Vec<String>,
        expr: Box<Expr>,
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
        expr: Box<Expr>,
    },

    NonexhaustiveLetRecPatterns {
        missing: Vec<String>,
        expr: Box<Expr>,
    },
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::MissingMain =>
                write!(f, "ERROR_MISSING_MAIN:\n  no `main` function declared in the program"),
            TypeError::UndefinedVariable(name) =>
                write!(f, "ERROR_UNDEFINED_VARIABLE:\n  variable `{name}` is not defined"),
            TypeError::UnexpectedTypeForExpression { expected, got, expr } => {
                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_EXPRESSION:\n  expected: {expected}\n  actual:   {got}")?;
                if let Some(e) = expr {
                    write!(f, "\n  for:\n    {e}")?;
                }
                Ok(())
            }
            TypeError::NotAFunction { ty, expr } =>
                write!(f, "ERROR_NOT_A_FUNCTION:\n  expected a function type, but got: {ty}\n  for:\n    {expr}"),
            TypeError::NotATuple { ty, expr } =>
                write!(f, "ERROR_NOT_A_TUPLE:\n  expected a tuple type, but got: {ty}\n  for:\n    {expr}"),
            TypeError::NotARecord { ty, expr } =>
                write!(f, "ERROR_NOT_A_RECORD:\n  expected a record type, but got: {ty}\n  for:\n    {expr}"),
            TypeError::NotAList { ty, expr } =>
                write!(f, "ERROR_NOT_A_LIST:\n  expected a list type, but got: {ty}\n  for:\n    {expr}"),
            TypeError::UnexpectedLambda { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_LAMBDA:\n  checked against non-function type: {expected}\n  for:\n    {expr}"),
            TypeError::UnexpectedTypeForParameter { param, expected, got } =>
                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_PARAMETER:\n  parameter: `{param}`\n  declared:  {got}\n  expected:  {expected}"),
            TypeError::UnexpectedTuple { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_TUPLE:\n  tuple literal checked against non-tuple type: {expected}\n  for:\n    {expr}"),
            TypeError::UnexpectedRecord { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_RECORD:\n  record literal checked against non-record type: {expected}\n  for:\n    {expr}"),
            TypeError::UnexpectedVariant { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_VARIANT:\n  variant expression checked against non-variant type: {expected}\n  for:\n    {expr}"),
            TypeError::UnexpectedList { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_LIST:\n  list expression checked against non-list type: {expected}\n  for:\n    {expr}"),
            TypeError::UnexpectedInjection { expected, expr } =>
                write!(f, "ERROR_UNEXPECTED_INJECTION:\n  injection (inl/inr) checked against non-sum type: {expected}\n  for:\n    {expr}"),
            TypeError::MissingRecordFields { missing, expr } =>
                write!(f, "ERROR_MISSING_RECORD_FIELDS:\n  missing: {}\n  for:\n    {expr}", missing.join(", ")),
            TypeError::UnexpectedRecordFields { unexpected, expr } =>
                write!(f, "ERROR_UNEXPECTED_RECORD_FIELDS:\n  unexpected: {}\n  for:\n    {expr}", unexpected.join(", ")),
            TypeError::UnexpectedFieldAccess { field, record_type, expr } => {
                write!(f, "ERROR_UNEXPECTED_FIELD_ACCESS:\n  field:       `{field}`\n  record type: {record_type}")?;
                if let Some(e) = expr {
                    write!(f, "\n  for:\n    {e}")?;
                }
                Ok(())
            }
            TypeError::UnexpectedVariantLabel { label, variant_type, expr } => {
                write!(f, "ERROR_UNEXPECTED_VARIANT_LABEL:\n  label:        `{label}`\n  variant type: {variant_type}")?;
                if let Some(e) = expr {
                    write!(f, "\n  for:\n    {e}")?;
                }
                Ok(())
            }
            TypeError::TupleIndexOutOfBounds { index, length, expr } =>
                write!(f, "ERROR_TUPLE_INDEX_OUT_OF_BOUNDS:\n  index:  {index}\n  length: {length}\n  for:\n    {expr}"),
            TypeError::UnexpectedTupleLength { expected, got } =>
                write!(f, "ERROR_UNEXPECTED_TUPLE_LENGTH:\n  expected: {expected} element(s)\n  got:      {got} element(s)"),
            TypeError::AmbiguousSumType { expr } =>
                write!(f, "ERROR_AMBIGUOUS_SUM_TYPE:\n  cannot determine sum type (type annotation required)\n  for:\n    {expr}"),
            TypeError::AmbiguousVariantType { expr } =>
                write!(f, "ERROR_AMBIGUOUS_VARIANT_TYPE:\n  cannot determine variant type (type annotation required)\n  for:\n    {expr}"),
            TypeError::AmbiguousList { expr } =>
                write!(f, "ERROR_AMBIGUOUS_LIST:\n  cannot determine list element type (type annotation required)\n  for:\n    {expr}"),
            TypeError::IllegalEmptyMatching { expr } =>
                write!(f, "ERROR_ILLEGAL_EMPTY_MATCHING:\n  match expression must have at least one case\n  for:\n    {expr}"),
            TypeError::NonexhaustiveMatchPatterns { missing, expr } =>
                write!(f, "ERROR_NONEXHAUSTIVE_MATCH_PATTERNS:\n  missing cases: {}\n  for:\n    {expr}", missing.join(", ")),
            TypeError::UnexpectedPatternForType { pattern_desc, scrutinee_type } =>
                write!(f, "ERROR_UNEXPECTED_PATTERN_FOR_TYPE:\n  pattern:        `{pattern_desc}`\n  scrutinee type: {scrutinee_type}"),
            TypeError::DuplicateRecordFields { field } =>
                write!(f, "ERROR_DUPLICATE_RECORD_FIELDS:\n  field `{field}` appears more than once in the record literal"),
            TypeError::DuplicateRecordTypeFields { field } =>
                write!(f, "ERROR_DUPLICATE_RECORD_TYPE_FIELDS:\n  field `{field}` appears more than once in the record type"),
            TypeError::DuplicateVariantTypeFields { label } =>
                write!(f, "ERROR_DUPLICATE_VARIANT_TYPE_FIELDS:\n  label `{label}` appears more than once in the variant type"),
            TypeError::DuplicateFunctionDeclaration { name } =>
                write!(f, "ERROR_DUPLICATE_FUNCTION_DECLARATION:\n  function `{name}` is declared more than once"),
            TypeError::IncorrectArityOfMain { got } =>
                write!(f, "ERROR_INCORRECT_ARITY_OF_MAIN:\n  `main` must have exactly 1 parameter, but has {got}"),
            TypeError::IncorrectNumberOfArguments { expected, got } =>
                write!(f, "ERROR_INCORRECT_NUMBER_OF_ARGUMENTS:\n  expected: {expected} argument(s)\n  got:      {got}"),
            TypeError::UnexpectedNumberOfParametersInLambda { expected, got } =>
                write!(f, "ERROR_UNEXPECTED_NUMBER_OF_PARAMETERS_IN_LAMBDA:\n  expected: {expected} parameter(s)\n  got:      {got}"),
            TypeError::DuplicateRecordPatternFields { field } =>
                write!(f, "ERROR_DUPLICATE_RECORD_PATTERN_FIELDS:\n  field `{field}` appears more than once in the record pattern"),
            TypeError::UnexpectedDataForNullaryLabel { label } =>
                write!(f, "ERROR_UNEXPECTED_DATA_FOR_NULLARY_LABEL:\n  label `{label}` is nullary (carries no data) but a payload was provided"),
            TypeError::MissingDataForLabel { label } =>
                write!(f, "ERROR_MISSING_DATA_FOR_LABEL:\n  label `{label}` requires a payload but none was provided"),
            TypeError::UnexpectedNonNullaryVariantPattern { label } =>
                write!(f, "ERROR_UNEXPECTED_NON_NULLARY_VARIANT_PATTERN:\n  pattern for label `{label}` has a data binding but the label is nullary"),
            TypeError::UnexpectedNullaryVariantPattern { label } =>
                write!(f, "ERROR_UNEXPECTED_NULLARY_VARIANT_PATTERN:\n  pattern for label `{label}` has no data binding but the label carries a payload"),
            TypeError::DuplicateFunctionParameter { name } =>
                write!(f, "ERROR_DUPLICATE_FUNCTION_PARAMETER:\n  parameter `{name}` appears more than once"),
            TypeError::DuplicateLetBinding { name } =>
                write!(f, "ERROR_DUPLICATE_LET_BINDING:\n  binding `{name}` appears more than once in the let-expression"),
            TypeError::DuplicateTypeParameter { name } =>
                write!(f, "ERROR_DUPLICATE_TYPE_PARAMETER:\n  type parameter `{name}` appears more than once"),
            TypeError::AmbiguousTuple { expr } =>
                write!(f, "ERROR_AMBIGUOUS_TUPLE:\n  cannot determine tuple element types (type annotation required)\n  for:\n    {expr}"),
            TypeError::AmbiguousFunction { expr } =>
                write!(f, "ERROR_AMBIGUOUS_FUNCTION:\n  cannot determine function type (type annotation required)\n  for:\n    {expr}"),
            TypeError::NonexhaustiveLetPatterns { missing, expr } =>
                write!(f, "ERROR_NONEXHAUSTIVE_LET_PATTERNS:\n  missing cases: {}\n  for:\n    {expr}", missing.join(", ")),
            TypeError::NonexhaustiveLetRecPatterns { missing, expr } =>
                write!(f, "ERROR_NONEXHAUSTIVE_LET_REC_PATTERNS:\n  missing cases: {}\n  for:\n    {expr}", missing.join(", ")),
        }
    }
}
