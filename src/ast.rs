/// Root program node.
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

// ---------------------------------------------------------------------------
// Declarations
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Expressions  (all precedence levels collapsed into one enum)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Expr {
    // --- Expr (top level) ---
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

    // --- Expr2 (non-associative comparisons) ---
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

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Helper structs
// ---------------------------------------------------------------------------

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
