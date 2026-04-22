use crate::ast::{
    Decl, DeclFun, DeclFunGeneric, Expr, ExprKind, ParamDecl, Pattern, PatternKind, Program,
    RecordFieldType, ReturnType, Spanned, StellaExtension, Type, VariantFieldType,
};
use crate::type_error::{TypeCheckError, TypeError};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct Context {
    vars: HashMap<String, Type>,
    pub exception_type: Option<Type>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extend(&mut self, name: impl Into<String>, ty: Type) -> Option<Type> {
        self.vars.insert(name.into(), ty)
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.vars.get(name)
    }
}

#[derive(Default)]
pub struct TypeChecker {
    errors: Vec<TypeCheckError>,
    current_function: Option<String>,
    src: Rc<str>,
    extensions: HashSet<StellaExtension>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }

    fn push_error(&mut self, error: TypeError) {
        self.errors.push(TypeCheckError {
            error,
            in_function: self.current_function.clone(),
            src: self.src.clone(),
        });
    }

    fn types_equal(t1: &Type, t2: &Type) -> bool {
        t1 == &Type::Auto || t2 == &Type::Auto || t1 == t2
    }

    pub fn assert_types_equal(&mut self, expected: &Type, got: &Type) {
        if !Self::types_equal(expected, got) {
            self.push_error(TypeError::UnexpectedTypeForExpression {
                expected: expected.clone(),
                got: got.clone(),
                expr_span: None,
            });
        }
    }

    pub fn assert_types_equal_for_expr(&mut self, expected: &Type, got: &Type, expr: &Expr) {
        if !Self::types_equal(expected, got) {
            self.push_error(TypeError::UnexpectedTypeForExpression {
                expected: expected.clone(),
                got: got.clone(),
                expr_span: Some(expr.span),
            });
        }
    }

    pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
        if sub == sup {
            return true;
        }
        match (sub, sup) {
            (Type::Auto, _) | (_, Type::Auto) => true,
            (Type::Bottom, _) => true,
            (_, Type::Top) => true,
            (Type::Fun(sub_params, sub_ret), Type::Fun(sup_params, sup_ret)) => {
                sub_params.len() == sup_params.len()
                    && sup_params
                        .iter()
                        .zip(sub_params.iter())
                        .all(|(sup_p, sub_p)| Self::is_subtype(sup_p, sub_p))
                    && Self::is_subtype(sub_ret, sup_ret)
            }
            (Type::Tuple(sub_elems), Type::Tuple(sup_elems)) => {
                sub_elems.len() == sup_elems.len()
                    && sub_elems
                        .iter()
                        .zip(sup_elems.iter())
                        .all(|(s, t)| Self::is_subtype(s, t))
            }
            (Type::Record(sub_fields), Type::Record(sup_fields)) => sup_fields.iter().all(|sf| {
                sub_fields
                    .iter()
                    .find(|f| f.name == sf.name)
                    .is_some_and(|f| Self::is_subtype(&f.ty, &sf.ty))
            }),
            (Type::Variant(sub_labels), Type::Variant(sup_labels)) => sub_labels.iter().all(|sl| {
                sup_labels
                    .iter()
                    .find(|l| l.name == sl.name)
                    .is_some_and(|l| match (&sl.ty, &l.ty) {
                        (None, None) => true,
                        (Some(st), Some(lt)) => Self::is_subtype(st, lt),
                        _ => false,
                    })
            }),
            (Type::List(sub_elem), Type::List(sup_elem)) => Self::is_subtype(sub_elem, sup_elem),
            (Type::Sum(sl, sr), Type::Sum(tl, tr)) => {
                Self::is_subtype(sl, tl) && Self::is_subtype(sr, tr)
            }
            // References are invariant: &T <: &U iff T <: U and U <: T
            (Type::Ref(a), Type::Ref(b)) => Self::is_subtype(a, b) && Self::is_subtype(b, a),
            _ => false,
        }
    }

    pub fn assert_is_assignable_at(&mut self, expected: &Type, got: &Type, span: crate::ast::Span) {
        if self
            .extensions
            .contains(&StellaExtension::StructuralSubtyping)
        {
            if !Self::is_subtype(got, expected) {
                match (got, expected) {
                    (Type::Record(got_fields), Type::Record(exp_fields)) => {
                        let missing: Vec<String> = exp_fields
                            .iter()
                            .filter(|ef| !got_fields.iter().any(|gf| gf.name == ef.name))
                            .map(|ef| ef.name.clone())
                            .collect();
                        if !missing.is_empty() {
                            self.push_error(TypeError::MissingRecordFields {
                                missing,
                                expr_span: span,
                            });
                            return;
                        }
                    }

                    (Type::Tuple(got_elems), Type::Tuple(exp_elems))
                        if got_elems.len() != exp_elems.len() =>
                    {
                        self.push_error(TypeError::UnexpectedTupleLength {
                            expected: exp_elems.len(),
                            got: got_elems.len(),
                            expr_span: span,
                        });
                        return;
                    }

                    (Type::Variant(got_labels), Type::Variant(exp_labels)) => {
                        if let Some(extra) = got_labels
                            .iter()
                            .find(|gl| !exp_labels.iter().any(|el| el.name == gl.name))
                        {
                            self.push_error(TypeError::UnexpectedVariantLabel {
                                label: extra.name.clone(),
                                variant_type: expected.clone(),
                                expr_span: Some(span),
                            });
                            return;
                        }
                    }

                    (Type::Fun(got_params, _), Type::Fun(exp_params, _))
                        if got_params.len() != exp_params.len() =>
                    {
                        self.push_error(TypeError::IncorrectNumberOfArguments {
                            expected: exp_params.len(),
                            got: got_params.len(),
                            expr_span: span,
                        });
                        return;
                    }
                    _ => {}
                }
                self.push_error(TypeError::UnexpectedSubtype {
                    expected: expected.clone(),
                    got: got.clone(),
                    expr_span: Some(span),
                });
            }
        } else if !Self::types_equal(expected, got) {
            self.push_error(TypeError::UnexpectedTypeForExpression {
                expected: expected.clone(),
                got: got.clone(),
                expr_span: Some(span),
            });
        }
    }

    pub fn assert_is_assignable(&mut self, expected: &Type, got: &Type, expr: &Expr) {
        self.assert_is_assignable_at(expected, got, expr.span);
    }

    pub fn infer(&mut self, ctx: &Context, expr: &Expr) -> Option<Type> {
        match &expr.node {
            ExprKind::ConstTrue | ExprKind::ConstFalse => Some(Type::Bool),
            ExprKind::ConstUnit => Some(Type::Unit),
            ExprKind::ConstInt(_) => Some(Type::Nat),

            ExprKind::Var(_name) => ctx.lookup(_name).cloned().or_else(|| {
                self.push_error(TypeError::UndefinedVariable {
                    name: _name.clone(),
                    expr_span: expr.span,
                });
                None
            }),

            ExprKind::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                let then_ty = self.infer(ctx, then_)?;
                self.check(ctx, else_, &then_ty);
                Some(then_ty)
            }

            ExprKind::TypeAsc(_e, _ty) => {
                self.check(ctx, _e, _ty);
                Some(_ty.as_ref().clone())
            }

            ExprKind::TypeCast(_e, _ty) => {
                self.infer(ctx, _e);
                Some(_ty.as_ref().clone())
            }

            ExprKind::Sequence(e1, e2) => {
                self.check(ctx, e1, &Type::Unit);
                self.infer(ctx, e2)
            }

            ExprKind::Let(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.push_error(TypeError::DuplicateLetBinding {
                                name,
                                expr_span: expr.span,
                            });
                        }
                    }
                    if let Some(ty) = self.infer(ctx, &binding.expr) {
                        self.check_let_exhaustiveness(&binding.pattern, &ty, &binding.expr);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.infer(&local_ctx, body)
            }
            ExprKind::LetRec(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.push_error(TypeError::DuplicateLetBinding {
                                name,
                                expr_span: expr.span,
                            });
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
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty, &binding.expr);
                        self.check(&local_ctx, &binding.expr, &ty);
                    } else {
                        self.push_error(TypeError::AmbiguousPatternType {
                            pat_span: binding.pattern.span,
                        });
                    }
                }
                self.infer(&local_ctx, body)
            }

            ExprKind::Abstraction { params, body } => {
                let mut local_ctx = ctx.clone();
                let mut seen_names = HashSet::new();
                let param_types = params
                    .iter()
                    .map(|p| {
                        if !seen_names.insert(p.name.clone()) {
                            self.push_error(TypeError::DuplicateFunctionParameter {
                                name: p.name.clone(),
                            });
                        }
                        if p.ty == Type::Auto {
                            self.push_error(TypeError::UnexpectedTypeForParameter {
                                param: p.name.clone(),
                                expected: Type::Auto,
                                got: Type::Auto,
                                expr_span: expr.span,
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
            ExprKind::Application { func, args } => {
                let func_ty = self.infer(ctx, func)?;
                match &func_ty {
                    Type::Fun(param_types, return_type) if param_types.len() == args.len() => {
                        for (arg, param_ty) in args.iter().zip(param_types) {
                            self.check(ctx, arg, param_ty);
                        }
                        Some(return_type.as_ref().clone())
                    }
                    Type::Fun(param_types, _) => {
                        self.push_error(TypeError::IncorrectNumberOfArguments {
                            expected: param_types.len(),
                            got: args.len(),
                            expr_span: expr.span,
                        });
                        None
                    }
                    _ => {
                        self.push_error(TypeError::NotAFunction {
                            ty: func_ty,
                            expr_span: func.span,
                        });
                        None
                    }
                }
            }

            ExprKind::Add(left, right)
            | ExprKind::Subtract(left, right)
            | ExprKind::Multiply(left, right)
            | ExprKind::Divide(left, right) => {
                self.check(ctx, left, &Type::Nat);
                self.check(ctx, right, &Type::Nat);
                Some(Type::Nat)
            }

            ExprKind::LessThan(left, right)
            | ExprKind::LessThanOrEqual(left, right)
            | ExprKind::GreaterThan(left, right)
            | ExprKind::GreaterThanOrEqual(left, right)
            | ExprKind::Equal(left, right)
            | ExprKind::NotEqual(left, right) => {
                self.check(ctx, left, &Type::Nat);
                self.check(ctx, right, &Type::Nat);
                Some(Type::Bool)
            }

            ExprKind::LogicAnd(left, right) | ExprKind::LogicOr(left, right) => {
                self.check(ctx, left, &Type::Bool);
                self.check(ctx, right, &Type::Bool);
                Some(Type::Bool)
            }
            ExprKind::LogicNot(e) => {
                self.check(ctx, e, &Type::Bool);
                Some(Type::Bool)
            }

            ExprKind::Succ(e) | ExprKind::Pred(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Nat)
            }
            ExprKind::IsZero(e) => {
                self.check(ctx, e, &Type::Nat);
                Some(Type::Bool)
            }
            ExprKind::NatRec(_n, _z, _s) => {
                self.check(ctx, _n, &Type::Nat);
                let z_ty = self.infer(ctx, _z)?;
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

            ExprKind::Fix(_f) => {
                let f_ty = self.infer(ctx, _f)?;
                match &f_ty {
                    Type::Fun(param_types, return_type) if param_types.len() == 1 => {
                        let param_ty = &param_types[0];
                        self.assert_types_equal(param_ty, return_type.as_ref());
                        Some(param_ty.clone())
                    }
                    Type::Fun(param_types, _) => {
                        self.push_error(TypeError::IncorrectNumberOfArguments {
                            expected: 1,
                            got: param_types.len(),
                            expr_span: expr.span,
                        });
                        None
                    }
                    _ => {
                        self.push_error(TypeError::NotAFunction {
                            ty: f_ty,
                            expr_span: _f.span,
                        });
                        None
                    }
                }
            }

            ExprKind::Tuple(exprs) => {
                let elem_types = exprs
                    .iter()
                    .map(|e| self.infer(ctx, e))
                    .collect::<Option<Vec<Type>>>()?;
                Some(Type::Tuple(elem_types))
            }

            ExprKind::DotTuple(e, index) => {
                if *index < 1 {
                    self.push_error(TypeError::TupleIndexOutOfBounds {
                        index: *index,
                        length: 0,
                        expr_span: e.span,
                    });
                    return None;
                }
                let tuple_ty = self.infer(ctx, e)?;
                match tuple_ty {
                    Type::Tuple(elem_types) if *index - 1 < elem_types.len() => {
                        Some(elem_types[*index - 1].clone())
                    }
                    Type::Tuple(elem_types) => {
                        self.push_error(TypeError::TupleIndexOutOfBounds {
                            index: *index,
                            length: elem_types.len(),
                            expr_span: e.span,
                        });
                        None
                    }
                    _ => {
                        self.push_error(TypeError::NotATuple {
                            ty: tuple_ty,
                            expr_span: e.span,
                        });
                        None
                    }
                }
            }

            ExprKind::Record(bindings) => {
                let mut seen = HashMap::new();
                let mut field_types = Vec::new();
                for binding in bindings {
                    if seen.insert(binding.name.clone(), ()).is_some() {
                        self.push_error(TypeError::DuplicateRecordFields {
                            field: binding.name.clone(),
                            expr_span: expr.span,
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

            ExprKind::DotRecord(expr, field) => match self.infer(ctx, expr) {
                Some(Type::Record(field_types)) => {
                    if let Some(ft) = field_types.iter().find(|f| f.name == *field) {
                        Some(ft.ty.clone())
                    } else {
                        self.push_error(TypeError::UnexpectedFieldAccess {
                            field: field.clone(),
                            record_type: Type::Record(field_types),
                            expr_span: Some(expr.span),
                        });
                        None
                    }
                }
                Some(record_ty) => {
                    self.push_error(TypeError::NotARecord {
                        ty: record_ty,
                        expr_span: expr.span,
                    });
                    None
                }
                None => None,
            },

            ExprKind::Inl(inner) => {
                if self
                    .extensions
                    .contains(&StellaExtension::AmbiguousTypeAsBottom)
                {
                    let inner_ty = self.infer(ctx, inner)?;
                    Some(Type::Sum(Box::new(inner_ty), Box::new(Type::Bottom)))
                } else {
                    self.push_error(TypeError::AmbiguousSumType {
                        expr_span: expr.span,
                    });
                    None
                }
            }

            ExprKind::Inr(inner) => {
                if self
                    .extensions
                    .contains(&StellaExtension::AmbiguousTypeAsBottom)
                {
                    let inner_ty = self.infer(ctx, inner)?;
                    Some(Type::Sum(Box::new(Type::Bottom), Box::new(inner_ty)))
                } else {
                    self.push_error(TypeError::AmbiguousSumType {
                        expr_span: expr.span,
                    });
                    None
                }
            }

            ExprKind::Variant { label: _, payload } => {
                if self
                    .extensions
                    .contains(&StellaExtension::AmbiguousTypeAsBottom)
                {
                    // type-check the payload for side effects, then return Bottom
                    if let Some(p) = payload {
                        self.infer(ctx, p);
                    }
                    Some(Type::Bottom)
                } else {
                    self.push_error(TypeError::AmbiguousVariantType {
                        expr_span: expr.span,
                    });
                    None
                }
            }

            ExprKind::List(exprs) => {
                if exprs.is_empty() {
                    if self
                        .extensions
                        .contains(&StellaExtension::AmbiguousTypeAsBottom)
                    {
                        Some(Type::List(Box::new(Type::Bottom)))
                    } else {
                        self.push_error(TypeError::AmbiguousList {
                            expr_span: expr.span,
                        });
                        None
                    }
                } else {
                    let ty = self.infer(ctx, &exprs[0])?;
                    exprs.iter().skip(1).for_each(|e| self.check(ctx, e, &ty));
                    Some(Type::List(Box::new(ty)))
                }
            }
            ExprKind::ConsList(head, tail) => {
                let elem_ty = self.infer(ctx, head)?;
                self.check(ctx, tail, &Type::List(Box::new(elem_ty.clone())));
                Some(Type::List(Box::new(elem_ty)))
            }
            ExprKind::Head(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(elem_ty) => Some(*elem_ty),
                    a => {
                        self.push_error(TypeError::NotAList {
                            ty: a,
                            expr_span: e.span,
                        });
                        None
                    }
                }
            }
            ExprKind::Tail(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(elem_ty) => Some(Type::List(elem_ty)),
                    a => {
                        self.push_error(TypeError::NotAList {
                            ty: a,
                            expr_span: e.span,
                        });
                        None
                    }
                }
            }
            ExprKind::IsEmpty(e) => {
                let list_ty = self.infer(ctx, e)?;
                match list_ty {
                    Type::List(_) => Some(Type::Bool),
                    a => {
                        self.push_error(TypeError::NotAList {
                            ty: a,
                            expr_span: e.span,
                        });
                        None
                    }
                }
            }
            ExprKind::Match { expr, cases } => {
                let scrutinee_ty = self.infer(ctx, expr)?;
                if cases.is_empty() {
                    self.push_error(TypeError::IllegalEmptyMatching {
                        expr_span: expr.span,
                    });
                    return None;
                }
                let errors_before = self.errors.len();
                let mut local_ctxs = Vec::new();
                for match_case in cases.iter() {
                    let mut local_ctx = ctx.clone();
                    self.extend_ctx_by_pattern(&mut local_ctx, &match_case.pattern, &scrutinee_ty);
                    local_ctxs.push(local_ctx);
                }
                let pattern_errors = self.errors.len() > errors_before;
                let mut case_type_exprs: Vec<(Type, &Expr)> = Vec::new();
                for (match_case, local_ctx) in cases.iter().zip(local_ctxs.iter()) {
                    if let Some(case_ty) = self.infer(local_ctx, &match_case.expr) {
                        case_type_exprs.push((case_ty, &match_case.expr));
                    }
                }
                if !pattern_errors {
                    let patterns: Vec<&Pattern> = cases.iter().map(|c| &c.pattern).collect();
                    self.check_match_exhaustiveness(&scrutinee_ty, &patterns, expr);
                }
                if let Some((first_case_ty, _)) = case_type_exprs.first() {
                    let first_case_ty = first_case_ty.clone();
                    for (case_ty, case_expr) in &case_type_exprs[1..] {
                        self.assert_is_assignable(&first_case_ty, case_ty, case_expr);
                    }
                    Some(first_case_ty)
                } else {
                    None
                }
            }
            ExprKind::Panic => {
                if self
                    .extensions
                    .contains(&StellaExtension::AmbiguousTypeAsBottom)
                {
                    Some(Type::Bottom)
                } else {
                    self.push_error(TypeError::AmbiguousPanicType {
                        expr_span: expr.span,
                    });
                    None
                }
            }

            ExprKind::Throw(e) => {
                match ctx.exception_type.clone() {
                    Some(exn_ty) => {
                        self.check(ctx, e, &exn_ty);
                        if self
                            .extensions
                            .contains(&StellaExtension::AmbiguousTypeAsBottom)
                        {
                            return Some(Type::Bottom);
                        }
                        self.push_error(TypeError::AmbiguousThrowType {
                            expr_span: expr.span,
                        });
                    }
                    None => {
                        self.push_error(TypeError::ExceptionTypeNotDeclared {
                            expr_span: expr.span,
                        });
                    }
                }
                None
            }

            ExprKind::TryCatch {
                try_expr,
                pattern,
                catch_expr,
            } => {
                let result_ty = self.infer(ctx, try_expr)?;
                let exn_ty = match ctx.exception_type.clone() {
                    Some(t) => t,
                    None => {
                        self.push_error(TypeError::ExceptionTypeNotDeclared {
                            expr_span: expr.span,
                        });
                        return None;
                    }
                };
                let mut catch_ctx = ctx.clone();
                self.extend_ctx_by_pattern(&mut catch_ctx, pattern, &exn_ty);
                let catch_ty = self.infer(&catch_ctx, catch_expr)?;
                self.assert_is_assignable(&result_ty, &catch_ty, catch_expr);
                Some(result_ty)
            }

            ExprKind::TryWith {
                try_expr,
                with_expr,
            } => {
                let result_ty = self.infer(ctx, try_expr)?;
                self.check(ctx, with_expr, &result_ty);
                Some(result_ty)
            }

            ExprKind::Ref(e) => {
                let inner_ty = self.infer(ctx, e)?;
                Some(Type::Ref(Box::new(inner_ty)))
            }

            ExprKind::Deref(e) => {
                let ref_ty = self.infer(ctx, e)?;
                match ref_ty {
                    Type::Ref(inner) => Some(*inner),
                    ty => {
                        self.push_error(TypeError::NotAReference {
                            ty,
                            expr_span: e.span,
                        });
                        None
                    }
                }
            }

            ExprKind::Assign(lhs, rhs) => {
                let lhs_ty = self.infer(ctx, lhs)?;
                match lhs_ty {
                    Type::Ref(inner) => {
                        self.check(ctx, rhs, &inner);
                        Some(Type::Unit)
                    }
                    ty => {
                        self.push_error(TypeError::NotAReference {
                            ty,
                            expr_span: lhs.span,
                        });
                        None
                    }
                }
            }

            ExprKind::ConstMemory(_) => {
                if self
                    .extensions
                    .contains(&StellaExtension::AmbiguousTypeAsBottom)
                {
                    Some(Type::Ref(Box::new(Type::Bottom)))
                } else {
                    self.push_error(TypeError::AmbiguousReferenceType {
                        expr_span: expr.span,
                    });
                    None
                }
            }

            ExprKind::TryCastAs {
                try_expr,
                ty,
                pattern,
                catch_expr,
                with_expr,
            } => {
                self.infer(ctx, try_expr);
                let mut catch_ctx = ctx.clone();
                self.extend_ctx_by_pattern(&mut catch_ctx, pattern, ty);
                let catch_ty = self.infer(&catch_ctx, catch_expr)?;
                self.check(ctx, with_expr, &catch_ty);
                Some(catch_ty)
            }

            _ => unimplemented!(),
        }
    }

    pub fn check(&mut self, ctx: &Context, expr: &Expr, expected: &Type) {
        match &expr.node {
            ExprKind::If { cond, then_, else_ } => {
                self.check(ctx, cond, &Type::Bool);
                self.check(ctx, then_, expected);
                self.check(ctx, else_, expected);
            }

            ExprKind::Abstraction { params, body } => match expected {
                Type::Fun(param_types, return_type) => {
                    if param_types.len() != params.len() {
                        self.push_error(TypeError::UnexpectedNumberOfParametersInLambda {
                            expected: param_types.len(),
                            got: params.len(),
                            expr_span: expr.span,
                        });
                    }
                    let mut local_ctx = ctx.clone();
                    let mut seen_names = HashSet::new();
                    for (p, expected_param_ty) in params.iter().zip(param_types) {
                        if !seen_names.insert(p.name.clone()) {
                            self.push_error(TypeError::DuplicateFunctionParameter {
                                name: p.name.clone(),
                            });
                        }
                        if self
                            .extensions
                            .contains(&StellaExtension::StructuralSubtyping)
                        {
                            // Contravariant check: expected_param_ty <: declared p.ty
                            self.assert_is_assignable_at(
                                &p.ty,
                                expected_param_ty,
                                expr.span,
                            );
                        } else if !Self::types_equal(&p.ty, expected_param_ty) {
                            self.push_error(TypeError::UnexpectedTypeForParameter {
                                param: p.name.clone(),
                                expected: expected_param_ty.clone(),
                                got: p.ty.clone(),
                                expr_span: expr.span,
                            });
                        }
                        local_ctx.extend(p.name.clone(), expected_param_ty.clone());
                    }
                    self.check(&local_ctx, body, return_type);
                }
                _ => self.push_error(TypeError::UnexpectedLambda {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },

            ExprKind::Application { func, args } => match self.infer(ctx, func) {
                Some(Type::Fun(param_types, return_type)) => {
                    if param_types.len() != args.len() {
                        self.push_error(TypeError::IncorrectNumberOfArguments {
                            expected: param_types.len(),
                            got: args.len(),
                            expr_span: expr.span,
                        });
                    } else {
                        for (arg, param_ty) in args.iter().zip(param_types) {
                            self.check(ctx, arg, &param_ty);
                        }
                    }
                    self.assert_is_assignable(expected, return_type.as_ref(), expr);
                }
                Some(func_ty) => self.push_error(TypeError::NotAFunction {
                    ty: func_ty,
                    expr_span: func.span,
                }),
                None => self.push_error(TypeError::AmbiguousFunction {
                    expr_span: func.span,
                }),
            },

            ExprKind::Tuple(exprs) => match expected {
                Type::Tuple(elem_types) => {
                    if elem_types.len() != exprs.len() {
                        self.push_error(TypeError::UnexpectedTupleLength {
                            expected: elem_types.len(),
                            got: exprs.len(),
                            expr_span: expr.span,
                        });
                    }
                    for (e, ty) in exprs.iter().zip(elem_types) {
                        self.check(ctx, e, ty);
                    }
                }
                _ => self.push_error(TypeError::UnexpectedTuple {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },

            ExprKind::DotTuple(e, index) => {
                if *index < 1 {
                    self.push_error(TypeError::TupleIndexOutOfBounds {
                        index: *index,
                        length: 0,
                        expr_span: e.span,
                    });
                    return;
                }
                match self.infer(ctx, e) {
                    Some(Type::Tuple(elem_types)) => {
                        if *index - 1 < elem_types.len() {
                            self.assert_is_assignable(expected, &elem_types[*index - 1], expr);
                        } else {
                            self.push_error(TypeError::TupleIndexOutOfBounds {
                                index: *index,
                                length: elem_types.len(),
                                expr_span: e.span,
                            });
                        }
                    }
                    Some(tuple_ty) => self.push_error(TypeError::NotATuple {
                        ty: tuple_ty,
                        expr_span: e.span,
                    }),
                    None => self.push_error(TypeError::AmbiguousTuple { expr_span: e.span }),
                }
            }

            ExprKind::Record(bindings) => match expected {
                Type::Record(field_types) => {
                    let mut seen = HashMap::new();
                    for b in bindings {
                        if seen.insert(b.name.clone(), ()).is_some() {
                            self.push_error(TypeError::DuplicateRecordFields {
                                field: b.name.clone(),
                                expr_span: expr.span,
                            });
                        }
                    }
                    seen.clear();
                    for f in field_types {
                        if seen.insert(f.name.clone(), ()).is_some() {
                            self.push_error(TypeError::DuplicateRecordTypeFields {
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
                        self.push_error(TypeError::MissingRecordFields {
                            missing,
                            expr_span: expr.span,
                        });
                    }
                    let unexpected: Vec<String> = binding_names
                        .iter()
                        .filter(|n| !expected_names.contains(n))
                        .map(|n| n.to_string())
                        .collect();
                    // With structural subtyping, extra fields on the literal are fine
                    if !unexpected.is_empty()
                        && !self
                            .extensions
                            .contains(&StellaExtension::StructuralSubtyping)
                    {
                        self.push_error(TypeError::UnexpectedRecordFields {
                            unexpected,
                            expr_span: expr.span,
                        });
                    }
                    for binding in bindings {
                        if let Some(ft) = field_types.iter().find(|f| f.name == binding.name) {
                            self.check(ctx, &binding.expr, &ft.ty);
                        }
                    }
                }
                _ => self.push_error(TypeError::UnexpectedRecord {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },

            ExprKind::Variant { label, payload } => match expected {
                Type::Variant(variants) => {
                    let mut seen = HashMap::new();
                    for v in variants {
                        if seen.insert(v.name.clone(), ()).is_some() {
                            self.push_error(TypeError::DuplicateVariantTypeFields {
                                label: v.name.clone(),
                            });
                        }
                    }
                    if let Some(vt) = variants.iter().find(|v| v.name == *label) {
                        match (&vt.ty, payload) {
                            (None, Some(_)) => {
                                self.push_error(TypeError::UnexpectedDataForNullaryLabel {
                                    label: label.clone(),
                                    expr_span: expr.span,
                                })
                            }
                            (Some(_), None) => self.push_error(TypeError::MissingDataForLabel {
                                label: label.clone(),
                                expr_span: expr.span,
                            }),
                            (None, None) => {}
                            (Some(ty), Some(inner_expr)) => self.check(ctx, inner_expr, ty),
                        }
                    } else {
                        self.push_error(TypeError::UnexpectedVariantLabel {
                            label: label.clone(),
                            variant_type: expected.clone(),
                            expr_span: Some(expr.span),
                        });
                    }
                }
                _ => self.push_error(TypeError::UnexpectedVariant {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },

            ExprKind::Inl(inner) => match expected {
                Type::Sum(left, _) => self.check(ctx, inner, left),
                _ => self.push_error(TypeError::UnexpectedInjection {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },
            ExprKind::Inr(inner) => match expected {
                Type::Sum(_, right) => self.check(ctx, inner, right),
                _ => self.push_error(TypeError::UnexpectedInjection {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },

            ExprKind::List(exprs) => match expected {
                Type::List(elem_ty) => {
                    for e in exprs {
                        self.check(ctx, e, elem_ty);
                    }
                }
                _ => self.push_error(TypeError::UnexpectedList {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },
            ExprKind::ConsList(head, tail) => match expected {
                Type::List(elem_ty) => {
                    self.check(ctx, head, elem_ty);
                    self.check(ctx, tail, expected);
                }
                _ => self.push_error(TypeError::UnexpectedList {
                    expected: expected.clone(),
                    expr_span: expr.span,
                }),
            },
            ExprKind::IsEmpty(e) => {
                self.assert_types_equal(expected, &Type::Bool);
                match self.infer(ctx, e) {
                    Some(Type::List(_)) => {}
                    Some(a) => self.push_error(TypeError::NotAList {
                        ty: a,
                        expr_span: e.span,
                    }),
                    None => self.push_error(TypeError::AmbiguousList { expr_span: e.span }),
                }
            }
            ExprKind::Match { expr, cases } => {
                if let Some(scrutinee_ty) = self.infer(ctx, expr) {
                    if cases.is_empty() {
                        self.push_error(TypeError::IllegalEmptyMatching {
                            expr_span: expr.span,
                        });
                    } else {
                        let errors_before = self.errors.len();
                        let mut local_ctxs = Vec::new();
                        for match_case in cases.iter() {
                            let mut local_ctx = ctx.clone();
                            self.extend_ctx_by_pattern(
                                &mut local_ctx,
                                &match_case.pattern,
                                &scrutinee_ty,
                            );
                            local_ctxs.push(local_ctx);
                        }
                        if self.errors.len() == errors_before {
                            let patterns: Vec<&Pattern> =
                                cases.iter().map(|c| &c.pattern).collect();
                            self.check_match_exhaustiveness(&scrutinee_ty, &patterns, expr);
                        }
                        for (match_case, local_ctx) in cases.iter().zip(local_ctxs.iter()) {
                            self.check(local_ctx, &match_case.expr, expected);
                        }
                    }
                }
            }
            ExprKind::Sequence(e1, e2) => {
                self.check(ctx, e1, &Type::Unit);
                self.check(ctx, e2, expected);
            }

            ExprKind::Let(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.push_error(TypeError::DuplicateLetBinding {
                                name,
                                expr_span: expr.span,
                            });
                        }
                    }
                    if let Some(ty) = self.infer(ctx, &binding.expr) {
                        self.check_let_exhaustiveness(&binding.pattern, &ty, &binding.expr);
                        self.extend_ctx_by_pattern(&mut local_ctx, &binding.pattern, &ty);
                    }
                }
                self.check(&local_ctx, body, expected);
            }
            ExprKind::LetRec(bindings, body) => {
                let mut local_ctx = ctx.clone();
                let mut seen_names: HashSet<String> = HashSet::new();
                for binding in bindings {
                    for name in Self::pattern_bound_names(&binding.pattern) {
                        if !seen_names.insert(name.clone()) {
                            self.push_error(TypeError::DuplicateLetBinding {
                                name,
                                expr_span: expr.span,
                            });
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
                        self.check_letrec_exhaustiveness(&binding.pattern, &ty, &binding.expr);
                        self.check(&local_ctx, &binding.expr, &ty);
                    } else {
                        self.push_error(TypeError::AmbiguousPatternType {
                            pat_span: binding.pattern.span,
                        });
                    }
                }
                self.check(&local_ctx, body, expected);
            }

            ExprKind::Panic => {}

            ExprKind::Throw(e) => match ctx.exception_type.clone() {
                Some(exn_ty) => {
                    self.check(ctx, e, &exn_ty);
                }
                None => {
                    self.push_error(TypeError::ExceptionTypeNotDeclared {
                        expr_span: expr.span,
                    });
                }
            },

            ExprKind::TryCatch {
                try_expr,
                pattern,
                catch_expr,
            } => {
                let exn_ty = ctx.exception_type.clone();
                self.check(ctx, try_expr, expected);
                match exn_ty {
                    Some(exn_ty) => {
                        let mut catch_ctx = ctx.clone();
                        self.extend_ctx_by_pattern(&mut catch_ctx, pattern, &exn_ty);
                        self.check(&catch_ctx, catch_expr, expected);
                    }
                    None => {
                        self.push_error(TypeError::ExceptionTypeNotDeclared {
                            expr_span: expr.span,
                        });
                    }
                }
            }

            ExprKind::TryWith {
                try_expr,
                with_expr,
            } => {
                self.check(ctx, try_expr, expected);
                self.check(ctx, with_expr, expected);
            }

            ExprKind::Ref(e) => match expected {
                Type::Ref(inner_ty) => {
                    self.check(ctx, e, inner_ty);
                }
                _ => {
                    self.push_error(TypeError::UnexpectedReference {
                        expected: expected.clone(),
                        expr_span: expr.span,
                    });
                }
            },

            ExprKind::Deref(e) => {
                if matches!(e.node, ExprKind::ConstMemory(_)) {
                    return;
                }
                let saved_errors = self.errors.len();
                if let Some(ref_ty) = self.infer(ctx, e) {
                    match ref_ty {
                        Type::Ref(inner) => {
                            self.assert_is_assignable(expected, &inner, expr);
                        }
                        Type::Tuple(_) => {
                            self.push_error(TypeError::UnexpectedTuple {
                                expected: expected.clone(),
                                expr_span: expr.span,
                            });
                        }
                        ty => {
                            self.push_error(TypeError::UnexpectedTypeForExpression {
                                expected: Type::Ref(Box::new(expected.clone())),
                                got: ty,
                                expr_span: Some(e.span),
                            });
                        }
                    }
                } else {
                    // infer failed (e.g. ConstMemory inside if-branches without
                    // AmbiguousTypeAsBottom); roll back any errors pushed by infer
                    // and re-check e in check mode so that context propagates down.
                    self.errors.truncate(saved_errors);
                    self.check(ctx, e, &Type::Ref(Box::new(expected.clone())));
                }
            }

            ExprKind::ConstMemory(_) => match expected {
                Type::Ref(_) => {}
                _ => {
                    self.push_error(TypeError::UnexpectedMemoryAddress {
                        expected: expected.clone(),
                        expr_span: expr.span,
                    });
                }
            },

            ExprKind::TryCastAs {
                try_expr,
                ty,
                pattern,
                catch_expr,
                with_expr,
            } => {
                self.infer(ctx, try_expr);
                let mut catch_ctx = ctx.clone();
                self.extend_ctx_by_pattern(&mut catch_ctx, pattern, ty);
                self.check(&catch_ctx, catch_expr, expected);
                self.check(ctx, with_expr, expected);
            }

            _ => {
                if let Some(got) = self.infer(ctx, expr) {
                    self.assert_is_assignable(expected, &got, expr);
                }
            }
        }
    }

    fn check_decl(&mut self, ctx: &Context, decl: &Decl) {
        match decl {
            Decl::Fun(f) => self.check_fun(ctx, f),
            Decl::FunGeneric(f) => self.check_fun_generic(ctx, f),
            Decl::TypeAlias { .. } => {}
            Decl::ExceptionType { .. } => {
                if self.current_function.is_some() {
                    self.push_error(TypeError::IllegalLocalExceptionType);
                }
            }
            Decl::ExceptionVariant { .. } => {
                if self.current_function.is_some() {
                    self.push_error(TypeError::IllegalLocalOpenVariantException);
                }
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
                self.push_error(TypeError::DuplicateFunctionParameter {
                    name: p.name.clone(),
                });
                continue;
            }
            self.validate_type(&p.ty);
            local_ctx.extend(p.name.clone(), p.ty.clone());
        }
        self.validate_type(return_type.as_type());
        self.extend_ctx(local_decls, &mut local_ctx);

        for decl in local_decls {
            self.check_decl(&local_ctx, decl);
        }
        self.check(&local_ctx, body, return_type.as_type());
    }

    fn validate_type(&mut self, ty: &Type) {
        match ty {
            Type::Record(fields) => {
                let mut seen = HashMap::new();
                for f in fields {
                    if seen.insert(f.name.clone(), ()).is_some() {
                        self.push_error(TypeError::DuplicateRecordTypeFields {
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
                        self.push_error(TypeError::DuplicateVariantTypeFields {
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
        let prev_fn = self.current_function.replace(f.name.clone());
        self.check_fun_body(ctx, &f.params, &f.return_type, &f.local_decls, &f.body);
        self.current_function = prev_fn;
    }

    fn check_fun_generic(&mut self, ctx: &Context, f: &DeclFunGeneric) {
        let prev_fn = self.current_function.replace(f.name.clone());
        for type_param in &f.type_params {
            if f.type_params.iter().filter(|tp| *tp == type_param).count() > 1 {
                self.push_error(TypeError::DuplicateTypeParameter {
                    name: type_param.clone(),
                });
            }
        }
        self.check_fun_body(ctx, &f.params, &f.return_type, &f.local_decls, &f.body);
        self.current_function = prev_fn;
    }

    fn extend_ctx(&mut self, decls: &[Decl], ctx: &mut Context) {
        let mut exception_type_decl: Option<&Type> = None;
        let mut exception_type_count = 0usize;
        let mut variant_decls: Vec<(&String, &Type)> = Vec::new();

        for decl in decls {
            match decl {
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
                Decl::ExceptionType { ty } => {
                    exception_type_count += 1;
                    if exception_type_decl.is_none() {
                        exception_type_decl = Some(ty);
                    }
                }
                Decl::ExceptionVariant { name, ty } => {
                    variant_decls.push((name, ty));
                }
            }
        }

        let has_type_decl = exception_type_count > 0;
        let has_variant_decls = !variant_decls.is_empty();

        if has_type_decl && has_variant_decls {
            self.push_error(TypeError::ConflictingExceptionDeclarations);
        } else if exception_type_count > 1 {
            self.push_error(TypeError::DuplicateExceptionType);
        } else if has_variant_decls {
            let mut seen: HashSet<&str> = HashSet::new();
            for (name, _) in &variant_decls {
                if !seen.insert(name.as_str()) {
                    self.push_error(TypeError::DuplicateExceptionVariant {
                        label: name.to_string(),
                    });
                    break;
                }
            }
        }

        if let Some(ty) = exception_type_decl {
            ctx.exception_type = Some(ty.clone());
        } else if has_variant_decls {
            let variant_fields = variant_decls
                .into_iter()
                .map(|(name, ty)| VariantFieldType {
                    name: name.clone(),
                    ty: Some(ty.clone()),
                })
                .collect();
            ctx.exception_type = Some(Type::Variant(variant_fields));
        }
    }

    fn extract_declared_type(pattern: &Pattern) -> Option<Type> {
        match &pattern.node {
            PatternKind::Asc(_, ty) | PatternKind::CastAs(_, ty) => Some(*ty.clone()),
            PatternKind::Variant { label, data } => {
                let field = VariantFieldType {
                    name: label.clone(),
                    ty: data.as_deref().and_then(Self::extract_declared_type),
                };
                Some(Type::Variant(vec![field]))
            }
            PatternKind::Inl(inner) => {
                let _ = Self::extract_declared_type(inner)?;
                None
            }
            PatternKind::Inr(inner) => {
                let _ = Self::extract_declared_type(inner)?;
                None
            }
            PatternKind::Tuple(pats) => {
                let types = pats
                    .iter()
                    .map(Self::extract_declared_type)
                    .collect::<Option<Vec<_>>>()?;
                Some(Type::Tuple(types))
            }
            PatternKind::Record(fields) => {
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
            PatternKind::List(pats) => {
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
            PatternKind::Cons(head, tail) => {
                let head_ty = Self::extract_declared_type(head)?;
                match Self::extract_declared_type(tail)? {
                    Type::List(elem_ty) if *elem_ty == head_ty => {
                        Some(Type::List(Box::new(head_ty)))
                    }
                    _ => None,
                }
            }
            PatternKind::False | PatternKind::True => Some(Type::Bool),
            PatternKind::Unit => Some(Type::Unit),
            PatternKind::Int(_) | PatternKind::Succ(_) => Some(Type::Nat),
            _ => None,
        }
    }

    fn is_catch_all(pattern: &Pattern) -> bool {
        match &pattern.node {
            PatternKind::Var(_) => true,
            PatternKind::Asc(inner, _) => Self::is_catch_all(inner),
            _ => false,
        }
    }

    fn strip_asc(pattern: &Pattern) -> &Pattern {
        match &pattern.node {
            PatternKind::Asc(inner, _) | PatternKind::CastAs(inner, _) => Self::strip_asc(inner),
            _ => pattern,
        }
    }

    fn check_match_exhaustiveness(
        &mut self,
        scrutinee_ty: &Type,
        patterns: &[&Pattern],
        scrutinee: &Expr,
    ) {
        let matrix: Vec<Vec<Pattern>> = patterns.iter().map(|p| vec![(*p).clone()]).collect();
        if let Some(witness) = Self::find_missing_witness(&matrix, &[scrutinee_ty.clone()]) {
            self.push_error(TypeError::NonexhaustiveMatchPatterns {
                missing: witness,
                expr_span: scrutinee.span,
            });
        }
    }

    fn check_let_exhaustiveness(&mut self, pattern: &Pattern, expr_ty: &Type, binding_expr: &Expr) {
        let matrix: Vec<Vec<Pattern>> = vec![vec![pattern.clone()]];
        if let Some(witness) = Self::find_missing_witness(&matrix, &[expr_ty.clone()]) {
            self.push_error(TypeError::NonexhaustiveLetPatterns {
                missing: witness,
                expr_span: binding_expr.span,
            });
        }
    }

    fn check_letrec_exhaustiveness(
        &mut self,
        pattern: &Pattern,
        expr_ty: &Type,
        binding_expr: &Expr,
    ) {
        let matrix: Vec<Vec<Pattern>> = vec![vec![pattern.clone()]];
        if let Some(witness) = Self::find_missing_witness(&matrix, &[expr_ty.clone()]) {
            self.push_error(TypeError::NonexhaustiveLetRecPatterns {
                missing: witness,
                expr_span: binding_expr.span,
            });
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
        if matrix.iter().any(|row| row.iter().all(Self::is_catch_all)) {
            return false;
        }

        let first_ty = &types[0];
        let rest = &types[1..];

        if matrix.iter().all(|row| Self::is_catch_all(&row[0])) {
            let reduced_matrix: Vec<Vec<Pattern>> =
                matrix.iter().map(|row| row[1..].to_vec()).collect();
            if Self::find_missing_witness_rev(&reduced_matrix, rest, reversed_witness) {
                reversed_witness.push("_".to_string());
                return true;
            }
            return false;
        }

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
            Type::Top => {
                reversed_witness.push("_".to_string());
                true
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
                || matches!(&stripped.node, PatternKind::True if is_true)
                || matches!(&stripped.node, PatternKind::False if !is_true)
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
            if Self::is_catch_all(first)
                || matches!(&Self::strip_asc(first).node, PatternKind::Int(0))
            {
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
                new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            match &Self::strip_asc(first).node {
                PatternKind::Succ(inner) => {
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(*inner.clone());
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                PatternKind::Int(k) if *k > 0 => {
                    let mut new_row = Vec::with_capacity(row.len());
                    new_row.push(Spanned::new(PatternKind::Int(*k - 1), 0, 0));
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
            if Self::is_catch_all(first)
                || matches!(&Self::strip_asc(first).node, PatternKind::Unit)
            {
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
                new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let PatternKind::Inl(inner) = &Self::strip_asc(first).node {
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
                new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let PatternKind::Inr(inner) = &Self::strip_asc(first).node {
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
                new_row.extend(
                    (0..arity).map(|_| Spanned::new(PatternKind::Var("_".to_string()), 0, 0)),
                );
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let PatternKind::Tuple(pats) = &Self::strip_asc(first).node {
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
                new_row.extend(
                    (0..arity).map(|_| Spanned::new(PatternKind::Var("_".to_string()), 0, 0)),
                );
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            if let PatternKind::Record(labelled_pats) = &Self::strip_asc(first).node {
                let mut new_row = Vec::with_capacity(arity + row.len().saturating_sub(1));
                for ft in field_types {
                    let pat = labelled_pats
                        .iter()
                        .find(|lp| lp.label == ft.name)
                        .map(|lp| lp.pattern.clone())
                        .unwrap_or_else(|| Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
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
                || matches!(&Self::strip_asc(first).node, PatternKind::List(pats) if pats.is_empty())
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
                new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                new_row.extend(row[1..].iter().cloned());
                out.push(new_row);
                continue;
            }
            match &Self::strip_asc(first).node {
                PatternKind::Cons(h, t) => {
                    let mut new_row = Vec::with_capacity(row.len() + 1);
                    new_row.push(*h.clone());
                    new_row.push(*t.clone());
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                }
                PatternKind::List(pats) if !pats.is_empty() => {
                    let mut new_row = Vec::with_capacity(row.len() + 1);
                    new_row.push(pats[0].clone());
                    new_row.push(Spanned::new(PatternKind::List(pats[1..].to_vec()), 0, 0));
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
                    new_row.push(Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
                    new_row.extend(row[1..].iter().cloned());
                    out.push(new_row);
                } else {
                    out.push(row[1..].to_vec());
                }
                continue;
            }

            if let PatternKind::Variant { label: l, data } = &Self::strip_asc(first).node {
                if l != label {
                    continue;
                }
                if has_payload {
                    let inner = data
                        .as_ref()
                        .map(|d| *d.clone())
                        .unwrap_or_else(|| Spanned::new(PatternKind::Var("_".to_string()), 0, 0));
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
        match &pattern.node {
            PatternKind::Var(name) => vec![name.clone()],
            PatternKind::Asc(inner, _) | PatternKind::CastAs(inner, _) => {
                Self::pattern_bound_names(inner)
            }
            PatternKind::Tuple(patterns) | PatternKind::List(patterns) => patterns
                .iter()
                .flat_map(Self::pattern_bound_names)
                .collect(),
            PatternKind::Cons(head, tail) => {
                let mut names = Self::pattern_bound_names(head);
                names.extend(Self::pattern_bound_names(tail));
                names
            }
            PatternKind::Record(lps) => lps
                .iter()
                .flat_map(|lp| Self::pattern_bound_names(&lp.pattern))
                .collect(),
            PatternKind::Inl(inner) | PatternKind::Inr(inner) | PatternKind::Succ(inner) => {
                Self::pattern_bound_names(inner)
            }
            PatternKind::Variant { data, .. } => {
                data.as_deref().map_or(vec![], Self::pattern_bound_names)
            }
            PatternKind::True | PatternKind::False | PatternKind::Unit | PatternKind::Int(_) => {
                vec![]
            }
        }
    }

    fn extend_ctx_by_pattern(&mut self, ctx: &mut Context, pattern: &Pattern, ty: &Type) {
        match &pattern.node {
            PatternKind::Var(name) => {
                ctx.extend(name.clone(), ty.clone());
            }

            PatternKind::Asc(inner, ascribed_ty) => {
                self.assert_is_assignable_at(ascribed_ty, ty, pattern.span);
                self.extend_ctx_by_pattern(ctx, inner, ascribed_ty);
            }

            PatternKind::CastAs(inner, target_ty) => {
                if self
                    .extensions
                    .contains(&StellaExtension::StructuralSubtyping)
                    && !Self::is_subtype(target_ty, ty)
                {
                    self.push_error(TypeError::UnexpectedSubtype {
                        expected: ty.clone(),
                        got: *target_ty.clone(),
                        expr_span: Some(pattern.span),
                    });
                }
                self.extend_ctx_by_pattern(ctx, inner, target_ty);
            }

            PatternKind::Tuple(patterns) => match ty {
                Type::Tuple(elem_types) => {
                    if patterns.len() != elem_types.len() {
                        self.push_error(TypeError::UnexpectedPatternForType {
                            pattern_desc: "tuple".to_string(),
                            scrutinee_type: ty.clone(),
                            pat_span: pattern.span,
                        });
                    } else {
                        for (p, t) in patterns.iter().zip(elem_types) {
                            self.extend_ctx_by_pattern(ctx, p, t);
                        }
                    }
                }
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "tuple".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Record(labelled_patterns) => match ty {
                Type::Record(field_types) => {
                    let mut seen = HashMap::new();
                    for lp in labelled_patterns {
                        if seen.insert(lp.label.clone(), ()).is_some() {
                            self.push_error(TypeError::DuplicateRecordPatternFields {
                                field: lp.label.clone(),
                                pat_span: pattern.span,
                            });
                            continue;
                        }
                        if let Some(ft) = field_types.iter().find(|f| f.name == lp.label) {
                            self.extend_ctx_by_pattern(ctx, &lp.pattern, &ft.ty);
                        } else {
                            self.push_error(TypeError::UnexpectedFieldAccess {
                                field: lp.label.clone(),
                                record_type: ty.clone(),
                                expr_span: None,
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
                        self.push_error(TypeError::UnexpectedPatternForType {
                            pattern_desc: "record".to_string(),
                            scrutinee_type: ty.clone(),
                            pat_span: pattern.span,
                        });
                    }
                }
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "record".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::List(patterns) => match ty {
                Type::List(elem_ty) => {
                    for p in patterns {
                        self.extend_ctx_by_pattern(ctx, p, elem_ty);
                    }
                }
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "list".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Cons(head_pat, tail_pat) => match ty {
                Type::List(elem_ty) => {
                    self.extend_ctx_by_pattern(ctx, head_pat, elem_ty);
                    self.extend_ctx_by_pattern(ctx, tail_pat, ty);
                }
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "cons".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Inl(inner) => match ty {
                Type::Sum(left, _) => self.extend_ctx_by_pattern(ctx, inner, left),
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "inl".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Inr(inner) => match ty {
                Type::Sum(_, right) => self.extend_ctx_by_pattern(ctx, inner, right),
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "inr".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Variant { label, data } => match ty {
                Type::Variant(variants) => {
                    if let Some(vt) = variants.iter().find(|v| v.name == *label) {
                        match (&vt.ty, data) {
                            (Some(_), None) => {
                                self.push_error(TypeError::UnexpectedNullaryVariantPattern {
                                    label: label.clone(),
                                    pat_span: pattern.span,
                                })
                            }
                            (None, Some(_)) => {
                                self.push_error(TypeError::UnexpectedNonNullaryVariantPattern {
                                    label: label.clone(),
                                    pat_span: pattern.span,
                                })
                            }
                            (Some(inner_ty), Some(inner_pat)) => {
                                self.extend_ctx_by_pattern(ctx, inner_pat, inner_ty);
                            }
                            (None, None) => {}
                        }
                    } else {
                        self.push_error(TypeError::UnexpectedPatternForType {
                            pattern_desc: format!("variant label `{}`", label),
                            scrutinee_type: ty.clone(),
                            pat_span: pattern.span,
                        });
                    }
                }
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "variant".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::Succ(inner) => match ty {
                Type::Nat => self.extend_ctx_by_pattern(ctx, inner, &Type::Nat),
                _ => self.push_error(TypeError::UnexpectedPatternForType {
                    pattern_desc: "succ".to_string(),
                    scrutinee_type: ty.clone(),
                    pat_span: pattern.span,
                }),
            },

            PatternKind::True | PatternKind::False => {
                if !matches!(ty, Type::Bool) {
                    self.push_error(TypeError::UnexpectedPatternForType {
                        pattern_desc: "bool literal".to_string(),
                        scrutinee_type: ty.clone(),
                        pat_span: pattern.span,
                    });
                }
            }

            PatternKind::Unit => {
                if !matches!(ty, Type::Unit) {
                    self.push_error(TypeError::UnexpectedPatternForType {
                        pattern_desc: "unit".to_string(),
                        scrutinee_type: ty.clone(),
                        pat_span: pattern.span,
                    });
                }
            }

            PatternKind::Int(_) => {
                if !matches!(ty, Type::Nat) {
                    self.push_error(TypeError::UnexpectedPatternForType {
                        pattern_desc: "nat literal".to_string(),
                        scrutinee_type: ty.clone(),
                        pat_span: pattern.span,
                    });
                }
            }
        }
    }

    pub fn check_program(mut self, prog: &Program, src: &str) -> Vec<TypeCheckError> {
        self.src = Rc::from(src);

        self.extensions = prog
            .extensions
            .iter()
            .flat_map(|ext| ext.features.iter().cloned())
            .collect();

        let mut ctx = Context::new();
        self.extend_ctx(&prog.decls, &mut ctx);

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
            self.push_error(TypeError::MissingMain);
        }
    }

    fn check_main_arity(&mut self, prog: &Program) {
        if let Some(Decl::Fun(f)) = prog
            .decls
            .iter()
            .find(|d| matches!(d, Decl::Fun(f) if f.name == "main" && f.params.len() != 1))
        {
            self.push_error(TypeError::IncorrectArityOfMain {
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
                self.push_error(TypeError::DuplicateFunctionDeclaration { name });
            }
        }
    }
}
