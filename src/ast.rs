#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn start_line_col(&self, src: &str) -> (u32, u32) {
        offset_to_line_col(src, self.start)
    }

    pub fn end_line_col(&self, src: &str) -> (u32, u32) {
        offset_to_line_col(src, self.end)
    }
}

fn offset_to_line_col(src: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(src.len());
    let mut line: u32 = 1;
    let mut col: u32 = 1;
    for (i, ch) in src.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += ch.len_utf8() as u32;
        }
    }
    (line, col)
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(node: T, start: usize, end: usize) -> Self {
        Spanned { node, span: Span::new(start, end) }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub language_decl: LanguageDecl,
    pub extensions: Vec<Extension>,
    pub decls: Vec<Decl>,
}

/// `language core ;`
#[derive(Debug, Clone)]
pub struct LanguageDecl;

/// `extend with #feat1, #feat2 ;`
#[derive(Debug, Clone)]
pub struct Extension {
    pub names: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Decl {
    Fun(DeclFun),
    FunGeneric(DeclFunGeneric),
    TypeAlias { name: String, ty: Type },
    ExceptionType { ty: Type },
    ExceptionVariant { name: String, ty: Type },
}

/// `[annotations] fn name(params) return_type throw_type { local_decls return body }`
#[derive(Debug, Clone)]
pub struct DeclFun {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub return_type: ReturnType,
    pub throw_type: ThrowType,
    pub local_decls: Vec<Decl>,
    pub body: Box<Expr>,
}

/// `[annotations] generic fn name[type_params](params) return_type throw_type { local_decls return body }`
#[derive(Debug, Clone)]
pub struct DeclFunGeneric {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<ParamDecl>,
    pub return_type: ReturnType,
    pub throw_type: ThrowType,
    pub local_decls: Vec<Decl>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub enum Annotation {
    Inline,
}

#[derive(Debug, Clone)]
pub struct ParamDecl {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum ReturnType {
    NoReturn,
    SomeReturn(Box<Type>),
}

impl ReturnType {
    pub fn as_type(&self) -> &Type {
        match self {
            ReturnType::NoReturn => &Type::Unit,
            ReturnType::SomeReturn(ty) => ty.as_ref(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ThrowType {
    NoThrow,
    SomeThrow(Vec<Type>),
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    // --- Expr ---
    /// `e1 ; e2`
    Sequence(Box<Expr>, Box<Expr>),
    /// `let p1 = e1, ... in body`
    Let(Vec<PatternBinding>, Box<Expr>),
    /// `letrec p1 = e1, ... in body`
    LetRec(Vec<PatternBinding>, Box<Expr>),
    /// `generic [T1, ...] body`
    TypeAbstraction(Vec<String>, Box<Expr>),

    // --- Expr1 ---
    /// `lhs := rhs`
    Assign(Box<Expr>, Box<Expr>),
    /// `if cond then then_ else else_`
    If {
        cond: Box<Expr>,
        then_: Box<Expr>,
        else_: Box<Expr>,
    },

    // --- Expr2 ---
    LessThan(Box<Expr>, Box<Expr>),
    LessThanOrEqual(Box<Expr>, Box<Expr>),
    GreaterThan(Box<Expr>, Box<Expr>),
    GreaterThanOrEqual(Box<Expr>, Box<Expr>),
    Equal(Box<Expr>, Box<Expr>),
    NotEqual(Box<Expr>, Box<Expr>),

    // --- Expr3 ---
    /// `e as T`
    TypeAsc(Box<Expr>, Box<Type>),
    /// `e cast as T`
    TypeCast(Box<Expr>, Box<Type>),
    /// `fn(params) { return body }`
    Abstraction {
        params: Vec<ParamDecl>,
        body: Box<Expr>,
    },
    /// `<| label |>` or `<| label = e |>`
    Variant {
        label: String,
        payload: Option<Box<Expr>>,
    },
    /// `match e { p1 => e1 | p2 => e2 ... }`
    Match {
        expr: Box<Expr>,
        cases: Vec<MatchCase>,
    },
    /// `[e1, e2, ...]`
    List(Vec<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Subtract(Box<Expr>, Box<Expr>),
    LogicOr(Box<Expr>, Box<Expr>),

    // --- Expr4 ---
    Multiply(Box<Expr>, Box<Expr>),
    Divide(Box<Expr>, Box<Expr>),
    LogicAnd(Box<Expr>, Box<Expr>),

    // --- Expr5 ---
    /// `new(e)`
    Ref(Box<Expr>),
    /// `*e`
    Deref(Box<Expr>),

    // --- Expr6 ---
    /// `f(args)`
    Application {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `e[T1, T2, ...]`
    TypeApplication {
        expr: Box<Expr>,
        type_args: Vec<Type>,
    },
    /// `e.fieldname`
    DotRecord(Box<Expr>, String),
    /// `e.0`
    DotTuple(Box<Expr>, usize),
    /// `{e1, e2, ...}`
    Tuple(Vec<Expr>),
    /// `{field1 = e1, ...}`
    Record(Vec<Binding>),
    /// `cons(head, tail)`
    ConsList(Box<Expr>, Box<Expr>),
    /// `List::head(e)`
    Head(Box<Expr>),
    /// `List::isempty(e)`
    IsEmpty(Box<Expr>),
    /// `List::tail(e)`
    Tail(Box<Expr>),
    /// `panic!`
    Panic,
    /// `throw(e)`
    Throw(Box<Expr>),
    /// `try { e } catch { p => handler }`
    TryCatch {
        try_expr: Box<Expr>,
        pattern: Box<Pattern>,
        catch_expr: Box<Expr>,
    },
    /// `try { e } with { handler }`
    TryWith {
        try_expr: Box<Expr>,
        with_expr: Box<Expr>,
    },
    /// `try { e } cast as T { p => handler } with { fallback }`
    TryCastAs {
        try_expr: Box<Expr>,
        ty: Box<Type>,
        pattern: Box<Pattern>,
        catch_expr: Box<Expr>,
        with_expr: Box<Expr>,
    },
    /// `inl(e)`
    Inl(Box<Expr>),
    /// `inr(e)`
    Inr(Box<Expr>),
    /// `succ(e)`
    Succ(Box<Expr>),
    /// `not(e)`
    LogicNot(Box<Expr>),
    /// `Nat::pred(e)`
    Pred(Box<Expr>),
    /// `Nat::iszero(e)`
    IsZero(Box<Expr>),
    /// `fix(e)`
    Fix(Box<Expr>),
    /// `Nat::rec(n, z, s)`
    NatRec(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `fold[T] e`
    Fold {
        ty: Box<Type>,
        expr: Box<Expr>,
    },
    /// `unfold[T] e`
    Unfold {
        ty: Box<Type>,
        expr: Box<Expr>,
    },

    // --- Expr7 (atoms) ---
    ConstTrue,
    ConstFalse,
    ConstUnit,
    ConstInt(usize),
    /// `<0xDEADBEEF>`
    ConstMemory(String),
    /// An identifier reference
    Var(String),
}

/// An expression together with its source span.
pub type Expr = Spanned<ExprKind>;

#[derive(Debug, Clone)]
pub enum Pattern {
    /// `p cast as T`
    CastAs(Box<Pattern>, Box<Type>),
    /// `p as T`
    Asc(Box<Pattern>, Box<Type>),
    /// `<| label |>` or `<| label = p |>`
    Variant {
        label: String,
        data: Option<Box<Pattern>>,
    },
    /// `inl(p)`
    Inl(Box<Pattern>),
    /// `inr(p)`
    Inr(Box<Pattern>),
    /// `{p1, p2, ...}`
    Tuple(Vec<Pattern>),
    /// `{field = p, ...}`
    Record(Vec<LabelledPattern>),
    /// `[p1, p2, ...]`
    List(Vec<Pattern>),
    /// `cons(p1, p2)`
    Cons(Box<Pattern>, Box<Pattern>),
    False,
    True,
    Unit,
    Int(usize),
    /// `succ(p)`
    Succ(Box<Pattern>),
    /// An identifier (variable binding)
    Var(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// `auto`
    Auto,
    /// `fn(T1, ...) -> T`
    Fun(Vec<Type>, Box<Type>),
    /// `forall T1, ... . T`
    ForAll(Vec<String>, Box<Type>),
    /// `µ X . T`
    Rec(String, Box<Type>),
    /// `T1 + T2`
    Sum(Box<Type>, Box<Type>),
    /// `{T1, T2, ...}`
    Tuple(Vec<Type>),
    /// `{field: T, ...}`
    Record(Vec<RecordFieldType>),
    /// `<| variant: T, ... |>`
    Variant(Vec<VariantFieldType>),
    /// `[T]`
    List(Box<Type>),
    Bool,
    Nat,
    Unit,
    Top,
    Bottom,
    /// `&T`
    Ref(Box<Type>),
    /// A type variable
    Var(String),
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub expr: Expr,
}

/// `p = e` inside `let`/`letrec`
#[derive(Debug, Clone)]
pub struct PatternBinding {
    pub pattern: Pattern,
    pub expr: Expr,
}

/// `name = e` inside a record expression
#[derive(Debug, Clone)]
pub struct Binding {
    pub name: String,
    pub expr: Expr,
}

/// `name = p` inside a record pattern
#[derive(Debug, Clone)]
pub struct LabelledPattern {
    pub label: String,
    pub pattern: Pattern,
}

/// `name : T` inside a record type
#[derive(Debug, Clone, PartialEq)]
pub struct RecordFieldType {
    pub name: String,
    pub ty: Type,
}

/// `name : T` or just `name` inside a variant type
#[derive(Debug, Clone, PartialEq)]
pub struct VariantFieldType {
    pub name: String,
    pub ty: Option<Type>,
}

use std::fmt;

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Auto => write!(f, "auto"),
            Type::Bool => write!(f, "Bool"),
            Type::Nat => write!(f, "Nat"),
            Type::Unit => write!(f, "Unit"),
            Type::Top => write!(f, "Top"),
            Type::Bottom => write!(f, "Bot"),
            Type::Var(name) => write!(f, "{}", name),
            Type::Fun(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::ForAll(params, ty) => {
                write!(f, "forall ")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ". {}", ty)
            }
            Type::Rec(var, ty) => write!(f, "µ{}.{}", var, ty),
            Type::Sum(l, r) => write!(f, "{} + {}", l, r),
            Type::Tuple(elems) => {
                write!(f, "{{")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "}}")
            }
            Type::Record(fields) => {
                write!(f, "{{")?;
                for (i, ft) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} : {}", ft.name, ft.ty)?;
                }
                write!(f, "}}")
            }
            Type::Variant(vs) => {
                write!(f, "<|")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    if let Some(ty) = &v.ty {
                        write!(f, " {} : {}", v.name, ty)?;
                    } else {
                        write!(f, " {}", v.name)?;
                    }
                }
                write!(f, " |>")
            }
            Type::List(elem) => write!(f, "[{}]", elem),
            Type::Ref(ty) => write!(f, "&{}", ty),
        }
    }
}

impl fmt::Display for ExprKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::ConstTrue => write!(f, "true"),
            ExprKind::ConstFalse => write!(f, "false"),
            ExprKind::ConstUnit => write!(f, "unit"),
            ExprKind::ConstInt(n) => write!(f, "{}", n),
            ExprKind::ConstMemory(s) => write!(f, "<{}>", s),
            ExprKind::Var(name) => write!(f, "{}", name),
            ExprKind::Panic => write!(f, "panic!"),

            ExprKind::Sequence(e1, e2) => write!(f, "{}; {}", e1, e2),
            ExprKind::Assign(lhs, rhs) => write!(f, "{} := {}", lhs, rhs),

            ExprKind::If { cond, then_, else_ } => {
                write!(f, "if {} then {} else {}", cond, then_, else_)
            }

            ExprKind::TypeAsc(e, ty) => write!(f, "{} as {}", e, ty),
            ExprKind::TypeCast(e, ty) => write!(f, "{} cast as {}", e, ty),

            ExprKind::Abstraction { params, body } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} : {}", p.name, p.ty)?;
                }
                write!(f, ") {{ return {} }}", body)
            }

            ExprKind::Application { func, args } => {
                write!(f, "{}(", func)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }

            ExprKind::TypeApplication { expr, type_args } => {
                write!(f, "{}[", expr)?;
                for (i, t) in type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            }

            ExprKind::TypeAbstraction(params, body) => {
                write!(f, "generic [")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "] {}", body)
            }

            ExprKind::Let(bindings, body) => {
                write!(f, "let ")?;
                for (i, b) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", b.pattern, b.expr)?;
                }
                write!(f, " in {}", body)
            }

            ExprKind::LetRec(bindings, body) => {
                write!(f, "letrec ")?;
                for (i, b) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", b.pattern, b.expr)?;
                }
                write!(f, " in {}", body)
            }

            ExprKind::Match { expr, cases } => {
                write!(f, "match {} {{", expr)?;
                for (i, c) in cases.iter().enumerate() {
                    if i > 0 {
                        write!(f, " |")?;
                    }
                    write!(f, " {} => {}", c.pattern, c.expr)?;
                }
                write!(f, " }}")
            }

            ExprKind::Tuple(elems) => {
                write!(f, "{{")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "}}")
            }

            ExprKind::Record(bindings) => {
                write!(f, "{{")?;
                for (i, b) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", b.name, b.expr)?;
                }
                write!(f, "}}")
            }

            ExprKind::DotRecord(e, field) => write!(f, "{}.{}", e, field),
            ExprKind::DotTuple(e, idx) => write!(f, "{}.{}", e, idx),

            ExprKind::List(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }

            ExprKind::ConsList(head, tail) => write!(f, "cons({}, {})", head, tail),
            ExprKind::Head(e) => write!(f, "List::head({})", e),
            ExprKind::Tail(e) => write!(f, "List::tail({})", e),
            ExprKind::IsEmpty(e) => write!(f, "List::isempty({})", e),

            ExprKind::Inl(e) => write!(f, "inl({})", e),
            ExprKind::Inr(e) => write!(f, "inr({})", e),

            ExprKind::Variant {
                label,
                payload: Some(e),
            } => write!(f, "<| {} = {} |>", label, e),
            ExprKind::Variant {
                label,
                payload: None,
            } => write!(f, "<| {} |>", label),

            ExprKind::Succ(e) => write!(f, "succ({})", e),
            ExprKind::Pred(e) => write!(f, "Nat::pred({})", e),
            ExprKind::IsZero(e) => write!(f, "Nat::iszero({})", e),
            ExprKind::NatRec(n, z, s) => write!(f, "Nat::rec({}, {}, {})", n, z, s),

            ExprKind::Fix(e) => write!(f, "fix({})", e),

            ExprKind::Ref(e) => write!(f, "new({})", e),
            ExprKind::Deref(e) => write!(f, "*{}", e),

            ExprKind::Throw(e) => write!(f, "throw({})", e),
            ExprKind::TryCatch {
                try_expr,
                pattern,
                catch_expr,
            } => {
                write!(
                    f,
                    "try {{ {} }} catch {{ {} => {} }}",
                    try_expr, pattern, catch_expr
                )
            }
            ExprKind::TryWith {
                try_expr,
                with_expr,
            } => {
                write!(f, "try {{ {} }} with {{ {} }}", try_expr, with_expr)
            }
            ExprKind::TryCastAs {
                try_expr,
                ty,
                pattern,
                catch_expr,
                with_expr,
            } => {
                write!(
                    f,
                    "try {{ {} }} cast as {} {{ {} => {} }} with {{ {} }}",
                    try_expr, ty, pattern, catch_expr, with_expr
                )
            }

            ExprKind::Fold { ty, expr } => write!(f, "fold[{}] {}", ty, expr),
            ExprKind::Unfold { ty, expr } => write!(f, "unfold[{}] {}", ty, expr),

            ExprKind::Add(l, r) => write!(f, "{} + {}", l, r),
            ExprKind::Subtract(l, r) => write!(f, "{} - {}", l, r),
            ExprKind::Multiply(l, r) => write!(f, "{} * {}", l, r),
            ExprKind::Divide(l, r) => write!(f, "{} / {}", l, r),
            ExprKind::LessThan(l, r) => write!(f, "{} < {}", l, r),
            ExprKind::LessThanOrEqual(l, r) => write!(f, "{} <= {}", l, r),
            ExprKind::GreaterThan(l, r) => write!(f, "{} > {}", l, r),
            ExprKind::GreaterThanOrEqual(l, r) => write!(f, "{} >= {}", l, r),
            ExprKind::Equal(l, r) => write!(f, "{} == {}", l, r),
            ExprKind::NotEqual(l, r) => write!(f, "{} != {}", l, r),
            ExprKind::LogicAnd(l, r) => write!(f, "{} && {}", l, r),
            ExprKind::LogicOr(l, r) => write!(f, "{} || {}", l, r),
            ExprKind::LogicNot(e) => write!(f, "not({})", e),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.node, f)
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pattern::Var(name) => write!(f, "{}", name),
            Pattern::True => write!(f, "true"),
            Pattern::False => write!(f, "false"),
            Pattern::Unit => write!(f, "unit"),
            Pattern::Int(n) => write!(f, "{}", n),
            Pattern::Inl(p) => write!(f, "inl({})", p),
            Pattern::Inr(p) => write!(f, "inr({})", p),
            Pattern::Succ(p) => write!(f, "succ({})", p),
            Pattern::Cons(h, t) => write!(f, "cons({}, {})", h, t),
            Pattern::Tuple(pats) => {
                write!(f, "{{")?;
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "}}")
            }
            Pattern::Record(fields) => {
                write!(f, "{{")?;
                for (i, lp) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{} = {}", lp.label, lp.pattern)?;
                }
                write!(f, "}}")
            }
            Pattern::List(pats) => {
                write!(f, "[")?;
                for (i, p) in pats.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, "]")
            }
            Pattern::Variant {
                label,
                data: Some(p),
            } => write!(f, "<| {} = {} |>", label, p),
            Pattern::Variant { label, data: None } => write!(f, "<| {} |>", label),
            Pattern::Asc(p, ty) => write!(f, "{} as {}", p, ty),
            Pattern::CastAs(p, ty) => write!(f, "{} cast as {}", p, ty),
        }
    }
}
