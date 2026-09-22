use super::{Checker, Result, Value};
use crate::ast::{self, ExprKind};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn record_field(
        &mut self,
        ty: Type,
        name: &str,
        span: ast::Span,
    ) -> Result<(usize, Type, bool)> {
        let Type::Record { fields, .. } = ty else {
            return Err(Diagnostic::unsupported(
                "mutable field access requires concrete record storage",
                span,
            ));
        };
        if !self.flow.spend(fields.len().saturating_mul(name.len() + 1)) {
            return Err(Diagnostic::unsupported(
                "mutable field lookup budget exhausted",
                span,
            ));
        }
        let (index, field) = fields
            .into_iter()
            .enumerate()
            .find(|(_, field)| field.name == name)
            .ok_or_else(|| Self::error("E201", format!("unknown record field `{name}`"), span))?;
        Ok((index, field.ty, field.mutable))
    }

    pub(crate) fn write_path(
        &mut self,
        target: &ast::Expr,
        value: &ast::Expr,
    ) -> Result<hir::Stmt> {
        let mut root = target;
        let mut steps = Vec::new();
        loop {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "assignment path budget exhausted",
                    target.span,
                ));
            }
            match &root.kind {
                ExprKind::Group(value) => root = value,
                ExprKind::Field { value, .. } | ExprKind::Index { value, .. } => {
                    if steps.len() == crate::list::MAX_WRITE_PATH {
                        return Err(Diagnostic::unsupported(
                            "assignment path budget exhausted",
                            target.span,
                        ));
                    }
                    steps.push(root);
                    root = value;
                }
                _ => break,
            }
        }
        let ExprKind::Name(name) = &root.kind else {
            return Err(Diagnostic::unsupported(
                "assignment path outside ordinary local storage",
                target.span,
            ));
        };
        let Value::Local {
            id, ty, mutable, ..
        } = self.value(name, root.span)?
        else {
            return Err(Diagnostic::unsupported(
                "assignment path outside ordinary local storage",
                target.span,
            ));
        };
        if (!self.places.contains(&id) && !self.proofs.aliases.contains_key(&id))
            || !matches!(ty, Type::Record { .. } | Type::List { .. })
            || (ty.has_reference() && !ty.fixed_borrowed_value())
        {
            return Err(Diagnostic::unsupported(
                "assignment path requires supported ordinary storage",
                target.span,
            ));
        }
        crate::borrow_contract::type_weight(&ty, &mut self.flow, target.span)?;
        let mut ty = ty;
        let mut mutable = mutable;
        let mut path = Vec::new();
        let mut names = Vec::new();
        let mut indexed = false;
        for step in steps.into_iter().rev() {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "assignment path budget exhausted",
                    step.span,
                ));
            }
            match &step.kind {
                ExprKind::Field { name, .. } => {
                    let (index, field, writable) = self.record_field(ty, name, step.span)?;
                    mutable = writable;
                    path.push(hir::WriteStep::Field(index));
                    if !indexed {
                        names.push(name.clone());
                    }
                    ty = field;
                }
                ExprKind::Index { index, .. } => {
                    let Type::List { element, capacity } = ty else {
                        return Err(Diagnostic::unsupported(
                            "index assignment requires concrete lists",
                            step.span,
                        ));
                    };
                    indexed = true;
                    path.push(hir::WriteStep::Index(hir::IndexStep {
                        index: self.list_position(index, None, capacity)?,
                        span: step.span,
                    }));
                    ty = *element;
                }
                _ => unreachable!(),
            }
        }
        if let Some(hir::WriteStep::Index(last)) = path.last_mut() {
            last.span = target.span;
        }
        if !mutable {
            return Err(Self::error(
                "E305",
                "assignment target is immutable",
                target.span,
            ));
        }
        let value = self.expr(value, Some(&ty))?;
        let derived = self.control
            || self.derived_expr(&value)
            || path.iter().any(|step| match step {
                hir::WriteStep::Index(step) => self.derived_expr(&step.index),
                hir::WriteStep::Field(_) => false,
            });
        self.forget_field(id, &names, target.span)?;
        let fields = path
            .iter()
            .map(|step| match step {
                hir::WriteStep::Field(index) => Some(*index),
                hir::WriteStep::Index(_) => None,
            })
            .collect::<Option<Vec<_>>>();
        if let Some(fields) = fields {
            self.write_reference_field(id, &fields, &value)?;
            self.track_record_prefix(id, &fields, &value, true)?;
        }
        if derived {
            self.mark_derived(id);
        }
        Ok(hir::Stmt::SetPath {
            id,
            path,
            value,
            span: target.span,
        })
    }
}
