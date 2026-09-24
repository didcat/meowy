use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result};
use crate::hir::{ExprKind, Place, Type};

impl Checker {
    pub(super) fn shape_temporary(&mut self, value: &Expr) -> Result<Option<Place>> {
        let mut base = value;
        let mut fields: Vec<usize> = Vec::new();
        for _ in 0..=MAX_DEPTH {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof temporary shape budget exhausted",
                    value.span,
                ));
            }
            match &base.kind {
                ExprKind::TemporaryBorrow {
                    id, value: source, ..
                } => {
                    fields.reverse();
                    let mut ty = &source.ty;
                    for index in &fields {
                        let Some(Type::Record { fields, .. }) = Self::origin_record(ty) else {
                            return Ok(None);
                        };
                        let Some(field) = fields.get(*index) else {
                            return Ok(None);
                        };
                        ty = &field.ty;
                    }
                    return Ok(Some(Place { root: *id, fields }));
                }
                ExprKind::Reborrow {
                    value: inner,
                    fields: path,
                    ..
                } => {
                    if fields.len() + path.len() > MAX_DEPTH || !self.flow.spend(path.len()) {
                        return Err(Diagnostic::unsupported(
                            "proof temporary shape path budget exhausted",
                            value.span,
                        ));
                    }
                    fields.extend(path.iter().rev().copied());
                    base = inner;
                }
                _ => return Ok(None),
            }
        }
        Err(Diagnostic::unsupported(
            "proof temporary shape depth exhausted",
            value.span,
        ))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod sources;
