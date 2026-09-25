use super::{Checker, Result, Value, dependencies::PointKind};
use crate::ast::{self, ExprKind, Span};
use crate::diagnostic::Diagnostic;
use crate::flow::FALSE;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn expr(&mut self, expr: &ast::Expr, expected: Option<&Type>) -> Result<hir::Expr> {
        self.expr_point(expr, expected).map(|(_, value)| value)
    }

    pub(crate) fn expr_point(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<(hir::PointId, hir::Expr)> {
        self.with_continuation(expr.span, "expression", |checker| {
            let kind = if matches!(expected, Some(Type::Reference(_))) {
                PointKind::Expr
            } else {
                PointKind::expression(expr)
            };
            checker.with_point_id(kind, expr.span, |checker| {
                checker.coerced_expression(expr, expected)
            })
        })
    }

    pub(crate) fn coerced_expression(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        let mut value = if matches!(expected, Some(Type::Reference(_))) {
            self.with_point_id(PointKind::expression(expr), expr.span, |checker| {
                checker.expression_value(expr, expected)
            })?
            .1
        } else {
            self.expression_value(expr, expected)?
        };
        if value.ty == Type::Never {
            self.reach = FALSE;
            return Ok(value);
        }
        if let (Some(Type::Reference(target)), Type::Exclusive(source)) = (expected, &value.ty)
            && target == source
        {
            let site = self.reborrows;
            self.reborrows += 1;
            value = hir::Expr {
                kind: hir::ExprKind::Reborrow {
                    site,
                    value: Box::new(value),
                    fields: Vec::new(),
                },
                ty: expected.unwrap().clone(),
                span: expr.span,
            };
        }
        match expected {
            Some(expected) => Self::expected_value(value, expected, expr.span),
            None => Ok(value),
        }
    }

    pub(crate) fn expected_value(
        value: hir::Expr,
        expected: &Type,
        span: Span,
    ) -> Result<hir::Expr> {
        if value.ty == Type::Never {
            return Ok(value);
        }
        if value.ty != *expected {
            if expected.accepts(&value.ty) {
                return Ok(Self::coerce(value, expected.clone()));
            }
            if let Type::Record { primary, .. } = &value.ty
                && expected.accepts(primary)
                && !matches!(expected, Type::Record { .. })
            {
                let ty = *primary.clone();
                return Ok(Self::coerce(
                    hir::Expr {
                        ty,
                        span,
                        kind: hir::ExprKind::Primary(Box::new(value)),
                    },
                    expected.clone(),
                ));
            }
            return Err(Self::error(
                "E207",
                format!("expected {expected:?}, found {:?}", value.ty),
                span,
            ));
        }
        Ok(value)
    }

    pub(crate) fn expression(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        self.expression_point(expr, expected)
            .map(|(_, value)| value)
    }

    pub(crate) fn expression_point(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<(hir::PointId, hir::Expr)> {
        self.with_continuation(expr.span, "expression", |checker| {
            checker.with_point_id(PointKind::expression(expr), expr.span, |checker| {
                checker.expression_value(expr, expected)
            })
        })
    }

    pub(crate) fn expression_value(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        let value = self.raw_expression(expr, expected)?;
        if value.ty == Type::Bool {
            self.guard(&value);
        }
        if value.ty == Type::Never {
            self.reach = FALSE;
        }
        Ok(value)
    }

    pub(crate) fn raw_expression(
        &mut self,
        expr: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<hir::Expr> {
        let bits = self.bits_expression(expr)?;
        let expr = bits.as_ref().unwrap_or(expr);
        self.charge_integer(expr)?;
        let (kind, ty) = match &expr.kind {
            ExprKind::Int(text) => return self.integer(text, false, expected, expr.span),
            ExprKind::Float(text) => return Self::floating(text, expected, expr.span),
            ExprKind::String(parts) => {
                let mut text = String::new();
                for part in parts {
                    match part {
                        ast::StringPart::Text(value) => text.push_str(value),
                        ast::StringPart::Value(_) => {
                            return Err(Diagnostic::unsupported(
                                "interpolated strings outside debug.print/debug.panic",
                                expr.span,
                            ));
                        }
                    }
                }
                (hir::ExprKind::String(text), Type::String)
            }
            ExprKind::Name(_)
            | ExprKind::Import(_)
            | ExprKind::TypeValue(_)
            | ExprKind::TypeQuery(_) => match self.symbol(expr)?.expect("symbol") {
                Value::Foundation(crate::foundation::Item::Heap) => {
                    return Ok(hir::Expr {
                        kind: hir::ExprKind::Heap,
                        ty: Type::Foundation(hir::FoundationType::Allocator),
                        span: expr.span,
                    });
                }
                Value::Foundation(item) => {
                    return Err(Diagnostic::unsupported(
                        format!("runtime use of `{}`", item.name()),
                        expr.span,
                    ));
                }
                Value::FileModule { id, ty } => {
                    let primary = Self::primary_type(&ty);
                    if self.required
                        && self.proven_inputs()
                        && matches!(primary, Type::Int { .. })
                        && (ty == primary || matches!(expected, Some(Type::Int { .. })))
                    {
                        let input = self.required_primary(id, expr.span)?;
                        if let Some(error) = input.error {
                            return Err(error);
                        }
                        let value = input.value.ok_or_else(|| {
                            Self::error(
                                "E211",
                                "required primary has no checked integer value",
                                expr.span,
                            )
                        })?;
                        return Ok(hir::Expr {
                            kind: hir::ExprKind::Int(value),
                            ty: primary,
                            span: expr.span,
                        });
                    }
                    if self.owner != 0 {
                        return Err(Diagnostic::unsupported(
                            "file-module values in function bodies",
                            expr.span,
                        ));
                    }
                    return Ok(hir::Expr {
                        kind: hir::ExprKind::Local(id),
                        ty,
                        span: expr.span,
                    });
                }
                Value::Local { id, ty, .. } => {
                    if self.required
                        && self.proven_inputs()
                        && let Some(value) = self.inputs.get(&id).and_then(|input| input.value)
                    {
                        return Ok(hir::Expr {
                            kind: hir::ExprKind::Int(value),
                            ty,
                            span: expr.span,
                        });
                    }
                    return self.narrow(hir::Expr {
                        kind: hir::ExprKind::Local(id),
                        ty,
                        span: expr.span,
                    });
                }
                Value::Constant(value) => return Ok(Self::constant_expr(value, expr.span)),
                Value::Static { value, ty } => {
                    let mut value = Self::constant_expr(value, expr.span);
                    value.ty = ty;
                    return Ok(value);
                }
                Value::Pending(_) => {
                    return Err(Self::error(
                        "E223",
                        "proof descriptors have no runtime value",
                        expr.span,
                    ));
                }
                Value::Function { .. } => {
                    return Err(Diagnostic::unsupported(
                        "first-class function values",
                        expr.span,
                    ));
                }
                Value::Type(_) => {
                    return Err(Diagnostic::unsupported(
                        "runtime use of type values",
                        expr.span,
                    ));
                }
                _ => {
                    return Err(Diagnostic::unsupported(
                        "runtime use of compile-time identities",
                        expr.span,
                    ));
                }
            },
            ExprKind::Group(value) => {
                let (child, value) = self.expr_point(value, expected)?;
                if let Some(point) = self.point {
                    self.region_edges(point, child, expr.span)?;
                }
                return Ok(value);
            }
            ExprKind::Unary { op, value } => {
                if op == "-"
                    && let ExprKind::Int(text) = &value.kind
                {
                    return self.integer(text, true, expected, expr.span);
                }
                if op == "&" {
                    return self.borrowed(value, expr.span);
                }
                if op == "&!" {
                    return self.exclusive_borrow(value, expr.span);
                }
                if op == "*" {
                    return self.deref_point(value, expr.span).map(|(_, value)| value);
                }
                if ["&!", ">>", "<<"].contains(&op.as_str()) {
                    return Err(Diagnostic::unsupported(format!("unary `{op}`"), expr.span));
                }
                let (input, value) = self.unary_point(op, value, expected, expr.span)?;
                if let Some(point) = self.point {
                    self.unary_operation(point, input, &value, expr.span)?;
                }
                return Ok(value);
            }
            ExprKind::Binary { op, left, right } => {
                return self.binary(op, left, right, expected, expr.span);
            }
            ExprKind::Call { callee, args } => return self.call(callee, args, None, expr.span),
            ExprKind::Dispatch {
                value,
                callee,
                args,
            } => return self.call(callee, args, Some(value), expr.span),
            ExprKind::DispatchBlock { value, block } => {
                let value = self.expr(value, None)?;
                if value.ty.has_exclusive() {
                    return Err(Diagnostic::unsupported(
                        "exclusive dispatch receivers",
                        expr.span,
                    ));
                }
                let block = self.block(block, expected.cloned(), Some(value))?;
                let ty = block.ty.clone();
                (hir::ExprKind::Block(block), ty)
            }
            ExprKind::Block(block) => {
                let block = self.block(block, expected.cloned(), None)?;
                if let Some(point) = self.point {
                    self.block_result(point, block.id, expr.span)?;
                }
                let ty = block.ty.clone();
                (hir::ExprKind::Block(block), ty)
            }
            ExprKind::Field { value, name } => {
                if self.required && self.proven_inputs() {
                    self.charge_ancestors(expr)?;
                    let (ty, input) = self.required_field(expr)?;
                    if let Some(error) = input.error {
                        return Err(error);
                    }
                    let value = input.value.ok_or_else(|| {
                        Self::error(
                            "E211",
                            "required field has no checked integer value",
                            expr.span,
                        )
                    })?;
                    return Ok(hir::Expr {
                        kind: hir::ExprKind::Int(value),
                        ty,
                        span: expr.span,
                    });
                }
                if let Some(symbol) = self.symbol(expr)? {
                    return match symbol {
                        Value::Foundation(crate::foundation::Item::Heap) => Ok(hir::Expr {
                            kind: hir::ExprKind::Heap,
                            ty: Type::Foundation(hir::FoundationType::Allocator),
                            span: expr.span,
                        }),
                        Value::Foundation(item) => Err(Diagnostic::unsupported(
                            format!("runtime use of `{}`", item.name()),
                            expr.span,
                        )),
                        Value::Static { value, ty } => {
                            let mut value = Self::constant_expr(value, expr.span);
                            value.ty = ty;
                            Ok(value)
                        }
                        Value::Constant(value) => Ok(Self::constant_expr(value, expr.span)),
                        _ => Err(Diagnostic::unsupported(
                            "runtime use of intrinsic operation values",
                            expr.span,
                        )),
                    };
                }
                let mut value = self.expr(value, None)?;
                if let Type::Reference(ty) = &value.ty {
                    value = hir::Expr {
                        ty: *ty.clone(),
                        span: value.span,
                        kind: hir::ExprKind::Deref(Box::new(value)),
                    };
                }
                let Type::Record { fields, .. } = &value.ty else {
                    return Err(Self::error(
                        "E201",
                        format!("type {:?} has no field `{name}`", value.ty),
                        expr.span,
                    ));
                };
                let (index, field) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, field)| &field.name == name)
                    .ok_or_else(|| {
                        Self::error("E201", format!("unknown record field `{name}`"), expr.span)
                    })?;
                let ty = field.ty.clone();
                return self.narrow(hir::Expr {
                    kind: hir::ExprKind::Field {
                        value: Box::new(value),
                        index,
                    },
                    ty,
                    span: expr.span,
                });
            }
            ExprKind::Ascribe {
                value,
                ty,
                predicate,
            } => {
                let value = self.expr(value, None)?;
                let ty = self.construct_type(ty)?;
                if value.ty == Type::Never {
                    return Ok(value);
                }
                if *predicate {
                    return Ok(hir::Expr {
                        kind: hir::ExprKind::TypeTest {
                            value: Box::new(value),
                            ty,
                        },
                        ty: Type::Bool,
                        span: expr.span,
                    });
                }
                if !ty.accepts(&value.ty) {
                    return Err(Self::error(
                        "E208",
                        format!(
                            "value of type {:?} is not proven to have type {ty:?}",
                            value.ty
                        ),
                        expr.span,
                    ));
                }
                return Ok(Self::coerce(value, ty));
            }
            ExprKind::Function { .. } => {
                return Err(Diagnostic::unsupported(
                    "anonymous function values",
                    expr.span,
                ));
            }
            ExprKind::Index { value, index } => return self.list_index(value, index, expr.span),
            ExprKind::List(values) => return self.list_literal(values, expected, expr.span),
            ExprKind::Label(_) => {
                return Err(Self::error(
                    "E201",
                    "labels are control targets, not ordinary values",
                    expr.span,
                ));
            }
            ExprKind::Specialize { .. } => {
                return Err(Diagnostic::unsupported(
                    "generic call specialization",
                    expr.span,
                ));
            }
            ExprKind::Unsupported(feature) => {
                return Err(Diagnostic::unsupported(feature, expr.span));
            }
        };
        Ok(hir::Expr {
            kind,
            ty,
            span: expr.span,
        })
    }

    pub(crate) fn primary_type(ty: &Type) -> Type {
        match ty {
            Type::Record { primary, .. } => Self::primary_type(primary),
            ty => ty.clone(),
        }
    }

    pub(crate) fn project(value: hir::Expr) -> hir::Expr {
        let ty = Self::primary_type(&value.ty);
        if ty == value.ty {
            value
        } else {
            hir::Expr {
                ty,
                span: value.span,
                kind: hir::ExprKind::Primary(Box::new(value)),
            }
        }
    }

    pub(crate) fn hint_symbol(&mut self, expr: &ast::Expr) -> Option<Value> {
        let mut base = expr;
        while let ExprKind::Group(value) | ExprKind::Field { value, .. } = &base.kind {
            base = value;
        }
        if matches!(base.kind, ExprKind::Name(_) | ExprKind::Import(_)) {
            self.symbol(expr).ok().flatten()
        } else {
            None
        }
    }

    pub(crate) fn hint(&mut self, expr: &ast::Expr) -> Option<Type> {
        let bits = self.bits_expression(expr).ok().flatten();
        let expr = bits.as_ref().unwrap_or(expr);
        if matches!(expr.kind, ExprKind::Name(_) | ExprKind::Field { .. })
            && matches!(
                self.hint_symbol(expr),
                Some(Value::Foundation(crate::foundation::Item::Heap))
            )
        {
            return Some(Type::Foundation(hir::FoundationType::Allocator));
        }
        match &expr.kind {
            ExprKind::List(values) => self.list_hint(values),
            ExprKind::Index { value, .. } => {
                let ty = self.hint(value)?;
                let ty = if let Type::Reference(ty) = ty {
                    *ty
                } else {
                    ty
                };
                if let Type::List { element, .. } = ty {
                    Some(*element)
                } else {
                    None
                }
            }
            ExprKind::Name(name) => match self.value(name, expr.span).ok()? {
                Value::Local { id, ty, .. } => Some(self.refined((id, Vec::new()), &ty)),
                value => value.data_type(),
            },
            ExprKind::Group(value) => self.hint(value),
            ExprKind::Unary { op, value } if op == "&" => self
                .address_hint(value)
                .map(|ty| Type::Reference(Box::new(ty))),
            ExprKind::Unary { op, value } if op == "*" => match self.hint(value)? {
                Type::Reference(ty) | Type::Exclusive(ty) => Some(*ty),
                _ => None,
            },
            ExprKind::Unary { value, .. } => self.hint(value),
            ExprKind::Binary { left, right, op } => {
                if ["==", "!=", "<", ">", "<=", ">=", "&&", "||"].contains(&op.as_str()) {
                    Some(Type::Bool)
                } else {
                    self.hint(left)
                        .or_else(|| self.hint(right))
                        .map(|ty| Self::primary_type(&ty))
                }
            }
            ExprKind::String(_) => Some(Type::String),
            ExprKind::Field { value, name } => {
                if ["always", "never", "indeterminable"].contains(&name.as_str())
                    && matches!(self.hint_symbol(value), Some(Value::Pending(_)))
                {
                    return Some(Type::Bool);
                }
                if let Some(Value::Static { ty, .. }) = self.hint_symbol(expr) {
                    return Some(ty);
                }
                let ty = self.hint(value).map(|ty| match ty {
                    Type::Reference(ty) => *ty,
                    ty => ty,
                });
                if let Some(Type::Record { fields, .. }) = ty {
                    let ty = fields
                        .into_iter()
                        .find(|field| &field.name == name)
                        .map(|field| field.ty)?;
                    if let Some(place) = self.ast_place(expr) {
                        Some(self.refined(place, &ty))
                    } else {
                        Some(ty)
                    }
                } else {
                    None
                }
            }
            ExprKind::Call { callee, .. } => {
                if let ExprKind::Name(name) = &callee.kind {
                    match self.value(name, expr.span).ok()? {
                        Value::Function { result, .. } => result,
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub(crate) fn format_parts(
        &mut self,
        expr: &ast::Expr,
        parts: &mut Vec<hir::Expr>,
    ) -> Result<Vec<Option<hir::PointId>>> {
        let mut points = Vec::new();
        self.format_points(expr, parts, &mut points)?;
        Ok(points)
    }

    pub(crate) fn format_points(
        &mut self,
        expr: &ast::Expr,
        parts: &mut Vec<hir::Expr>,
        points: &mut Vec<Option<hir::PointId>>,
    ) -> Result<()> {
        match &expr.kind {
            ExprKind::String(values) => {
                for value in values {
                    match value {
                        ast::StringPart::Text(text) => {
                            parts.push(hir::Expr {
                                kind: hir::ExprKind::String(text.clone()),
                                ty: Type::String,
                                span: expr.span,
                            });
                            points.push(None);
                        }
                        ast::StringPart::Value(value) => {
                            self.format_points(value, parts, points)?
                        }
                    }
                }
            }
            ExprKind::Group(value) => self.format_points(value, parts, points)?,
            _ => {
                let (point, value) = self.expr_point(expr, None)?;
                let value = Self::project(value);
                if value.ty.has_reference() {
                    return Err(Diagnostic::unsupported(
                        "reference formatting; dereference the copyable value",
                        expr.span,
                    ));
                }
                if !Self::value_formattable(&value.ty) {
                    return Err(Diagnostic::unsupported(
                        "bounded-list or foundation value formatting",
                        expr.span,
                    ));
                }
                parts.push(value);
                points.push(Some(point));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod formatting;

#[cfg(test)]
mod roots;
