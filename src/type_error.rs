use crate::ast::{Span, Type};

use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct TypeCheckError {
    pub error: TypeError,
    pub in_function: Option<String>,
    pub src: Rc<str>,
}

impl fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)?;
        if let Some(span) = self.error.primary_span() {
            write_source_excerpt(f, &self.src, span)?;
        }
        if let Some(func) = &self.in_function {
            write!(f, "\nin function `{}`", func)?;
        }
        Ok(())
    }
}

fn write_source_excerpt(f: &mut fmt::Formatter<'_>, src: &str, span: Span) -> fmt::Result {
    const RED_BOLD: &str = "\x1b[1;31m";
    const RESET: &str = "\x1b[0m";

    let start = span.start.min(src.len());
    let end = span.end.min(src.len()).max(start);
    let lines = source_lines(src);
    let line = find_line_index(&lines, start) + 1;
    let col = start - lines[line - 1].0 + 1;

    if lines.is_empty() {
        return Ok(());
    }

    let start_line_idx = find_line_index(&lines, start);
    let end_lookup = end.saturating_sub(1);
    let end_line_idx = find_line_index(&lines, end_lookup.min(src.len().saturating_sub(1)));
    let context_start = start_line_idx.saturating_sub(1);
    let context_end = (end_line_idx + 1).min(lines.len() - 1);
    let line_no_width = (context_end + 1).to_string().len();

    write!(f, "\n  --> [{line}:{col}]")?;
    for (idx, &(line_start, line_end)) in lines
        .iter()
        .enumerate()
        .skip(context_start)
        .take(context_end - context_start + 1)
    {
        let line_number = idx + 1;
        let line = &src[line_start..line_end];
        let line = line.strip_suffix('\n').unwrap_or(line);
        write!(f, "\n  {line_number:>line_no_width$} | {line}")?;

        let highlight_start = start.max(line_start);
        let highlight_end = end.min(line_end);
        if highlight_start < highlight_end
            || (start == end && start >= line_start && start <= line_end)
        {
            let underline_offset = src[line_start..highlight_start].chars().count();
            let underline_len = if highlight_start < highlight_end {
                src[highlight_start..highlight_end].chars().count().max(1)
            } else {
                1
            };
            let underline = " ".repeat(underline_offset) + &"^".repeat(underline_len);
            write!(
                f,
                "\n  {:>line_no_width$} | {RED_BOLD}{underline}{RESET}",
                ""
            )?;
        }
    }

    Ok(())
}

fn source_lines(src: &str) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut line_start = 0;

    for (idx, ch) in src.char_indices() {
        if ch == '\n' {
            lines.push((line_start, idx + ch.len_utf8()));
            line_start = idx + ch.len_utf8();
        }
    }

    if line_start < src.len() || src.is_empty() {
        lines.push((line_start, src.len()));
    }

    lines
}

fn find_line_index(lines: &[(usize, usize)], offset: usize) -> usize {
    lines
        .iter()
        .position(|&(line_start, line_end)| {
            offset >= line_start
                && (offset < line_end || (offset == line_end && offset == line_start))
        })
        .unwrap_or(lines.len().saturating_sub(1))
}

#[derive(Debug, Clone)]
pub enum TypeError {
    MissingMain,

    UndefinedVariable {
        name: String,
        expr_span: Span,
    },

    UnexpectedTypeForExpression {
        expected: Type,
        got: Type,
        expr_span: Option<Span>,
    },

    NotAFunction {
        ty: Type,
        expr_span: Span,
    },

    NotATuple {
        ty: Type,
        expr_span: Span,
    },

    NotARecord {
        ty: Type,
        expr_span: Span,
    },

    NotAList {
        ty: Type,
        expr_span: Span,
    },

    UnexpectedLambda {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedTypeForParameter {
        param: String,
        expected: Type,
        got: Type,
        expr_span: Span,
    },

    UnexpectedTuple {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedRecord {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedVariant {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedList {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedInjection {
        expected: Type,
        expr_span: Span,
    },

    MissingRecordFields {
        missing: Vec<String>,
        expr_span: Span,
    },

    UnexpectedRecordFields {
        unexpected: Vec<String>,
        expr_span: Span,
    },

    UnexpectedFieldAccess {
        field: String,
        record_type: Type,
        expr_span: Option<Span>,
    },

    UnexpectedVariantLabel {
        label: String,
        variant_type: Type,
        expr_span: Option<Span>,
    },

    TupleIndexOutOfBounds {
        index: usize,
        length: usize,
        expr_span: Span,
    },

    UnexpectedTupleLength {
        expected: usize,
        got: usize,
        expr_span: Span,
    },

    AmbiguousSumType {
        expr_span: Span,
    },

    AmbiguousVariantType {
        expr_span: Span,
    },

    AmbiguousList {
        expr_span: Span,
    },
    AmbiguousTuple {
        expr_span: Span,
    },
    AmbiguousFunction {
        expr_span: Span,
    },

    IllegalEmptyMatching {
        expr_span: Span,
    },

    NonexhaustiveMatchPatterns {
        missing: Vec<String>,
        expr_span: Span,
    },

    UnexpectedPatternForType {
        pattern_desc: String,
        scrutinee_type: Type,
        pat_span: Span,
    },

    DuplicateRecordFields {
        field: String,
        expr_span: Span,
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
        expr_span: Span,
    },

    UnexpectedNumberOfParametersInLambda {
        expected: usize,
        got: usize,
        expr_span: Span,
    },

    DuplicateRecordPatternFields {
        field: String,
        pat_span: Span,
    },

    UnexpectedDataForNullaryLabel {
        label: String,
        expr_span: Span,
    },

    MissingDataForLabel {
        label: String,
        expr_span: Span,
    },

    UnexpectedNonNullaryVariantPattern {
        label: String,
        pat_span: Span,
    },

    UnexpectedNullaryVariantPattern {
        label: String,
        pat_span: Span,
    },

    DuplicateFunctionParameter {
        name: String,
    },

    DuplicateLetBinding {
        name: String,
        expr_span: Span,
    },

    DuplicateTypeParameter {
        name: String,
    },

    NonexhaustiveLetPatterns {
        missing: Vec<String>,
        expr_span: Span,
    },

    NonexhaustiveLetRecPatterns {
        missing: Vec<String>,
        expr_span: Span,
    },

    AmbiguousPatternType {
        pat_span: Span,
    },

    ExceptionTypeNotDeclared {
        expr_span: Span,
    },

    AmbiguousThrowType {
        expr_span: Span,
    },

    AmbiguousReferenceType {
        expr_span: Span,
    },

    AmbiguousPanicType {
        expr_span: Span,
    },

    NotAReference {
        ty: Type,
        expr_span: Span,
    },

    UnexpectedMemoryAddress {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedReference {
        expected: Type,
        expr_span: Span,
    },

    UnexpectedSubtype {
        expected: Type,
        got: Type,
        expr_span: Option<Span>,
    },

    DuplicateExceptionType,

    DuplicateExceptionVariant {
        label: String,
    },

    ConflictingExceptionDeclarations,

    IllegalLocalExceptionType,

    IllegalLocalOpenVariantException,
}

impl TypeError {
    pub fn primary_span(&self) -> Option<Span> {
        match self {
            TypeError::UnexpectedTypeForExpression { expr_span, .. } => *expr_span,
            TypeError::NotAFunction { expr_span, .. } => Some(*expr_span),
            TypeError::NotATuple { expr_span, .. } => Some(*expr_span),
            TypeError::NotARecord { expr_span, .. } => Some(*expr_span),
            TypeError::NotAList { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedLambda { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedTuple { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedRecord { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedVariant { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedList { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedInjection { expr_span, .. } => Some(*expr_span),
            TypeError::MissingRecordFields { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedRecordFields { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedFieldAccess { expr_span, .. } => *expr_span,
            TypeError::UnexpectedVariantLabel { expr_span, .. } => *expr_span,
            TypeError::TupleIndexOutOfBounds { expr_span, .. } => Some(*expr_span),
            TypeError::AmbiguousSumType { expr_span } => Some(*expr_span),
            TypeError::AmbiguousVariantType { expr_span } => Some(*expr_span),
            TypeError::AmbiguousList { expr_span } => Some(*expr_span),
            TypeError::AmbiguousTuple { expr_span } => Some(*expr_span),
            TypeError::AmbiguousFunction { expr_span } => Some(*expr_span),
            TypeError::IllegalEmptyMatching { expr_span } => Some(*expr_span),
            TypeError::NonexhaustiveMatchPatterns { expr_span, .. } => Some(*expr_span),
            TypeError::NonexhaustiveLetPatterns { expr_span, .. } => Some(*expr_span),
            TypeError::NonexhaustiveLetRecPatterns { expr_span, .. } => Some(*expr_span),
            TypeError::AmbiguousPatternType { pat_span } => Some(*pat_span),
            TypeError::UndefinedVariable { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedTypeForParameter { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedTupleLength { expr_span, .. } => Some(*expr_span),
            TypeError::IncorrectNumberOfArguments { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedNumberOfParametersInLambda { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedDataForNullaryLabel { expr_span, .. } => Some(*expr_span),
            TypeError::MissingDataForLabel { expr_span, .. } => Some(*expr_span),
            TypeError::DuplicateLetBinding { expr_span, .. } => Some(*expr_span),
            TypeError::DuplicateRecordFields { expr_span, .. } => Some(*expr_span),
            TypeError::ExceptionTypeNotDeclared { expr_span } => Some(*expr_span),
            TypeError::AmbiguousThrowType { expr_span } => Some(*expr_span),
            TypeError::AmbiguousReferenceType { expr_span } => Some(*expr_span),
            TypeError::AmbiguousPanicType { expr_span } => Some(*expr_span),
            TypeError::NotAReference { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedMemoryAddress { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedReference { expr_span, .. } => Some(*expr_span),
            TypeError::UnexpectedSubtype { expr_span, .. } => *expr_span,
            _ => None,
        }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::MissingMain =>
                write!(f, "ERROR_MISSING_MAIN:\n  no `main` function declared in the program"),
            TypeError::UndefinedVariable { name, .. } =>
                write!(f, "ERROR_UNDEFINED_VARIABLE:\n  variable `{name}` is not defined"),
            TypeError::UnexpectedTypeForExpression { expected, got, .. } =>
                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_EXPRESSION:\n  expected: {expected}\n  actual:   {got}"),
            TypeError::NotAFunction { ty, .. } =>
                write!(f, "ERROR_NOT_A_FUNCTION:\n  expected a function type, but got: {ty}"),
            TypeError::NotATuple { ty, .. } =>
                write!(f, "ERROR_NOT_A_TUPLE:\n  expected a tuple type, but got: {ty}"),
            TypeError::NotARecord { ty, .. } =>
                write!(f, "ERROR_NOT_A_RECORD:\n  expected a record type, but got: {ty}"),
            TypeError::NotAList { ty, .. } =>
                write!(f, "ERROR_NOT_A_LIST:\n  expected a list type, but got: {ty}"),
            TypeError::UnexpectedLambda { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_LAMBDA:\n  checked against non-function type: {expected}"),
            TypeError::UnexpectedTypeForParameter { param, expected, got, .. } =>
                write!(f, "ERROR_UNEXPECTED_TYPE_FOR_PARAMETER:\n  parameter: `{param}`\n  declared:  {got}\n  expected:  {expected}"),
            TypeError::UnexpectedTuple { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_TUPLE:\n  tuple literal checked against non-tuple type: {expected}"),
            TypeError::UnexpectedRecord { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_RECORD:\n  record literal checked against non-record type: {expected}"),
            TypeError::UnexpectedVariant { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_VARIANT:\n  variant expression checked against non-variant type: {expected}"),
            TypeError::UnexpectedList { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_LIST:\n  list expression checked against non-list type: {expected}"),
            TypeError::UnexpectedInjection { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_INJECTION:\n  injection (inl/inr) checked against non-sum type: {expected}"),
            TypeError::MissingRecordFields { missing, .. } =>
                write!(f, "ERROR_MISSING_RECORD_FIELDS:\n  missing: {}", missing.join(", ")),
            TypeError::UnexpectedRecordFields { unexpected, .. } =>
                write!(f, "ERROR_UNEXPECTED_RECORD_FIELDS:\n  unexpected: {}", unexpected.join(", ")),
            TypeError::UnexpectedFieldAccess { field, record_type, .. } =>
                write!(f, "ERROR_UNEXPECTED_FIELD_ACCESS:\n  field:       `{field}`\n  record type: {record_type}"),
            TypeError::UnexpectedVariantLabel { label, variant_type, .. } =>
                write!(f, "ERROR_UNEXPECTED_VARIANT_LABEL:\n  label:        `{label}`\n  variant type: {variant_type}"),
            TypeError::TupleIndexOutOfBounds { index, length, .. } =>
                write!(f, "ERROR_TUPLE_INDEX_OUT_OF_BOUNDS:\n  index:  {index}\n  length: {length}"),
            TypeError::UnexpectedTupleLength { expected, got, .. } =>
                write!(f, "ERROR_UNEXPECTED_TUPLE_LENGTH:\n  expected: {expected} element(s)\n  got:      {got} element(s)"),
            TypeError::AmbiguousSumType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_SUM_TYPE:\n  cannot determine sum type (type annotation required)"),
            TypeError::AmbiguousVariantType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_VARIANT_TYPE:\n  cannot determine variant type (type annotation required)"),
            TypeError::AmbiguousList { .. } =>
                write!(f, "ERROR_AMBIGUOUS_LIST_TYPE:\n  cannot determine list element type (type annotation required)"),
            TypeError::IllegalEmptyMatching { .. } =>
                write!(f, "ERROR_ILLEGAL_EMPTY_MATCHING:\n  match expression must have at least one case"),
            TypeError::NonexhaustiveMatchPatterns { missing, .. } =>
                write!(f, "ERROR_NONEXHAUSTIVE_MATCH_PATTERNS:\n  missing cases: {}", missing.join(", ")),
            TypeError::UnexpectedPatternForType { pattern_desc, scrutinee_type, .. } =>
                write!(f, "ERROR_UNEXPECTED_PATTERN_FOR_TYPE:\n  pattern:        `{pattern_desc}`\n  scrutinee type: {scrutinee_type}"),
            TypeError::DuplicateRecordFields { field, .. } =>
                write!(f, "ERROR_DUPLICATE_RECORD_FIELDS:\n  field `{field}` appears more than once in the record literal"),
            TypeError::DuplicateRecordTypeFields { field } =>
                write!(f, "ERROR_DUPLICATE_RECORD_TYPE_FIELDS:\n  field `{field}` appears more than once in the record type"),
            TypeError::DuplicateVariantTypeFields { label } =>
                write!(f, "ERROR_DUPLICATE_VARIANT_TYPE_FIELDS:\n  label `{label}` appears more than once in the variant type"),
            TypeError::DuplicateFunctionDeclaration { name } =>
                write!(f, "ERROR_DUPLICATE_FUNCTION_DECLARATION:\n  function `{name}` is declared more than once"),
            TypeError::IncorrectArityOfMain { got } =>
                write!(f, "ERROR_INCORRECT_ARITY_OF_MAIN:\n  `main` must have exactly 1 parameter, but has {got}"),
            TypeError::IncorrectNumberOfArguments { expected, got, .. } =>
                write!(f, "ERROR_INCORRECT_NUMBER_OF_ARGUMENTS:\n  expected: {expected} argument(s)\n  got:      {got}"),
            TypeError::UnexpectedNumberOfParametersInLambda { expected, got, .. } =>
                write!(f, "ERROR_UNEXPECTED_NUMBER_OF_PARAMETERS_IN_LAMBDA:\n  expected: {expected} parameter(s)\n  got:      {got}"),
            TypeError::DuplicateRecordPatternFields { field, .. } =>
                write!(f, "ERROR_DUPLICATE_RECORD_PATTERN_FIELDS:\n  field `{field}` appears more than once in the record pattern"),
            TypeError::UnexpectedDataForNullaryLabel { label, .. } =>
                write!(f, "ERROR_UNEXPECTED_DATA_FOR_NULLARY_LABEL:\n  label `{label}` is nullary (carries no data) but a payload was provided"),
            TypeError::MissingDataForLabel { label, .. } =>
                write!(f, "ERROR_MISSING_DATA_FOR_LABEL:\n  label `{label}` requires a payload but none was provided"),
            TypeError::UnexpectedNonNullaryVariantPattern { label, .. } =>
                write!(f, "ERROR_UNEXPECTED_NON_NULLARY_VARIANT_PATTERN:\n  pattern for label `{label}` has a data binding but the label is nullary"),
            TypeError::UnexpectedNullaryVariantPattern { label, .. } =>
                write!(f, "ERROR_UNEXPECTED_NULLARY_VARIANT_PATTERN:\n  pattern for label `{label}` has no data binding but the label carries a payload"),
            TypeError::DuplicateFunctionParameter { name } =>
                write!(f, "ERROR_DUPLICATE_FUNCTION_PARAMETER:\n  parameter `{name}` appears more than once"),
            TypeError::DuplicateLetBinding { name, .. } =>
                write!(f, "ERROR_DUPLICATE_LET_BINDING:\n  binding `{name}` appears more than once in the let-expression"),
            TypeError::DuplicateTypeParameter { name } =>
                write!(f, "ERROR_DUPLICATE_TYPE_PARAMETER:\n  type parameter `{name}` appears more than once"),
            TypeError::AmbiguousTuple { .. } =>
                write!(f, "ERROR_AMBIGUOUS_TUPLE:\n  cannot determine tuple element types (type annotation required)"),
            TypeError::AmbiguousFunction { .. } =>
                write!(f, "ERROR_AMBIGUOUS_FUNCTION:\n  cannot determine function type (type annotation required)"),
            TypeError::NonexhaustiveLetPatterns { missing, .. } =>
                write!(f, "ERROR_NONEXHAUSTIVE_LET_PATTERNS:\n  missing cases: {}", missing.join(", ")),
            TypeError::NonexhaustiveLetRecPatterns { missing, .. } =>
                write!(f, "ERROR_NONEXHAUSTIVE_LET_REC_PATTERNS:\n  missing cases: {}", missing.join(", ")),
            TypeError::AmbiguousPatternType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_PATTERN_TYPE:\n  cannot infer the type for pattern"),
            TypeError::ExceptionTypeNotDeclared { .. } =>
                write!(f, "ERROR_EXCEPTION_TYPE_NOT_DECLARED:\n  exceptions used without a declared exception type"),
            TypeError::AmbiguousThrowType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_THROW_TYPE:\n  cannot determine the type of the thrown value (annotation required)"),
            TypeError::AmbiguousReferenceType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_REFERENCE_TYPE:\n  cannot determine the type of this memory address (annotation required)"),
            TypeError::AmbiguousPanicType { .. } =>
                write!(f, "ERROR_AMBIGUOUS_PANIC_TYPE:\n  cannot determine the type of `panic!` (annotation required)"),
            TypeError::NotAReference { ty, .. } =>
                write!(f, "ERROR_NOT_A_REFERENCE:\n  expected a reference type, but got: {ty}"),
            TypeError::UnexpectedMemoryAddress { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_MEMORY_ADDRESS:\n  memory address cannot have non-reference type: {expected}"),
            TypeError::UnexpectedReference { expected, .. } =>
                write!(f, "ERROR_UNEXPECTED_REFERENCE:\n  `new(...)` cannot have non-reference type: {expected}"),
            TypeError::UnexpectedSubtype { expected, got, .. } =>
                write!(f, "ERROR_UNEXPECTED_SUBTYPE:\n  expected a subtype of type\n{expected}\nbut got type\n{got}"),
            TypeError::DuplicateExceptionType =>
                write!(f, "ERROR_DUPLICATE_EXCEPTION_TYPE:\n  more than one `exception type` declaration in the same scope"),
            TypeError::DuplicateExceptionVariant { label } =>
                write!(f, "ERROR_DUPLICATE_EXCEPTION_VARIANT:\n  variant label `{label}` appears more than once in exception variant declarations"),
            TypeError::ConflictingExceptionDeclarations =>
                write!(f, "ERROR_CONFLICTING_EXCEPTION_DECLARATIONS:\n  both `exception type` and `exception variant` declarations are present; they cannot be mixed"),
            TypeError::IllegalLocalExceptionType =>
                write!(f, "ERROR_ILLEGAL_LOCAL_EXCEPTION_TYPE:\n  `exception type` declaration appears in a local scope; it must be at the top level"),
            TypeError::IllegalLocalOpenVariantException =>
                write!(f, "ERROR_ILLEGAL_LOCAL_OPEN_VARIANT_EXCEPTION:\n  `exception variant` declaration appears in a local scope; it must be at the top level"),
        }
    }
}
