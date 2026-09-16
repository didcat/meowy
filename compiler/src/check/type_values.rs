#[cfg(test)]
mod accounting;
mod blocks;
mod booleans;
mod comparisons;
#[cfg(test)]
mod expression_accounting;
mod fields;
mod inferred;
#[cfg(test)]
mod integer_accounting;
mod integers;
mod matches;
mod metatypes;
mod operands;
mod records;
mod scalars;
mod statements;
mod subtraction;
mod work;

pub(crate) use work::Work;

use super::{Checker, Result, Scope, Value};
use crate::ast::{self, ExprKind};
use crate::diagnostic::Diagnostic;
use crate::hir::Type;

pub(crate) const MAX_WORK: usize = 4096;
pub(crate) const MAX_DEPTH: usize = 64;
pub(crate) const MAX_NODES: usize = 16384;

#[derive(Default)]
pub(crate) struct Output {
    pub(crate) infer: bool,
    pub(crate) ty: Option<Type>,
    pub(crate) value: Option<Value>,
    pub(crate) fields: std::collections::BTreeMap<usize, Value>,
}

impl Output {
    pub(crate) fn record(&self) -> bool {
        matches!(self.ty, Some(Type::Record { .. }))
    }
}

pub(crate) fn transparent_type(expr: &ast::Expr) -> bool {
    match &expr.kind {
        ExprKind::Group(_) => true,
        ExprKind::TypeValue(ast::TypeExpr {
            kind: ast::TypeKind::Computed(value),
            ..
        }) => expr.span == value.span,
        _ => false,
    }
}

impl Checker {
    pub(crate) fn type_value(&mut self, expr: &ast::Expr) -> Result<Type> {
        self.required_root(expr.span, |checker| {
            checker.type_work.as_mut().unwrap().enter(expr.span)?;
            let result = checker.type_value_inner(expr);
            let work = checker.type_work.as_mut().unwrap();
            work.depth -= 1;
            let ty = result?;
            work.type_result(&ty, expr)?;
            Ok(ty)
        })
    }

    pub(crate) fn type_value_inner(&mut self, expr: &ast::Expr) -> Result<Type> {
        if !transparent_type(expr) && !matches!(expr.kind, ExprKind::Block(_)) {
            self.type_work.as_mut().unwrap().logical.charge(1, 0)?;
        }
        if matches!(expr.kind, ExprKind::Field { .. }) {
            self.charge_ancestors(expr)?;
        }
        match &expr.kind {
            ExprKind::TypeValue(ty) => self.type_literal(ty),
            ExprKind::TypeQuery(value) => {
                if let Some(ty) = self.hint(value) {
                    return Ok(ty);
                }
                match &value.kind {
                    ExprKind::Int(_) => Ok(Type::Int {
                        bits: 32,
                        signed: true,
                    }),
                    ExprKind::Float(_) => Ok(Type::Float { bits: 64 }),
                    ExprKind::String(_) => Ok(Type::String),
                    _ => Err(Diagnostic::unsupported(
                        "type queries requiring expression evaluation",
                        expr.span,
                    )),
                }
            }
            ExprKind::Name(name) => match self.value(name, expr.span)? {
                Value::Type(ty) => Ok(ty),
                Value::Foundation(crate::foundation::Item::Type(ty)) => Ok(Type::Foundation(ty)),
                _ => Err(Self::error(
                    "E211",
                    "computed annotation does not produce a compile-time type",
                    expr.span,
                )),
            },
            ExprKind::Group(value) => self.type_value(value),
            ExprKind::Binary { op, left, right } if op == "!" => {
                self.type_operand_form(expr, self.type_work.as_ref().unwrap().depth, &mut 0)?;
                let left = self.subtraction_operand(left)?;
                let right = self.subtraction_operand(right)?;
                Ok(left.subtract(&right))
            }
            ExprKind::Block(block) => self.type_block(block),
            ExprKind::Field { .. } => match self.symbol(expr)? {
                Some(Value::Type(ty)) => Ok(ty),
                Some(Value::Foundation(crate::foundation::Item::Type(ty))) => {
                    Ok(Type::Foundation(ty))
                }
                _ => Err(Self::error(
                    "E211",
                    "computed member is not a compile-time type",
                    expr.span,
                )),
            },
            _ => Err(self.type_unavailable(expr)?),
        }
    }

    pub(crate) fn type_unavailable(&mut self, expr: &ast::Expr) -> Result<Diagnostic> {
        let mut expr = expr;
        while let ExprKind::Group(value) = &expr.kind {
            expr = value;
        }
        if let ExprKind::Call { callee, .. } = &expr.kind
            && matches!(self.symbol(callee)?, Some(Value::Print | Value::Panic))
        {
            return Ok(Self::error(
                "E219",
                "debug I/O or panic is forbidden during required type evaluation",
                expr.span,
            ));
        }
        Ok(Diagnostic::unsupported(
            "computed type evaluation",
            expr.span,
        ))
    }

    pub(crate) fn type_block(&mut self, block: &ast::Block) -> Result<Type> {
        let Value::Type(ty) = self.required_block(block, None)? else {
            unreachable!()
        };
        Ok(ty)
    }

    pub(crate) fn required_block(
        &mut self,
        block: &ast::Block,
        expected: Option<&Type>,
    ) -> Result<Value> {
        let output = self.required_output(block, expected)?;
        if output.record() {
            return self.finish_required_record(output, block.span);
        }
        output.value.ok_or_else(|| {
            Self::error(
                if expected.is_some() { "E204" } else { "E211" },
                if expected.is_some() {
                    "required scalar block does not initialize its primary"
                } else {
                    "computed block does not emit a compile-time type"
                },
                block.span,
            )
        })
    }

    pub(crate) fn required_output(
        &mut self,
        block: &ast::Block,
        expected: Option<&Type>,
    ) -> Result<Output> {
        self.scoped_output(
            block,
            Output {
                ty: expected.cloned(),
                ..Output::default()
            },
        )
    }

    pub(crate) fn scoped_output(
        &mut self,
        block: &ast::Block,
        mut output: Output,
    ) -> Result<Output> {
        if block.label.is_some() {
            return Err(Diagnostic::unsupported(
                if output.record() {
                    "labeled required record blocks"
                } else if output.ty.is_some() {
                    "labeled required scalar blocks"
                } else {
                    "labeled computed type blocks"
                },
                block.span,
            ));
        }
        self.type_work.as_mut().unwrap().logical.charge(1, 0)?;
        self.scopes.push(Scope::default());
        let result = self.type_statements(&block.stmts, &mut output);
        self.scopes.pop();
        result?;
        Ok(output)
    }

    pub(crate) fn type_binding(
        &mut self,
        expr: &ast::Expr,
        annotation: Option<&ast::TypeExpr>,
    ) -> Result<Value> {
        if let Some(annotation) = annotation
            && self.meta_annotation(annotation)?
        {
            return self.meta_binding(expr, annotation);
        }
        let mut form = expr;
        while let ExprKind::Group(value) = &form.kind {
            form = value;
        }
        if matches!(form.kind, ExprKind::Block(_))
            && let Some(annotation) = annotation
        {
            let ty = self.ty(annotation)?;
            if matches!(ty, Type::Record { .. }) {
                return self.record_block(expr, &ty);
            }
            if !matches!(ty, Type::Int { .. } | Type::Bool) {
                return Err(Diagnostic::unsupported(
                    "required block results outside integers and booleans",
                    expr.span,
                ));
            }
            return self.scalar_block(expr, &ty);
        }
        if matches!(form.kind, ExprKind::Block(_)) {
            return self.inferred_block(expr);
        }
        if matches!(form.kind, ExprKind::Name(_) | ExprKind::Field { .. })
            && matches!(self.required_hint(expr), Some(Type::Record { .. }))
        {
            let module = if let ExprKind::Name(name) = &form.kind {
                matches!(
                    self.required_value(name, form.span)?,
                    Value::FileModule { .. }
                )
            } else {
                false
            };
            if !module {
                return self.type_record(expr, annotation);
            }
        }
        let scalar = match &form.kind {
            ExprKind::Binary { op, .. } if op == "!" => false,
            ExprKind::Int(_)
            | ExprKind::Unary { .. }
            | ExprKind::Binary { .. }
            | ExprKind::Call { .. } => true,
            ExprKind::Name(name) => match self.required_value(name, form.span)? {
                Value::Static { .. } | Value::Local { .. } | Value::Constant(_) => true,
                Value::FileModule { ty, .. } => {
                    matches!(ty, Type::Int { .. } | Type::Bool)
                        || annotation.is_some()
                            && matches!(Self::primary_type(&ty), Type::Int { .. } | Type::Bool)
                }
                _ => false,
            },
            ExprKind::Field { .. } => {
                let saved = std::mem::replace(&mut self.required, true);
                let symbol = self.symbol(form);
                self.required = saved;
                !matches!(
                    symbol?,
                    Some(Value::Type(_) | Value::Foundation(crate::foundation::Item::Type(_)))
                )
            }
            _ => false,
        };
        if !scalar {
            if annotation.is_some() {
                return Err(Diagnostic::unsupported(
                    "annotated computed type identity",
                    expr.span,
                ));
            }
            return self.type_value(expr).map(Value::Type);
        }
        self.type_scalar(expr, annotation)
    }
}

#[cfg(test)]
mod tests;
