use crate::ast::{
    Decl, DeclFun, DeclFunGeneric, Expr, ParamDecl, Pattern, Program, RecordFieldType, ReturnType,
    Type, VariantFieldType,
};
use crate::type_error::TypeError;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct Context {
    vars: HashMap<String, Type>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extend(&mut self, name: impl Into<String>, ty: Type) -> Option<Type> {
        return self.vars.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        return self.vars.get(name);
    }
}

pub fn types_equal(t1: &Type, t2: &Type) -> bool {
    return t1 == &Type::Auto || t2 == &Type::Auto || t1 == t2;
}

pub struct TypeChecker {
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn assert_types_equal(&mut self, expected: &Type, got: &Type) {
        if !types_equal(expected, got) {
            self.errors.push(TypeError::UnexpectedTypeForExpression {
                expected: expected.clone(),
                got: got.clone(),
            });
        }
    }

    pub fn infer(&mut self, ctx: &Context, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::ConstTrue | Expr::ConstFalse => Some(Type::Bool),
            Expr::ConstUnit => Some(Type::Unit),
            Expr::ConstInt(_) => Some(Type::Nat),

            Expr::Var(_name) => ctx.lookup(_name).cloned().or_else(|| {
                self.errors
                    .push(TypeError::UndefinedVariable(_name.clone()));
                None
            }),

            Expr::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                let then_ty = self.infer(ctx, then_)?;
                self.check(ctx, else_, &then_ty);
                Some(then_ty)
            }

            Expr::TypeAsc(_e, _ty) => {
                self.check(ctx, _e, _ty);
                Some(_ty.as_ref().clone())
            }

            Expr::Sequence(e1, e2) => {
                self.infer(ctx, e1)?;
                self.infer(ctx, e2)
            }

            Expr::Let(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.errors.push(TypeError::DuplicateLetBinding { name });
                        }
                    }
                    if let Some(ty) = self.infer(&ctx, &binding.expr) {
                        self.check_let_exhaustiveness(&binding.pattern, &ty);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.infer(&local_ctx, body)
            }
            Expr::LetRec(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.errors.push(TypeError::DuplicateLetBinding { name });
                        }
                    }
                }
                // Allow mutual and self-recursion by first extending the context with all bindings
                for binding in bindings {
                    if let Some(ty) = Self::extract_declared_type(&binding.pattern) {
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                for binding in bindings {
                    if let Some(ty) = Self::extract_declared_type(&binding.pattern) {
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty);
                        self.check(&local_ctx, &binding.expr, &ty);
                    } else if let Some(ty) = self.infer(&local_ctx, &binding.expr) {
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.infer(&local_ctx, body)
            }

            Expr::Abstraction { params, body } => {
                let mut local_ctx = ctx.clone();
                let param_types = params
                    .iter()
                    .map(|p| {
                        if p.ty == Type::Auto {
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

            Expr::Add(left, right)
            | Expr::Subtract(left, right)
            | Expr::Multiply(left, right)
            | Expr::Divide(left, right) => {
                self.check(ctx, left, &Type::Nat);
                self.check(ctx, right, &Type::Nat);
                Some(Type::Nat)
            }

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

            Expr::LogicAnd(left, right) | Expr::LogicOr(left, right) => {
                self.check(ctx, left, &Type::Bool);
                self.check(ctx, right, &Type::Bool);
                Some(Type::Bool)
            }
            Expr::LogicNot(e) => {
                self.check(ctx, e, &Type::Bool);
                Some(Type::Bool)
            }

            Expr::Succ(e) | Expr::Pred(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Nat)
            }
            Expr::IsZero(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Bool)
            }
            Expr::NatRec(_n, _z, _s) => {
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

            Expr::Fix(_f) => {
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
                    Type::Tuple(elem_types) if *index >= 1 && *index - 1 < elem_types.len() => {
                        Some(elem_types[*index - 1].clone())
                    }
                    Type::Tuple(elem_types) => {
                        self.errors.push(TypeError::TupleIndexOutOfBounds {
                            index: *index,
                            length: elem_types.len(),
                        });
                        None
                    }
                    _ => {
                        self.errors.push(TypeError::NotATuple(tuple_ty));
                        None
                    }
                }
            }

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

            Expr::DotRecord(expr, field) => match self.infer(ctx, expr) {
                Some(Type::Record(field_types)) => {
                    if let Some(ft) = field_types.iter().find(|f| f.name == *field) {
                        Some(ft.ty.clone())
                    } else {
                        self.errors.push(TypeError::UnexpectedFieldAccess {
                            field: field.clone(),
                            record_type: Type::Record(field_types),
                        });
                        None
                    }
                }
                Some(record_ty) => {
                    self.errors.push(TypeError::NotARecord(record_ty));
                    None
                }
                None => None,
            },

            Expr::Inl(_) | Expr::Inr(_) => {
                self.errors.push(TypeError::AmbiguousSumType);
                None
            }

            Expr::Variant { .. } => {
                self.errors.push(TypeError::AmbiguousVariantType);
                None
            }

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
            Expr::Head(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(elem_ty) => Some(*elem_ty),
                    a => {
                        self.errors.push(TypeError::NotAList(a));
                        None
                    }
                }
            }
            Expr::Tail(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(elem_ty) => Some(Type::List(elem_ty)),
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
            Expr::Match { expr, cases } => {
                let scrutinee_ty = self.infer(ctx, expr)?;
                let mut case_types = Vec::new();
                for match_case in cases {
                    let mut local_ctx = ctx.clone();
                    self.extend_ctx_by_pattern(&mut local_ctx, &match_case.pattern, &scrutinee_ty);
                    let case_ty = self.infer(&local_ctx, &match_case.expr)?;
                    case_types.push(case_ty);
                }
                if cases.is_empty() {
                    self.errors.push(TypeError::IllegalEmptyMatching {});
                    return None;
                }
                let patterns: Vec<&Pattern> = cases.iter().map(|c| &c.pattern).collect();
                self.check_match_exhaustiveness(&scrutinee_ty, &patterns);
                if let Some(first_case_ty) = case_types.first() {
                    for case_ty in &case_types[1..] {
                        self.assert_types_equal(first_case_ty, case_ty);
                    }
                    Some(first_case_ty.clone())
                } else {
                    None
                }
            }
            _ => unimplemented!(),
        }
    }

    pub fn check(&mut self, ctx: &Context, expr: &Expr, expected: &Type) {
        match expr {
            Expr::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                self.check(ctx, then_, expected);
                self.check(ctx, else_, expected);
            }

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
                        if param.ty != *expected_param_ty {
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
                    if *index >= 1 && *index - 1 < elem_types.len() {
                        self.assert_types_equal(&elem_types[*index - 1], expected);
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
                    seen.clear();
                    for f in field_types {
                        if seen.insert(f.name.clone(), ()).is_some() {
                            self.errors.push(TypeError::DuplicateRecordTypeFields {
                                field: f.name.clone(),
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

            Expr::Variant { label, payload } => match expected {
                Type::Variant(variants) => {
                    let mut seen = HashMap::new();
                    for v in variants {
                        if seen.insert(v.name.clone(), ()).is_some() {
                            self.errors.push(TypeError::DuplicateVariantTypeFields {
                                label: v.name.clone(),
                            });
                        }
                    }
                    if let Some(vt) = variants.iter().find(|v| v.name == *label) {
                        match (&vt.ty, payload) {
                            (_, Some(_)) if *label == "none" => {
                                self.errors.push(TypeError::UnexpectedDataForNullaryLabel {
                                    label: label.clone(),
                                })
                            }
                            (_, None) if *label == "some" => {
                                self.errors.push(TypeError::MissingDataForLabel {
                                    label: label.clone(),
                                })
                            }
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
            Expr::IsEmpty(e) => {
                self.assert_types_equal(expected, &Type::Bool);
                match self.infer(ctx, e) {
                    Some(Type::List(_)) => {}
                    Some(a) => self.errors.push(TypeError::NotAList(a)),
                    None => self.errors.push(TypeError::AmbiguousList),
                }
            }
            Expr::Match { expr, cases } => match self.infer(ctx, expr) {
                Some(scrutinee_ty) => {
                    if cases.is_empty() {
                        self.errors.push(TypeError::IllegalEmptyMatching {});
                    } else {
                        let patterns: Vec<&Pattern> = cases.iter().map(|c| &c.pattern).collect();
                        self.check_match_exhaustiveness(&scrutinee_ty, &patterns);
                        for match_case in cases {
                            let mut local_ctx = ctx.clone();
                            self.extend_ctx_by_pattern(
                                &mut local_ctx,
                                &match_case.pattern,
                                &scrutinee_ty,
                            );
                            self.check(&local_ctx, &match_case.expr, expected);
                        }
                    }
                }
                None => (),
            },
            Expr::Let(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.errors.push(TypeError::DuplicateLetBinding { name });
                        }
                    }
                    if let Some(ty) = self.infer(&local_ctx, &binding.expr) {
                        self.check_let_exhaustiveness(&binding.pattern, &ty);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.check(&local_ctx, body, expected);
            }
            Expr::LetRec(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.errors.push(TypeError::DuplicateLetBinding { name });
                        }
                    }
                }
                // Allow mutual and self-recursion by first extending the context with all bindings
                for binding in bindings {
                    if let Some(ty) = Self::extract_declared_type(&binding.pattern) {
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                for binding in bindings {
                    if let Some(ty) = Self::extract_declared_type(&binding.pattern) {
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty);
                        self.check(&local_ctx, &binding.expr, &ty);
                    } else if let Some(ty) = self.infer(&local_ctx, &binding.expr) {
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.check(&local_ctx, body, expected);
            }

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

            Decl::FunGeneric(f) => self.check_fun_generic(ctx, f),
            Decl::TypeAlias { .. } | Decl::ExceptionType { .. } | Decl::ExceptionVariant { .. } => {
                unimplemented!()
            }
        }
    }

    fn check_fun_body(
        &mut self,
        ctx: &Context,
        params: &[ParamDecl],
        return_type: &ReturnType,
        local_decls: &[Decl],
        body: &Expr,
    ) {
        let mut local_ctx = ctx.clone();
        let mut seen_params = HashSet::new();
        for p in params {
            if !seen_params.insert(p.name.clone()) {
                self.errors.push(TypeError::DuplicateFunctionParameter {
                    name: p.name.clone(),
                });
                continue;
            }
            self.validate_type(&p.ty);
            local_ctx.extend(p.name.clone(), p.ty.clone());
        }
        self.validate_type(return_type.as_type());
        Self::extend_ctx(local_decls, &mut local_ctx);
        for decl in local_decls {
            self.check_decl(&local_ctx, decl);
        }
        self.check(&local_ctx, body, &return_type.as_type());
    }

    fn validate_type(&mut self, ty: &Type) {
        match ty {
            Type::Record(fields) => {
                let mut seen = HashMap::new();
                for f in fields {
                    if seen.insert(f.name.clone(), ()).is_some() {
                        self.errors.push(TypeError::DuplicateRecordTypeFields {
                            field: f.name.clone(),
                        });
                    }
                    self.validate_type(&f.ty);
                }
            }
            Type::Variant(variants) => {
                let mut seen = HashMap::new();
                for v in variants {
                    if seen.insert(v.name.clone(), ()).is_some() {
                        self.errors.push(TypeError::DuplicateVariantTypeFields {
                            label: v.name.clone(),
                        });
                    }
                    if let Some(inner_ty) = &v.ty {
                        self.validate_type(inner_ty);
                    }
                }
            }
            Type::Fun(params, ret) => {
                for p in params {
                    self.validate_type(p);
                }
                self.validate_type(ret);
            }
            Type::Tuple(elems) => {
                for e in elems {
                    self.validate_type(e);
                }
            }
            Type::List(inner) => self.validate_type(inner),
            Type::Sum(l, r) => {
                self.validate_type(l);
                self.validate_type(r);
            }
            _ => {}
        }
    }

    fn check_fun(&mut self, ctx: &Context, f: &DeclFun) {
        self.check_fun_body(ctx, &f.params, &f.return_type, &f.local_decls, &f.body);
    }

    fn check_fun_generic(&mut self, ctx: &Context, f: &DeclFunGeneric) {
        for type_param in &f.type_params {
            if f.type_params.iter().filter(|tp| *tp == type_param).count() > 1 {
                self.errors.push(TypeError::DuplicateTypeParameter {
                    name: type_param.clone(),
                });
            }
        }
        self.check_fun_body(ctx, &f.params, &f.return_type, &f.local_decls, &f.body);
    }

    fn extend_ctx(decls: &[Decl], ctx: &mut Context) {
        for decl in decls {
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
                Decl::FunGeneric(f) => {
                    let param_types = f.params.iter().map(|p| p.ty.clone()).collect();
                    let return_type = Box::new(f.return_type.as_type().clone());
                    let fun_type = Type::Fun(param_types, return_type);
                    ctx.extend(f.name.clone(), fun_type);
                }
                Decl::ExceptionType { .. } | Decl::ExceptionVariant { .. } => {
                    unimplemented!()
                }
            }
        }
    }

    fn extract_declared_type(pattern: &Pattern) -> Option<Type> {
        match pattern {
            Pattern::Asc(_, ty) | Pattern::CastAs(_, ty) => Some(*ty.clone()),
            Pattern::Variant { label, data } => {
                let field = VariantFieldType {
                    name: label.clone(),
                    ty: data.as_deref().and_then(Self::extract_declared_type),
                };
                Some(Type::Variant(vec![field]))
            }
            Pattern::Inl(inner) => {
                let _ = Self::extract_declared_type(inner)?;
                None
            }
            Pattern::Inr(inner) => {
                let _ = Self::extract_declared_type(inner)?;
                None
            }
            Pattern::Tuple(pats) => {
                let types = pats
                    .iter()
                    .map(Self::extract_declared_type)
                    .collect::<Option<Vec<_>>>()?;
                Some(Type::Tuple(types))
            }
            Pattern::Record(fields) => {
                let types = fields
                    .iter()
                    .map(|f| {
                        Some(RecordFieldType {
                            name: f.label.clone(),
                            ty: Self::extract_declared_type(&f.pattern)?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(Type::Record(types))
            }
            Pattern::List(pats) => {
                let mut tys = pats
                    .iter()
                    .map(Self::extract_declared_type)
                    .collect::<Option<Vec<_>>>()?;
                let elem = tys.pop()?;
                if tys.iter().all(|t| t == &elem) {
                    Some(Type::List(Box::new(elem)))
                } else {
                    None
                }
            }
            Pattern::Cons(head, tail) => {
                let head_ty = Self::extract_declared_type(head)?;
                match Self::extract_declared_type(tail)? {
                    Type::List(elem_ty) if *elem_ty == head_ty => {
                        Some(Type::List(Box::new(head_ty)))
                    }
                    _ => None,
                }
            }
            Pattern::False | Pattern::True => Some(Type::Bool),
            Pattern::Unit => Some(Type::Unit),
            Pattern::Int(_) | Pattern::Succ(_) => Some(Type::Nat),
            _ => None,
        }
    }

    fn is_catch_all(pattern: &Pattern) -> bool {
        match pattern {
            Pattern::Var(_) => true,
            Pattern::Asc(inner, _) | Pattern::CastAs(inner, _) => Self::is_catch_all(inner),
            _ => false,
        }
    }

    fn strip_asc(pattern: &Pattern) -> &Pattern {
        match pattern {
            Pattern::Asc(inner, _) | Pattern::CastAs(inner, _) => Self::strip_asc(inner),
            _ => pattern,
        }
    }

    fn check_match_exhaustiveness(&mut self, scrutinee_ty: &Type, patterns: &[&Pattern]) {
        let matrix: Vec<Vec<Pattern>> = patterns.iter().map(|p| vec![(*p).clone()]).collect();
        if let Some(witness) = Self::find_missing_witness(&matrix, &[scrutinee_ty.clone()]) {
            self.errors
                .push(TypeError::NonexhaustiveMatchPatterns { missing: witness });
        }
    }

    fn check_let_exhaustiveness(&mut self, pattern: &Pattern, expr_ty: &Type) {
        let matrix: Vec<Vec<Pattern>> = vec![vec![pattern.clone()]];
        if let Some(witness) = Self::find_missing_witness(&matrix, &[expr_ty.clone()]) {
            self.errors
                .push(TypeError::NonexhaustiveLetPatterns { missing: witness });
        }
    }

    fn check_letrec_exhaustiveness(&mut self, pattern: &Pattern, expr_ty: &Type) {
        let matrix: Vec<Vec<Pattern>> = vec![vec![pattern.clone()]];
        if let Some(witness) = Self::find_missing_witness(&matrix, &[expr_ty.clone()]) {
            self.errors
                .push(TypeError::NonexhaustiveLetRecPatterns { missing: witness });
        }
    }

    fn find_missing_witness(matrix: &[Vec<Pattern>], types: &[Type]) -> Option<Vec<String>> {
        let mut reversed_witness = Vec::new();
        if Self::find_missing_witness_rev(matrix, types, &mut reversed_witness) {
            reversed_witness.reverse();
            Some(reversed_witness)
        } else {
            None
        }
    }

    fn find_missing_witness_rev(
        matrix: &[Vec<Pattern>],
        types: &[Type],
        reversed_witness: &mut Vec<String>,
    ) -> bool {
        if types.is_empty() {
            return matrix.is_empty();
        }
        if matrix.is_empty() {
            reversed_witness.extend(types.iter().rev().map(|_| "_".to_string()));
            return true;
        }
        if matrix
            .iter()
            .any(|row| row.iter().all(|p| Self::is_catch_all(p)))
        {
            return false;
        }

        let first_ty = &types[0];
        let rest = &types[1..];

        match first_ty {
            Type::Bool => {
                let checkpoint = reversed_witness.len();
                if Self::find_missing_witness_rev(
                    &Self::spec_bool(matrix, true),
                    rest,
                    reversed_witness,
                ) {
                    reversed_witness.push("true".to_string());
                    return true;
                }
                reversed_witness.truncate(checkpoint);

                if Self::find_missing_witness_rev(
                    &Self::spec_bool(matrix, false),
                    rest,
                    reversed_witness,
                ) {
                    reversed_witness.push("false".to_string());
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Nat => {
                let checkpoint = reversed_witness.len();
                if Self::find_missing_witness_rev(
                    &Self::spec_nat_zero(matrix),
                    rest,
                    reversed_witness,
                ) {
                    reversed_witness.push("0".to_string());
                    return true;
                }
                reversed_witness.truncate(checkpoint);

                let succ_types: Vec<Type> = std::iter::once(Type::Nat)
                    .chain(rest.iter().cloned())
                    .collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_nat_succ(matrix),
                    &succ_types,
                    reversed_witness,
                ) {
                    let Some(inner) = reversed_witness.pop() else {
                        return false;
                    };
                    reversed_witness.push(format!("succ({})", inner));
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Unit => {
                let checkpoint = reversed_witness.len();
                if Self::find_missing_witness_rev(&Self::spec_unit(matrix), rest, reversed_witness)
                {
                    reversed_witness.push("unit".to_string());
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Sum(l_ty, r_ty) => {
                let checkpoint = reversed_witness.len();

                let inl_types: Vec<Type> = std::iter::once(*l_ty.clone())
                    .chain(rest.iter().cloned())
                    .collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_inl(matrix),
                    &inl_types,
                    reversed_witness,
                ) {
                    let Some(inner) = reversed_witness.pop() else {
                        return false;
                    };
                    reversed_witness.push(format!("inl({})", inner));
                    return true;
                }
                reversed_witness.truncate(checkpoint);

                let inr_types: Vec<Type> = std::iter::once(*r_ty.clone())
                    .chain(rest.iter().cloned())
                    .collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_inr(matrix),
                    &inr_types,
                    reversed_witness,
                ) {
                    let Some(inner) = reversed_witness.pop() else {
                        return false;
                    };
                    reversed_witness.push(format!("inr({})", inner));
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Tuple(elem_types) => {
                let checkpoint = reversed_witness.len();
                let arity = elem_types.len();
                let new_types: Vec<Type> = elem_types.iter().chain(rest.iter()).cloned().collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_tuple(matrix, arity),
                    &new_types,
                    reversed_witness,
                ) {
                    let mut elems = Vec::with_capacity(arity);
                    for _ in 0..arity {
                        let Some(elem) = reversed_witness.pop() else {
                            return false;
                        };
                        elems.push(elem);
                    }
                    reversed_witness.push(format!("{{{}}}", elems.join(", ")));
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Record(field_types) => {
                let checkpoint = reversed_witness.len();
                let arity = field_types.len();
                let new_types: Vec<Type> = field_types
                    .iter()
                    .map(|f| f.ty.clone())
                    .chain(rest.iter().cloned())
                    .collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_record(matrix, field_types),
                    &new_types,
                    reversed_witness,
                ) {
                    let mut vals = Vec::with_capacity(arity);
                    for _ in 0..arity {
                        let Some(val) = reversed_witness.pop() else {
                            return false;
                        };
                        vals.push(val);
                    }
                    let field_strs: Vec<String> = field_types
                        .iter()
                        .zip(vals)
                        .map(|(ft, val)| format!("{} = {}", ft.name, val))
                        .collect();
                    reversed_witness.push(format!("{{{}}}", field_strs.join(", ")));
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::List(elem_ty) => {
                let checkpoint = reversed_witness.len();
                if Self::find_missing_witness_rev(
                    &Self::spec_list_nil(matrix),
                    rest,
                    reversed_witness,
                ) {
                    reversed_witness.push("[]".to_string());
                    return true;
                }
                reversed_witness.truncate(checkpoint);

                let cons_types: Vec<Type> = [*elem_ty.clone(), Type::List(elem_ty.clone())]
                    .iter()
                    .chain(rest.iter())
                    .cloned()
                    .collect();
                if Self::find_missing_witness_rev(
                    &Self::spec_list_cons(matrix),
                    &cons_types,
                    reversed_witness,
                ) {
                    let Some(head) = reversed_witness.pop() else {
                        return false;
                    };
                    let Some(tail) = reversed_witness.pop() else {
                        return false;
                    };
                    reversed_witness.push(format!("cons({}, {})", head, tail));
                    return true;
                }
                reversed_witness.truncate(checkpoint);
                false
            }
            Type::Variant(variants) => {
                let checkpoint = reversed_witness.len();
                for v in variants {
                    let rows = Self::spec_variant(matrix, &v.name, v.ty.is_some());
                    let inner_types: Vec<Type> = if let Some(inner_ty) = &v.ty {
                        std::iter::once(inner_ty.clone())
                            .chain(rest.iter().cloned())
                            .collect()
                    } else {
                        rest.to_vec()
                    };
                    if Self::find_missing_witness_rev(&rows, &inner_types, reversed_witness) {
                        if v.ty.is_some() {
                            let Some(inner) = reversed_witness.pop() else {
                                return false;
                            };
                            reversed_witness.push(format!("<| {} = {} |>", v.name, inner));
                        } else {
                            reversed_witness.push(format!("<| {} |>", v.name));
                        }
                        return true;
                    }
                    reversed_witness.truncate(checkpoint);
                }
                false
            }
            _ => false,
        }
    }

    fn spec_bool(matrix: &[Vec<Pattern>], is_true: bool) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            let stripped = Self::strip_asc(first);
            if Self::is_catch_all(first)
                || matches!(stripped, Pattern::True if is_true)
                || matches!(stripped, Pattern::False if !is_true)
            {
                out.push(row[1..].to_vec());
            }
        }
        out
    }

    fn spec_nat_zero(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) || matches!(Self::strip_asc(first), Pattern::Int(0)) {
                out.push(row[1..].to_vec());
            }
        }
        out
    }

    fn spec_nat_succ(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(row.len());
                new_row.push(Pattern::Var("_".to_string()));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            match Self::strip_asc(first) {
                Pattern::Succ(inner) => {
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(*inner.clone());
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                Pattern::Int(k) if *k > 0 => {
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(Pattern::Int(*k - 1));
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                _ => {}
            }
        }
        out
    }

    fn spec_unit(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) || matches!(Self::strip_asc(first), Pattern::Unit) {
                out.push(row[1..].to_vec());
            }
        }
        out
    }

    fn spec_inl(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(row.len());
                new_row.push(Pattern::Var("_".to_string()));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let Pattern::Inl(inner) = Self::strip_asc(first) {
                let mut new_row = Vec::with_capacity(row.len());
                new_row.push(*inner.clone());
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
            }
        }
        out
    }

    fn spec_inr(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(row.len());
                new_row.push(Pattern::Var("_".to_string()));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let Pattern::Inr(inner) = Self::strip_asc(first) {
                let mut new_row = Vec::with_capacity(row.len());
                new_row.push(*inner.clone());
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
            }
        }
        out
    }

    fn spec_tuple(matrix: &[Vec<Pattern>], arity: usize) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(arity + row.len().saturating_sub(1));
                new_row.extend((0..arity).map(|_| Pattern::Var("_".to_string())));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let Pattern::Tuple(pats) = Self::strip_asc(first) {
                let mut new_row = Vec::with_capacity(pats.len() + row.len().saturating_sub(1));
                new_row.extend(pats.iter().cloned());
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
            }
        }
        out
    }

    fn spec_record(matrix: &[Vec<Pattern>], field_types: &[RecordFieldType]) -> Vec<Vec<Pattern>> {
        let arity = field_types.len();
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(arity + row.len().saturating_sub(1));
                new_row.extend((0..arity).map(|_| Pattern::Var("_".to_string())));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let Pattern::Record(labelled_pats) = Self::strip_asc(first) {
                let mut new_row = Vec::with_capacity(arity + row.len().saturating_sub(1));
                for ft in field_types {
                    let pat = labelled_pats
                        .iter()
                        .find(|lp| lp.label == ft.name)
                        .map(|lp| lp.pattern.clone())
                        .unwrap_or_else(|| Pattern::Var("_".to_string()));
                    new_row.push(pat);
                }
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
            }
        }
        out
    }

    fn spec_list_nil(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first)
                || matches!(Self::strip_asc(first), Pattern::List(pats) if pats.is_empty())
            {
                out.push(row[1..].to_vec());
            }
        }
        out
    }

    fn spec_list_cons(matrix: &[Vec<Pattern>]) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                let mut new_row = Vec::with_capacity(row.len() + 1);
                new_row.push(Pattern::Var("_".to_string()));
                new_row.push(Pattern::Var("_".to_string()));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            match Self::strip_asc(first) {
                Pattern::Cons(h, t) => {
                    let mut new_row = Vec::with_capacity(row.len() + 1);
                    new_row.push(*h.clone());
                    new_row.push(*t.clone());
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                Pattern::List(pats) if !pats.is_empty() => {
                    let mut new_row = Vec::with_capacity(row.len() + 1);
                    new_row.push(pats[0].clone());
                    new_row.push(Pattern::List(pats[1..].to_vec()));
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                _ => {}
            }
        }
        out
    }

    fn spec_variant(matrix: &[Vec<Pattern>], label: &str, has_payload: bool) -> Vec<Vec<Pattern>> {
        let mut out = Vec::with_capacity(matrix.len());
        for row in matrix {
            let first = &row[0];
            if Self::is_catch_all(first) {
                if has_payload {
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(Pattern::Var("_".to_string()));
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                } else {
                    out.push(row[1..].to_vec());
                }
                continue;
            }

            if let Pattern::Variant { label: l, data } = Self::strip_asc(first) {
                if l != label {
                    continue;
                }
                if has_payload {
                    let inner = data
                        .as_ref()
                        .map(|d| *d.clone())
                        .unwrap_or_else(|| Pattern::Var("_".to_string()));
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(inner);
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                } else {
                    out.push(row[1..].to_vec());
                }
            }
        }
        out
    }

    fn pattern_bound_names(pattern: &Pattern) -> Vec<String> {
        match pattern {
            Pattern::Var(name) => vec![name.clone()],
            Pattern::Asc(inner, _) | Pattern::CastAs(inner, _) => Self::pattern_bound_names(inner),
            Pattern::Tuple(patterns) | Pattern::List(patterns) => patterns
                .iter()
                .flat_map(Self::pattern_bound_names)
                .collect(),
            Pattern::Cons(head, tail) => {
                let mut names = Self::pattern_bound_names(head);
                names.extend(Self::pattern_bound_names(tail));
                names
            }
            Pattern::Record(lps) => lps
                .iter()
                .flat_map(|lp| Self::pattern_bound_names(&lp.pattern))
                .collect(),
            Pattern::Inl(inner) | Pattern::Inr(inner) | Pattern::Succ(inner) => {
                Self::pattern_bound_names(inner)
            }
            Pattern::Variant { data, .. } => {
                data.as_deref().map_or(vec![], Self::pattern_bound_names)
            }
            Pattern::True | Pattern::False | Pattern::Unit | Pattern::Int(_) => vec![],
        }
    }

    fn extend_ctx_by_pattern(&mut self, ctx: &mut Context, pattern: &Pattern, ty: &Type) {
        match pattern {
            Pattern::Var(name) => {
                ctx.extend(name.clone(), ty.clone());
            }

            Pattern::Asc(inner, ascribed_ty) => {
                self.assert_types_equal(ty, ascribed_ty);
                self.extend_ctx_by_pattern(ctx, inner, ascribed_ty);
            }

            Pattern::CastAs(inner, target_ty) => {
                self.extend_ctx_by_pattern(ctx, inner, target_ty);
            }

            Pattern::Tuple(patterns) => match ty {
                Type::Tuple(elem_types) => {
                    for (p, t) in patterns.iter().zip(elem_types) {
                        self.extend_ctx_by_pattern(ctx, p, t);
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "tuple".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Record(labelled_patterns) => match ty {
                Type::Record(field_types) => {
                    let mut seen = HashMap::new();
                    for lp in labelled_patterns {
                        if seen.insert(lp.label.clone(), ()).is_some() {
                            self.errors.push(TypeError::DuplicateRecordPatternFields {
                                field: lp.label.clone(),
                            });
                            continue;
                        }
                        if let Some(ft) = field_types.iter().find(|f| f.name == lp.label) {
                            self.extend_ctx_by_pattern(ctx, &lp.pattern, &ft.ty);
                        } else {
                            self.errors.push(TypeError::UnexpectedFieldAccess {
                                field: lp.label.clone(),
                                record_type: ty.clone(),
                            });
                        }
                    }

                    let pattern_labels: HashSet<&str> = labelled_patterns
                        .iter()
                        .map(|lp| lp.label.as_str())
                        .collect();
                    if field_types
                        .iter()
                        .any(|ft| !pattern_labels.contains(ft.name.as_str()))
                    {
                        self.errors.push(TypeError::UnexpectedPatternForType {
                            pattern_desc: "record".to_string(),
                            scrutinee_type: ty.clone(),
                        });
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "record".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::List(patterns) => match ty {
                Type::List(elem_ty) => {
                    for p in patterns {
                        self.extend_ctx_by_pattern(ctx, p, elem_ty);
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "list".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Cons(head_pat, tail_pat) => match ty {
                Type::List(elem_ty) => {
                    self.extend_ctx_by_pattern(ctx, head_pat, elem_ty);
                    self.extend_ctx_by_pattern(ctx, tail_pat, ty);
                }
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "cons".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Inl(inner) => match ty {
                Type::Sum(left, _) => self.extend_ctx_by_pattern(ctx, inner, left),
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "inl".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Inr(inner) => match ty {
                Type::Sum(_, right) => self.extend_ctx_by_pattern(ctx, inner, right),
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "inr".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Variant { label, data } => match ty {
                Type::Variant(variants) => {
                    if let Some(vt) = variants.iter().find(|v| v.name == *label) {
                        match (&vt.ty, data) {
                            (Some(_), None) => {
                                self.errors
                                    .push(TypeError::UnexpectedNullaryVariantPattern {
                                        label: label.clone(),
                                    })
                            }
                            (None, Some(_)) => {
                                self.errors
                                    .push(TypeError::UnexpectedNonNullaryVariantPattern {
                                        label: label.clone(),
                                    })
                            }
                            (Some(inner_ty), Some(inner_pat)) => {
                                self.extend_ctx_by_pattern(ctx, inner_pat, inner_ty);
                            }
                            (None, None) => {}
                        }
                    } else {
                        self.errors.push(TypeError::UnexpectedVariantLabel {
                            label: label.clone(),
                            variant_type: ty.clone(),
                        });
                    }
                }
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "variant".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::Succ(inner) => match ty {
                Type::Nat => self.extend_ctx_by_pattern(ctx, inner, &Type::Nat),
                _ => self.errors.push(TypeError::UnexpectedPatternForType {
                    pattern_desc: "succ".to_string(),
                    scrutinee_type: ty.clone(),
                }),
            },

            Pattern::True | Pattern::False => {
                if !matches!(ty, Type::Bool) {
                    self.errors.push(TypeError::UnexpectedPatternForType {
                        pattern_desc: "bool literal".to_string(),
                        scrutinee_type: ty.clone(),
                    });
                }
            }

            Pattern::Unit => {
                if !matches!(ty, Type::Unit) {
                    self.errors.push(TypeError::UnexpectedPatternForType {
                        pattern_desc: "unit".to_string(),
                        scrutinee_type: ty.clone(),
                    });
                }
            }

            Pattern::Int(_) => {
                if !matches!(ty, Type::Nat) {
                    self.errors.push(TypeError::UnexpectedPatternForType {
                        pattern_desc: "nat literal".to_string(),
                        scrutinee_type: ty.clone(),
                    });
                }
            }
        }
    }

    pub fn check_program(mut self, prog: &Program) -> Vec<TypeError> {
        let mut ctx = Context::new();

        Self::extend_ctx(&prog.decls, &mut ctx);

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
