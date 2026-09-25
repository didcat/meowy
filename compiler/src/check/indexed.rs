use super::dependencies::PathStep;
use super::{Checker, Result};
use crate::ast::{self, ExprKind, Span};
use crate::diagnostic::Diagnostic;
use crate::hir::{self, Type};

impl Checker {
    pub(crate) fn exclusive_indexed(
        &mut self,
        target: &ast::Expr,
        span: Span,
    ) -> Result<Option<hir::Expr>> {
        let Some((value, steps)) = self.exclusive_indexed_points(target, span)? else {
            return Ok(None);
        };
        if let Some(id) = self.point {
            self.exclusive_operation(id, &value, steps)?;
        }
        Ok(Some(value))
    }

    pub(crate) fn exclusive_indexed_points(
        &mut self,
        target: &ast::Expr,
        span: Span,
    ) -> Result<Option<(hir::Expr, Vec<PathStep>)>> {
        let mut root = target;
        let mut steps = Vec::new();
        loop {
            if !self.flow.spend(1) || steps.len() > crate::list::MAX_WRITE_PATH + 1 {
                return Err(crate::borrow_value::State::budget(span));
            }
            match &root.kind {
                ExprKind::Group(value) => root = value,
                ExprKind::Field { value, .. } | ExprKind::Index { value, .. } => {
                    steps.push(root);
                    root = value;
                }
                _ => break,
            }
        }
        let Some(first) = steps
            .iter()
            .rposition(|step| matches!(step.kind, ExprKind::Index { .. }))
        else {
            return Ok(None);
        };
        let ExprKind::Index { value: base, .. } = &steps[first].kind else {
            unreachable!()
        };
        let (place, mut ty, mut mutable) = self.exclusive_place(base, span, true)?;
        let mut path = Vec::new();
        let mut points = Vec::new();
        let mut diverges = false;
        for step in steps[..=first].iter().rev() {
            match &step.kind {
                ExprKind::Field { name, .. } => {
                    let (index, field, writable) = self.record_field(ty, name, step.span)?;
                    mutable = writable;
                    path.push(hir::WriteStep::Field(index));
                    points.push(PathStep::Field(index));
                    ty = field;
                }
                ExprKind::Index { index, .. } => {
                    let Type::List { element, capacity } = ty else {
                        return Err(Diagnostic::unsupported(
                            "indexed exclusive owner requires a list",
                            step.span,
                        ));
                    };
                    let length = if path.is_empty() && place.fields.is_empty() {
                        self.lengths.get(&place.root).map(|fact| fact.length)
                    } else {
                        None
                    };
                    let (point, index) = self.list_position_point(index, length, capacity)?;
                    points.push(PathStep::Index {
                        point,
                        capacity,
                        span: step.span,
                    });
                    diverges |= index.ty == Type::Never;
                    path.push(hir::WriteStep::Index(hir::IndexStep {
                        index,
                        span: step.span,
                    }));
                    ty = *element;
                }
                _ => unreachable!(),
            }
        }
        if let Some(hir::WriteStep::Index(step)) = path.last_mut() {
            step.span = span;
        }
        if let Some(PathStep::Index { span: last, .. }) = points.last_mut() {
            *last = span;
        }
        let result = self.exclusive_type(ty, span)?;
        if !mutable {
            return Err(Self::error(
                "E305",
                "exclusive borrow target is immutable",
                span,
            ));
        }
        Ok(Some((
            hir::Expr {
                kind: hir::ExprKind::ExclusivePath { place, path },
                ty: if diverges { Type::Never } else { result },
                span,
            },
            points,
        )))
    }
}

#[cfg(test)]
mod tests;
