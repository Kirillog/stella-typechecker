use crate::ast::{Decl, DeclFun, Expr, Pattern, Program, RecordFieldType, Type};
use crate::type_error::TypeError;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Typing context
// ---------------------------------------------------------------------------

/// Immutable typing environment: maps variable names to their types.
#[derive(Debug, Clone, Default)]
pub struct Context {
    vars: HashMap<String, Type>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a new `Context` that extends `self` with `name : ty`.
    pub fn extend(&mut self, name: impl Into<String>, ty: Type) {
        self.vars.insert(name.into(), ty);
    }

    /// Look up a variable, returning its type or `None` if unbound.
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        return self.vars.get(name);
    }
}

// ---------------------------------------------------------------------------
// Structural type equality
// ---------------------------------------------------------------------------

/// Deep structural equality — no subtyping.
pub fn types_equal(t1: &Type, t2: &Type) -> bool {
    return t1 == &Type::Auto || t2 == &Type::Auto || t1 == t2;
}

// ---------------------------------------------------------------------------
// Expression inference
// ---------------------------------------------------------------------------

pub struct TypeChecker {
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn get_errors(self) -> Vec<TypeError> {
        self.errors
    }

    pub fn assert_types_equal(&mut self, expected: &Type, got: &Type) {
        if !types_equal(expected, got) {
            self.errors.push(TypeError::UnexpectedTypeForExpression {
                expected: expected.clone(),
                got: got.clone(),
            });
        }
    }

    /// Infer the type of `expr` under `ctx`.
    ///
    /// On success returns `Some(ty)`.  On failure pushes one or more errors into
    /// `errors` and returns `None` so that checking can continue.
    pub fn infer(&mut self, ctx: &Context, expr: &Expr) -> Option<Type> {
        match expr {
            // --- literals ---
            Expr::ConstTrue | Expr::ConstFalse => Some(Type::Bool),
            Expr::ConstUnit => Some(Type::Unit),
            Expr::ConstInt(_) => Some(Type::Nat),

            // --- variable lookup ---
            Expr::Var(_name) => ctx.lookup(_name).cloned().or_else(|| {
                self.errors
                    .push(TypeError::UndefinedVariable(_name.clone()));
                None
            }),

            // --- control flow ---
            Expr::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                let then_ty = self.infer(ctx, then_)?;
                self.check(ctx, else_, &then_ty);
                Some(then_ty)
            }
            // --- type ascription ---
            Expr::TypeAsc(_e, _ty) => {
                self.check(ctx, _e, _ty);
                Some(_ty.as_ref().clone())
            }

            // --- sequence ---
            Expr::Sequence(e1, e2) => {
                self.infer(ctx, e1)?;
                self.infer(ctx, e2)
            }

            // --- let ---
            Expr::Let(bindings, body) => {
                let mut local_ctx = ctx.clone();
                for binding in bindings {
                    match &binding.pattern {
                        Pattern::Var(name) => {
                            let ty = self.infer(&local_ctx, &binding.expr)?;
                            local_ctx.extend(name.clone(), ty);
                        }
                        Pattern::Asc(inner, expected_ty) => {
                            self.check(&local_ctx, &binding.expr, expected_ty);
                            if let Pattern::Var(name) = inner.as_ref() {
                                local_ctx.extend(name.clone(), expected_ty.as_ref().clone());
                            } else {
                                unimplemented!() // add test
                            }
                        }
                        _ => unimplemented!(),
                    }
                }
                self.infer(&local_ctx, body)
            }

            // --- functions ---
            Expr::Abstraction { params, body } => {
                let mut local_ctx = ctx.clone();
                let param_types = params
                    .iter()
                    .map(|p| {
                        if p.ty == Type::Auto {
                            // cannot infer parameter types without annotations
                            self.errors.push(TypeError::UnexpectedTypeForParameter {
                                param: p.name.clone(),
                                expected: Type::Auto,
                                got: Type::Auto,
                            });
                            None
                        } else {
                            local_ctx.extend(p.name.clone(), p.ty.clone());
                            Some(p.ty.clone())
                        }
                    })
                    .collect::<Option<Vec<Type>>>()?;
                let body_ty = self.infer(&local_ctx, body)?;
                Some(Type::Fun(param_types, Box::new(body_ty)))
            }
            Expr::Application { func, args } => {
                let func_ty = self.infer(ctx, func)?;
                match &func_ty {
                    Type::Fun(param_types, return_type) if param_types.len() == args.len() => {
                        for (arg, param_ty) in args.iter().zip(param_types) {
                            self.check(ctx, arg, param_ty);
                        }
                        Some(return_type.as_ref().clone())
                    }
                    Type::Fun(param_types, _) => {
                        self.errors.push(TypeError::IncorrectNumberOfArguments {
                            expected: param_types.len(),
                            got: args.len(),
                        });
                        None
                    }
                    _ => {
                        self.errors.push(TypeError::NotAFunction(func_ty));
                        None
                    }
                }
            }

            // --- arithmetic (Nat × Nat → Nat) ---
            Expr::Add(left, right)
            | Expr::Subtract(left, right)
            | Expr::Multiply(left, right)
            | Expr::Divide(left, right) => {
                self.check(ctx, left, &Type::Nat);
                self.check(ctx, right, &Type::Nat);
                Some(Type::Nat)
            }

            // --- comparisons (Nat × Nat → Bool) ---
            Expr::LessThan(left, right)
            | Expr::LessThanOrEqual(left, right)
            | Expr::GreaterThan(left, right)
            | Expr::GreaterThanOrEqual(left, right)
            | Expr::Equal(left, right)
            | Expr::NotEqual(left, right) => {
                self.check(ctx, left, &Type::Nat);
                self.check(ctx, right, &Type::Nat);
                Some(Type::Bool)
            }
            // --- logic (Bool × Bool → Bool) ---
            Expr::LogicAnd(left, right) | Expr::LogicOr(left, right) => {
                self.check(ctx, left, &Type::Bool);
                self.check(ctx, right, &Type::Bool);
                Some(Type::Bool)
            }
            Expr::LogicNot(e) => {
                self.check(ctx, e, &Type::Bool);
                Some(Type::Bool)
            }

            // --- natural number builtins ---
            Expr::Succ(e) | Expr::Pred(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Nat)
            }
            Expr::IsZero(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Bool)
            }
            Expr::NatRec(_n, _z, _s) => {
                // n:Nat, z:T, s:fn(Nat)->fn(T)->T → T
                self.check(ctx, _n, &Type::Nat);
                let z_ty = self.infer(ctx, &_z)?;
                self.check(
                    ctx,
                    _s,
                    &Type::Fun(
                        vec![Type::Nat],
                        Box::new(Type::Fun(vec![z_ty.clone()], Box::new(z_ty.clone()))),
                    ),
                );
                Some(z_ty)
            }

            // --- fixpoint ---
            Expr::Fix(_f) => {
                // fn(T)->T → T
                let f_ty = self.infer(ctx, _f)?;
                match &f_ty {
                    Type::Fun(param_types, return_type) if param_types.len() == 1 => {
                        let param_ty = &param_types[0];
                        self.assert_types_equal(param_ty, return_type.as_ref());
                        Some(param_ty.clone())
                    }
                    Type::Fun(param_types, _) => {
                        self.errors.push(TypeError::IncorrectNumberOfArguments {
                            expected: 1,
                            got: param_types.len(),
                        });
                        None
                    }
                    _ => {
                        self.errors.push(TypeError::UnexpectedTypeForExpression {
                            expected: Type::Fun(vec![Type::Auto], Box::new(Type::Auto)),
                            got: f_ty,
                        });
                        None
                    }
                }
            }

            // --- tuple literal ---
            Expr::Tuple(exprs) => {
                let elem_types = exprs
                    .iter()
                    .map(|e| self.infer(ctx, e))
                    .collect::<Option<Vec<Type>>>()?;
                Some(Type::Tuple(elem_types))
            }

            Expr::DotTuple(e, index) => {
                let tuple_ty = self.infer(ctx, e)?;
                match tuple_ty {
                    Type::Tuple(elem_types) if *index < elem_types.len() => {
                        Some(elem_types[*index].clone())
                    }
                    _ => {
                        self.errors.push(TypeError::NotATuple(tuple_ty));
                        None
                    }
                }
            }

            // --- record literal ---
            Expr::Record(bindings) => {
                let mut seen = HashMap::new();
                let mut field_types = Vec::new();
                for binding in bindings {
                    if seen.insert(binding.name.clone(), ()).is_some() {
                        self.errors.push(TypeError::DuplicateRecordFields {
                            field: binding.name.clone(),
                        });
                    }
                    let ty = self.infer(ctx, &binding.expr)?;
                    field_types.push(RecordFieldType {
                        name: binding.name.clone(),
                        ty,
                    });
                }
                Some(Type::Record(field_types))
            }

            // --- sum injections: cannot synthesise without an expected type ---
            Expr::Inl(_) | Expr::Inr(_) => {
                self.errors.push(TypeError::AmbiguousSumType);
                None
            }

            // --- variant construction: cannot synthesise without an expected type ---
            Expr::Variant { .. } => {
                self.errors.push(TypeError::AmbiguousVariantType);
                None
            }

            // --- list literal ---
            Expr::List(exprs) => {
                if exprs.is_empty() {
                    self.errors.push(TypeError::AmbiguousList);
                    None
                } else {
                    let ty = self.infer(ctx, &exprs[0])?;
                    exprs.iter().skip(1).for_each(|e| self.check(ctx, e, &ty));
                    Some(Type::List(Box::new(ty)))
                }
            }
            Expr::ConsList(head, tail) => {
                let elem_ty = self.infer(ctx, head)?;
                self.check(ctx, tail, &Type::List(Box::new(elem_ty.clone())));
                Some(Type::List(Box::new(elem_ty)))
            }
            Expr::Head(e) | Expr::Tail(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(elem_ty) => Some(*elem_ty),
                    a => {
                        self.errors.push(TypeError::NotAList(a));
                        None
                    }
                }
            }
            Expr::IsEmpty(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(_) => Some(Type::Bool),
                    a => {
                        self.errors.push(TypeError::NotAList(a));
                        None
                    }
                }
            }

            _ => unimplemented!(),
        }
    }

    // ---------------------------------------------------------------------------
    // Expression checking  (analysis / "checking" mode)
    // ---------------------------------------------------------------------------

    /// Check that `expr` has type `expected` under `ctx`.
    pub fn check(&mut self, ctx: &Context, expr: &Expr, expected: &Type) {
        match expr {
            // --- literals ---
            Expr::ConstTrue | Expr::ConstFalse => {
                self.assert_types_equal(&Type::Bool, expected);
            }
            Expr::ConstUnit => {
                self.assert_types_equal(&Type::Unit, expected);
            }
            Expr::ConstInt(_) => {
                self.assert_types_equal(&Type::Nat, expected);
            }
            // --- variable lookup ---
            Expr::Var(name) => {
                let var_ty = ctx.lookup(name).cloned().unwrap_or_else(|| {
                    self.errors.push(TypeError::UndefinedVariable(name.clone()));
                    Type::Auto
                });
                self.assert_types_equal(&var_ty, expected);
            }
            // --- control flow ---
            Expr::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                self.check(ctx, then_, expected);
                self.check(ctx, else_, expected);
            }

            Expr::TypeAsc(e, ty) => {
                self.assert_types_equal(ty, expected);
                self.check(ctx, e, ty);
            }
            // --- lambda (abstraction) ---
            Expr::Abstraction { params, body } => match expected {
                Type::Fun(param_types, return_type) => {
                    if param_types.len() != params.len() {
                        self.errors
                            .push(TypeError::UnexpectedNumberOfParametersInLambda {
                                expected: param_types.len(),
                                got: params.len(),
                            });
                    }
                    let mut local_ctx = ctx.clone();
                    for (param, expected_param_ty) in params.iter().zip(param_types) {
                        if param.ty != Type::Auto {
                            self.errors.push(TypeError::UnexpectedTypeForParameter {
                                param: param.name.clone(),
                                expected: expected_param_ty.clone(),
                                got: param.ty.clone(),
                            });
                        }
                        local_ctx.extend(param.name.clone(), expected_param_ty.clone());
                    }
                    self.check(&local_ctx, body, return_type);
                }
                _ => self.errors.push(TypeError::UnexpectedLambda {
                    expected: expected.clone(),
                }),
            },
            // --- application ---
            Expr::Application { func, args } => match self.infer(ctx, func) {
                Some(Type::Fun(param_types, return_type)) => {
                    if param_types.len() != args.len() {
                        self.errors.push(TypeError::IncorrectNumberOfArguments {
                            expected: param_types.len(),
                            got: args.len(),
                        });
                    } else {
                        for (arg, param_ty) in args.iter().zip(param_types) {
                            self.check(ctx, arg, &param_ty);
                        }
                    }
                    self.assert_types_equal(return_type.as_ref(), expected);
                }
                Some(func_ty) => self.errors.push(TypeError::NotAFunction(func_ty)),
                None => self.errors.push(TypeError::AmbiguousFunction),
            },

            // --- tuple ---
            Expr::Tuple(exprs) => match expected {
                Type::Tuple(elem_types) => {
                    if elem_types.len() != exprs.len() {
                        self.errors.push(TypeError::UnexpectedTupleLength {
                            expected: elem_types.len(),
                            got: exprs.len(),
                        });
                    }
                    for (e, ty) in exprs.iter().zip(elem_types) {
                        self.check(ctx, e, ty);
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedTuple {
                    expected: expected.clone(),
                }),
            },

            Expr::DotTuple(e, index) => match self.infer(ctx, e) {
                Some(Type::Tuple(elem_types)) => {
                    if *index < elem_types.len() {
                        self.assert_types_equal(&elem_types[*index], expected);
                    } else {
                        self.errors.push(TypeError::TupleIndexOutOfBounds {
                            index: *index,
                            length: elem_types.len(),
                        });
                    }
                }
                Some(tuple_ty) => self.errors.push(TypeError::NotATuple(tuple_ty)),
                None => self.errors.push(TypeError::AmbiguousTuple),
            },

            // --- record ---
            Expr::Record(bindings) => match expected {
                Type::Record(field_types) => {
                    let mut seen = HashMap::new();
                    for b in bindings {
                        if seen.insert(b.name.clone(), ()).is_some() {
                            self.errors.push(TypeError::DuplicateRecordFields {
                                field: b.name.clone(),
                            });
                        }
                    }
                    let binding_names: Vec<&str> =
                        bindings.iter().map(|b| b.name.as_str()).collect();
                    let expected_names: Vec<&str> =
                        field_types.iter().map(|f| f.name.as_str()).collect();
                    let missing: Vec<String> = expected_names
                        .iter()
                        .filter(|n| !binding_names.contains(n))
                        .map(|n| n.to_string())
                        .collect();
                    if !missing.is_empty() {
                        self.errors.push(TypeError::MissingRecordFields { missing });
                    }
                    let unexpected: Vec<String> = binding_names
                        .iter()
                        .filter(|n| !expected_names.contains(n))
                        .map(|n| n.to_string())
                        .collect();
                    if !unexpected.is_empty() {
                        self.errors
                            .push(TypeError::UnexpectedRecordFields { unexpected });
                    }
                    for binding in bindings {
                        if let Some(ft) = field_types.iter().find(|f| f.name == binding.name) {
                            self.check(ctx, &binding.expr, &ft.ty);
                        }
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedRecord {
                    expected: expected.clone(),
                }),
            },

            // --- variant construction ---
            Expr::Variant { label, payload } => match expected {
                Type::Variant(variants) => {
                    if let Some(vt) = variants.iter().find(|v| v.name == *label) {
                        match (&vt.ty, payload) {
                            (None, None) => {}
                            (Some(ty), Some(expr)) => self.check(ctx, expr, ty),
                            _ => self.errors.push(TypeError::AmbiguousVariantType {}),
                        }
                    } else {
                        self.errors.push(TypeError::UnexpectedVariantLabel {
                            label: label.clone(),
                            variant_type: expected.clone(),
                        });
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedVariant {
                    expected: expected.clone(),
                }),
            },

            // --- sum type injections ---
            Expr::Inl(inner) => match expected {
                Type::Sum(left, _) => self.check(ctx, inner, left),
                _ => self.errors.push(TypeError::UnexpectedInjection {
                    expected: expected.clone(),
                }),
            },
            Expr::Inr(inner) => match expected {
                Type::Sum(_, right) => self.check(ctx, inner, right),
                _ => self.errors.push(TypeError::UnexpectedInjection {
                    expected: expected.clone(),
                }),
            },

            // --- list literal ---
            Expr::List(exprs) => match expected {
                Type::List(elem_ty) => {
                    for e in exprs {
                        self.check(ctx, e, elem_ty);
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedList {
                    expected: expected.clone(),
                }),
            },
            Expr::ConsList(head, tail) => match expected {
                Type::List(elem_ty) => {
                    self.check(ctx, head, elem_ty);
                    self.check(ctx, tail, expected);
                }
                _ => self.errors.push(TypeError::UnexpectedList {
                    expected: expected.clone(),
                }),
            },
            Expr::Head(e) | Expr::Tail(e) => match expected {
                Type::List(elem_ty) => {
                    self.check(ctx, e, expected);
                    self.assert_types_equal(elem_ty, expected);
                }
                _ => self.errors.push(TypeError::UnexpectedList {
                    expected: expected.clone(),
                }),
            },
            Expr::IsEmpty(e) => {
                self.assert_types_equal(expected, &Type::Bool);
                match self.infer(ctx, e) {
                    Some(Type::List(_)) => {}
                    Some(a) => self.errors.push(TypeError::NotAList(a)),
                    None => self.errors.push(TypeError::AmbiguousList),
                }
            }

            // --- default: synthesise and compare ---
            _ => {
                if let Some(got) = self.infer(ctx, expr) {
                    self.assert_types_equal(expected, &got);
                }
            }
        }
    }

    fn check_decl(&mut self, ctx: &Context, decl: &Decl) {
        match decl {
            Decl::Fun(f) => self.check_fun(ctx, f),

            Decl::FunGeneric(_)
            | Decl::TypeAlias { .. }
            | Decl::ExceptionType { .. }
            | Decl::ExceptionVariant { .. } => unimplemented!(),
        }
    }

    fn check_fun(&mut self, ctx: &Context, f: &DeclFun) {
        let mut local_ctx = ctx.clone();
        let mut seen_params = HashSet::new();
        for p in &f.params {
            if !seen_params.insert(p.name.clone()) {
                self.errors.push(TypeError::DuplicateFunctionParameter {
                    name: p.name.clone(),
                });
                continue;
            }
            local_ctx.extend(p.name.clone(), p.ty.clone());
        }
        self.check(&local_ctx, &f.body, &f.return_type.as_type());
    }

    /// Typecheck a whole program and return every error found.
    pub fn check_program(mut self, prog: &Program) -> Vec<TypeError> {
        let mut ctx = Context::new();

        for decl in &prog.decls {
            match &decl {
                Decl::Fun(f) => {
                    let param_types = f.params.iter().map(|p| p.ty.clone()).collect();
                    let return_type = Box::new(f.return_type.as_type().clone());
                    let fun_type = Type::Fun(param_types, return_type);
                    ctx.extend(f.name.clone(), fun_type);
                }
                Decl::TypeAlias { name, ty } => {
                    ctx.extend(name.clone(), ty.clone());
                }
                Decl::FunGeneric(_)
                | Decl::ExceptionType { .. }
                | Decl::ExceptionVariant { .. } => {
                    unimplemented!()
                }
            }
        }

        self.check_multiple_fun_definition(prog);
        self.check_missing_main(prog);
        self.check_main_arity(prog);

        for decl in &prog.decls {
            self.check_decl(&ctx, decl);
        }

        self.errors
    }

    fn check_missing_main(&mut self, prog: &Program) {
        if prog
            .decls
            .iter()
            .filter(|d| matches!(d, Decl::Fun(f) if f.name == "main"))
            .count()
            == 0
        {
            self.errors.push(TypeError::MissingMain);
        }
    }

    fn check_main_arity(&mut self, prog: &Program) {
        if let Some(Decl::Fun(f)) = prog
            .decls
            .iter()
            .filter(|d| matches!(d, Decl::Fun(f) if f.name == "main" && f.params.len() != 1))
            .next()
        {
            self.errors.push(TypeError::IncorrectArityOfMain {
                got: f.params.len(),
            });
        }
    }

    fn check_multiple_fun_definition(&mut self, prog: &Program) {
        let mut fun_name_counts: HashMap<String, usize> = HashMap::new();
        for decl in &prog.decls {
            if let Decl::Fun(f) = decl {
                *fun_name_counts.entry(f.name.clone()).or_insert(0) += 1;
            }
        }
        for (name, count) in fun_name_counts {
            if count > 1 {
                self.errors
                    .push(TypeError::DuplicateFunctionDeclaration { name });
            }
        }
    }
}
