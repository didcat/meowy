use super::{Checker, Constant, Result, Value, dependencies::PointKind};
use crate::ast::{self, Span};
use crate::flow::FALSE;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn unary_point(
        &mut self,
        op: &str,
        value: &ast::Expr,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(hir::PointId, hir::Expr)> {
        let context = self.unary_context(op, value, expected)?;
        let (point, value) = self.expr_point(value, context.as_ref())?;
        self.unary_value(op, value, span)
            .map(|value| (point, value))
    }

    pub(crate) fn unary_context(
        &mut self,
        op: &str,
        value: &ast::Expr,
        expected: Option<&Type>,
    ) -> Result<Option<Type>> {
        if let Some(ty) = self.hint(value) {
            return Ok(Some(Self::primary_type(&ty)));
        }
        if op == "!" {
            return Ok(None);
        }
        let literal = self.literal_default(value);
        let types: Vec<_> = expected
            .into_iter()
            .flat_map(Type::members)
            .filter(|ty| match (op, ty, &literal) {
                ("-" | "~", Type::Int { .. }, Some(Type::Float { .. })) => false,
                ("-" | "~", Type::Int { .. }, _) => true,
                ("-", Type::Float { .. }, Some(Type::Int { .. })) => false,
                ("-", Type::Float { .. }, _) => true,
                _ => false,
            })
            .cloned()
            .collect();
        if types.len() > 1 {
            return Err(Self::error(
                "E207",
                "unary operand has multiple possible expected types",
                value.span,
            ));
        }
        Ok(types.into_iter().next())
    }

    pub(crate) fn numeric_context(
        expected: Option<&Type>,
        integer: bool,
        span: Span,
    ) -> Result<Option<Type>> {
        let types: Vec<_> = expected
            .into_iter()
            .flat_map(Type::members)
            .filter(|ty| {
                if integer {
                    matches!(ty, Type::Int { .. })
                } else {
                    matches!(ty, Type::Float { .. })
                }
            })
            .cloned()
            .collect();
        if types.len() > 1 {
            return Err(Self::error(
                "E207",
                "numeric literal has multiple possible expected types",
                span,
            ));
        }
        Ok(types.into_iter().next())
    }

    pub(crate) fn floating(text: &str, expected: Option<&Type>, span: Span) -> Result<hir::Expr> {
        let context = Self::numeric_context(expected, false, span)?;
        let ty = match context.as_ref() {
            Some(Type::Float { bits }) => Type::Float { bits: *bits },
            _ => Type::Float { bits: 64 },
        };
        let text = text.replace('_', "");
        let value: f64 = if ty == (Type::Float { bits: 32 }) {
            text.parse::<f32>().map(f64::from)
        } else {
            text.parse::<f64>()
        }
        .map_err(|_| Self::error("E216", "floating literal is not representable", span))?;
        if !value.is_finite() {
            return Err(Self::error(
                "E216",
                format!("floating literal overflows {ty:?}"),
                span,
            ));
        }
        Ok(hir::Expr {
            kind: hir::ExprKind::Float(value),
            ty,
            span,
        })
    }

    pub(crate) fn integer(
        &self,
        text: &str,
        negative: bool,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<hir::Expr> {
        let text = text.replace('_', "");
        let (radix, digits) = if text.starts_with("0x") || text.starts_with("0X") {
            (16, &text[2..])
        } else if text.starts_with("0b") || text.starts_with("0B") {
            (2, &text[2..])
        } else {
            (10, text.as_str())
        };
        let context = Self::numeric_context(expected, true, span)?;
        let ty = match context.as_ref() {
            Some(Type::Int { bits, signed }) => Type::Int {
                bits: *bits,
                signed: *signed,
            },
            _ => Type::Int {
                bits: 32,
                signed: true,
            },
        };
        if negative && matches!(ty, Type::Int { signed: false, .. }) {
            return Err(Self::error(
                "E222",
                "unsigned integer negation is not defined",
                span,
            ));
        }
        let value = i128::from_str_radix(digits, radix)
            .ok()
            .and_then(|value| {
                if negative {
                    value.checked_neg()
                } else {
                    Some(value)
                }
            })
            .filter(|value| Self::in_range(*value, &ty))
            .ok_or_else(|| {
                Self::error(
                    "E216",
                    format!(
                        "literal `{}`{text} is not representable in {ty:?}",
                        if negative { "-" } else { "" }
                    ),
                    span,
                )
            })?;
        Ok(hir::Expr {
            kind: hir::ExprKind::Int(value),
            ty,
            span,
        })
    }

    pub(crate) fn in_range(value: i128, ty: &Type) -> bool {
        match ty {
            Type::Int { bits, signed: true } => {
                value >= -(1i128 << (bits - 1)) && value < (1i128 << (bits - 1))
            }
            Type::Int {
                bits,
                signed: false,
            } => value >= 0 && value < (1i128 << bits),
            _ => false,
        }
    }

    pub(crate) fn unary_value(
        &mut self,
        op: &str,
        mut value: hir::Expr,
        span: Span,
    ) -> Result<hir::Expr> {
        if value.ty == Type::Never {
            return Ok(value);
        }
        if matches!(value.ty, Type::Record { .. }) {
            value = Self::project(value);
        }
        let valid = match op {
            "-" => matches!(
                value.ty,
                Type::Int { signed: true, .. } | Type::Float { .. }
            ),
            "!" => value.ty == Type::Bool,
            "~" => matches!(value.ty, Type::Int { .. }),
            _ => false,
        };
        if !valid {
            return Err(Self::error(
                "E222",
                format!("operator `{op}` is not defined for {:?}", value.ty),
                span,
            ));
        }
        if op == "-"
            && self.reach != FALSE
            && let Some(Constant::Int(number)) = self.constant(&value)
            && number
                .checked_neg()
                .is_none_or(|number| !Self::in_range(number, &value.ty))
        {
            return Err(Self::error(
                "E107",
                format!("negating {number} overflows {:?}", value.ty),
                span,
            ));
        }
        let ty = value.ty.clone();
        Ok(hir::Expr {
            kind: hir::ExprKind::Unary {
                op: op.into(),
                value: Box::new(value),
            },
            ty,
            span,
        })
    }

    pub(crate) fn binary(
        &mut self,
        op: &str,
        left: &ast::Expr,
        right: &ast::Expr,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<hir::Expr> {
        if matches!(self.symbol(left)?, Some(Value::Function { .. }))
            || matches!(self.symbol(right)?, Some(Value::Function { .. }))
        {
            return Err(Self::error(
                "E222",
                format!("operator `{op}` is not defined for function values"),
                span,
            ));
        }
        let boolean = ["&&", "||"].contains(&op);
        let equality = ["==", "!="].contains(&op);
        let compare = ["==", "!=", "<", ">", "<=", ">="].contains(&op);
        let context = if boolean {
            Some(Type::Bool)
        } else {
            self.hint(left)
                .or_else(|| self.hint(right))
                .map(|ty| {
                    if equality {
                        ty
                    } else {
                        Self::primary_type(&ty)
                    }
                })
                .or_else(|| {
                    if compare {
                        None
                    } else {
                        expected.map(Self::primary_type)
                    }
                })
        };
        let (test, input, left) = if boolean {
            let (id, (root, value)) =
                self.with_point_id(PointKind::Condition, left.span, |checker| {
                    checker.expression_point(left, context.as_ref())
                })?;
            (Some(id), Some(root), value)
        } else if equality && matches!(context, Some(Type::Record { .. })) {
            let record = context.clone().expect("record context");
            let primary = Self::primary_type(&record);
            (None, None, self.composed(left, record, Some(&primary))?)
        } else {
            let (root, value) = self.expression_point(left, context.as_ref())?;
            (None, Some(root), value)
        };
        let skipped = if boolean {
            let guard = self.guard(&left);
            let guard = if op == "||" {
                self.flow.not(guard)
            } else {
                guard
            };
            let absent = self.flow.not(guard);
            let skipped = self.flow.and(self.reach, absent);
            self.reach = self.flow.and(self.reach, guard);
            skipped
        } else {
            FALSE
        };
        let right_context = if equality {
            left.ty.clone()
        } else {
            Self::primary_type(&left.ty)
        };
        let (arm, content, right) = if boolean {
            let kind = if op == "&&" {
                PointKind::Then
            } else {
                PointKind::Else
            };
            let (id, (root, value)) = self.with_point_id(kind, right.span, |checker| {
                checker.expression_point(right, Some(&right_context))
            })?;
            (Some(id), Some(root), value)
        } else if equality && matches!(right_context, Type::Record { .. }) {
            let primary = Self::primary_type(&right_context);
            (
                None,
                None,
                self.composed(right, right_context, Some(&primary))?,
            )
        } else {
            let (root, value) = self.expression_point(right, Some(&right_context))?;
            (None, Some(root), value)
        };
        let skipped = if boolean {
            self.reach = self.flow.or(self.reach, skipped);
            let kind = if op == "&&" {
                PointKind::Else
            } else {
                PointKind::Then
            };
            Some(self.with_point_id(kind, span, |_| Ok(()))?.0)
        } else {
            None
        };
        let value = self.binary_values(op, left, right, span)?;
        if let hir::ExprKind::Binary {
            point: Some(point), ..
        } = &value.kind
        {
            let test = test.expect("short-circuit condition");
            let arm = arm.expect("short-circuit operand");
            let skipped = skipped.expect("short-circuit skipped path");
            let (then, otherwise) = if op == "&&" {
                (arm, skipped)
            } else {
                (skipped, arm)
            };
            self.region_edges(test, input.expect("condition root"), span)?;
            self.region_edges(arm, content.expect("operand root"), span)?;
            self.branch_edges(*point, test, then, otherwise, span)?;
        } else if !boolean && let Some(point) = self.point {
            self.sequence(
                super::dependencies::SequenceSource::Expr(point),
                vec![input, content],
                span,
            )?;
        }
        Ok(value)
    }

    pub(crate) fn binary_values(
        &mut self,
        op: &str,
        mut left: hir::Expr,
        mut right: hir::Expr,
        span: Span,
    ) -> Result<hir::Expr> {
        let boolean = ["&&", "||"].contains(&op);
        let compare = ["==", "!=", "<", ">", "<=", ">="].contains(&op);
        if !["==", "!="].contains(&op)
            || !matches!(
                (&left.ty, &right.ty),
                (Type::Record { .. }, Type::Record { .. })
            )
        {
            left = Self::project(left);
            right = Self::project(right);
        }
        let diverges = !boolean && (left.ty == Type::Never || right.ty == Type::Never);
        if !diverges
            && left.ty != right.ty
            && !(boolean
                && matches!(left.ty, Type::Bool | Type::Never)
                && matches!(right.ty, Type::Bool | Type::Never))
        {
            let numeric = matches!(left.ty, Type::Int { .. } | Type::Float { .. })
                && matches!(right.ty, Type::Int { .. } | Type::Float { .. });
            return Err(Self::error(
                if numeric { "E213" } else { "E222" },
                format!(
                    "operator `{op}` requires compatible operands, found {:?} and {:?}",
                    left.ty, right.ty
                ),
                span,
            ));
        }
        let valid = match op {
            "+" | "-" | "*" | "/" => matches!(left.ty, Type::Int { .. } | Type::Float { .. }),
            "%" | "&" | "|" | "^" => matches!(left.ty, Type::Int { .. }),
            "&&" | "||" => matches!(left.ty, Type::Bool | Type::Never),
            "<" | ">" | "<=" | ">=" => matches!(
                left.ty,
                Type::Int { .. } | Type::Float { .. } | Type::String
            ),
            "==" | "!=" => left.ty.has_equality(),
            _ => false,
        };
        if !valid && !diverges {
            return Err(Self::error(
                "E222",
                format!("operator `{op}` is not defined for {:?}", left.ty),
                span,
            ));
        }
        if self.reach != FALSE
            && matches!(left.ty, Type::Int { .. })
            && !compare
            && let (Some(Constant::Int(a)), Some(Constant::Int(b))) =
                (self.constant(&left), self.constant(&right))
        {
            let value = match op {
                "+" => a.checked_add(b),
                "-" => a.checked_sub(b),
                "*" => a.checked_mul(b),
                "/" => a.checked_div(b),
                "%" => a.checked_rem(b),
                "&" => Some(a & b),
                "|" => Some(a | b),
                "^" => Some(a ^ b),
                _ => Some(0),
            };
            if value.is_none_or(|value| !Self::in_range(value, &left.ty)) {
                return Err(Self::error(
                    "E107",
                    format!("{a} {op} {b} is invalid in {:?}", left.ty),
                    span,
                ));
            }
        }
        let ty = if diverges {
            Type::Never
        } else if boolean || compare {
            Type::Bool
        } else {
            left.ty.clone()
        };
        Ok(hir::Expr {
            kind: hir::ExprKind::Binary {
                point: self.point.filter(|id| {
                    let point = &self.points[*id];
                    point.owner == self.owner
                        && ((op == "&&" && point.kind == PointKind::And)
                            || (op == "||" && point.kind == PointKind::Or))
                }),
                op: op.into(),
                left: Box::new(left),
                right: Box::new(right),
            },
            ty,
            span,
        })
    }

    pub(crate) fn constant_expr(value: Constant, span: Span) -> hir::Expr {
        let (kind, ty) = match value {
            Constant::Null => (hir::ExprKind::Null, Type::Null),
            Constant::Bool(value) => (hir::ExprKind::Bool(value), Type::Bool),
            Constant::Int(value) => (
                hir::ExprKind::Int(value),
                Type::Int {
                    bits: 32,
                    signed: true,
                },
            ),
            Constant::Float(value) => (hir::ExprKind::Float(value), Type::Float { bits: 64 }),
            Constant::String(value) => (hir::ExprKind::String(value), Type::String),
        };
        hir::Expr { kind, ty, span }
    }

    pub(crate) fn constant(&self, expr: &hir::Expr) -> Option<Constant> {
        if self.derived_expr(expr) {
            return None;
        }
        match &expr.kind {
            hir::ExprKind::ListSize(value) => self
                .list_length(value)
                .map(|size| Constant::Int(size as i128)),
            hir::ExprKind::Null => Some(Constant::Null),
            hir::ExprKind::Bool(value) => Some(Constant::Bool(*value)),
            hir::ExprKind::Int(value) => Some(Constant::Int(*value)),
            hir::ExprKind::Float(value) => Some(Constant::Float(*value)),
            hir::ExprKind::String(value) => Some(Constant::String(value.clone())),
            hir::ExprKind::Local(id) => self
                .scopes
                .iter()
                .rev()
                .flat_map(|scope| scope.values.values())
                .find_map(|value| match value {
                    Value::Local {
                        id: local,
                        constant,
                        ..
                    } if id == local => constant.clone(),
                    _ => None,
                }),
            hir::ExprKind::Unary { op, value } => match (op.as_str(), self.constant(value)?) {
                ("!", Constant::Bool(value)) => Some(Constant::Bool(!value)),
                ("-", Constant::Int(value)) => value.checked_neg().map(Constant::Int),
                ("~", Constant::Int(value)) => Some(Constant::Int(match expr.ty {
                    Type::Int {
                        bits,
                        signed: false,
                    } => !value & ((1i128 << bits) - 1),
                    _ => !value,
                })),
                ("-", Constant::Float(value)) => Some(Constant::Float(-value)),
                _ => None,
            },
            hir::ExprKind::Binary {
                op, left, right, ..
            } => {
                let a = self.constant(left)?;
                if let Constant::Bool(value) = a
                    && (op == "&&" && !value || op == "||" && value)
                {
                    return Some(Constant::Bool(value));
                }
                let b = self.constant(right)?;
                match (a, b) {
                    (Constant::Int(a), Constant::Int(b)) => match op.as_str() {
                        "+" => a.checked_add(b).map(Constant::Int),
                        "-" => a.checked_sub(b).map(Constant::Int),
                        "*" => a.checked_mul(b).map(Constant::Int),
                        "/" => a.checked_div(b).map(Constant::Int),
                        "%" => a.checked_rem(b).map(Constant::Int),
                        "&" => Some(Constant::Int(a & b)),
                        "|" => Some(Constant::Int(a | b)),
                        "^" => Some(Constant::Int(a ^ b)),
                        "==" => Some(Constant::Bool(a == b)),
                        "!=" => Some(Constant::Bool(a != b)),
                        "<" => Some(Constant::Bool(a < b)),
                        ">" => Some(Constant::Bool(a > b)),
                        "<=" => Some(Constant::Bool(a <= b)),
                        ">=" => Some(Constant::Bool(a >= b)),
                        _ => None,
                    },
                    (Constant::Bool(a), Constant::Bool(b)) => match op.as_str() {
                        "&&" => Some(Constant::Bool(a && b)),
                        "||" => Some(Constant::Bool(a || b)),
                        "==" => Some(Constant::Bool(a == b)),
                        "!=" => Some(Constant::Bool(a != b)),
                        _ => None,
                    },
                    (Constant::Null, Constant::Null) => match op.as_str() {
                        "==" => Some(Constant::Bool(true)),
                        "!=" => Some(Constant::Bool(false)),
                        _ => None,
                    },
                    (Constant::String(a), Constant::String(b)) => match op.as_str() {
                        "==" => Some(Constant::Bool(a == b)),
                        "!=" => Some(Constant::Bool(a != b)),
                        "<" => Some(Constant::Bool(a < b)),
                        ">" => Some(Constant::Bool(a > b)),
                        "<=" => Some(Constant::Bool(a <= b)),
                        ">=" => Some(Constant::Bool(a >= b)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            hir::ExprKind::Block(block) => {
                self.constants
                    .get(&block.id)
                    .cloned()
                    .or_else(|| match block.stmts.as_slice() {
                        [
                            hir::Stmt::Emit {
                                target,
                                field: None,
                                value,
                                ..
                            },
                        ] if *target == block.id => self.constant(value),
                        _ => None,
                    })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod unary;
