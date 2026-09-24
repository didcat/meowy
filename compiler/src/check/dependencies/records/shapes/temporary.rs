use super::{Checker, Diagnostic, Expr, MAX_DEPTH, Result};
use crate::hir::ExprKind;

impl Checker {
    pub(super) fn shape_temporary(&mut self, value: &Expr) -> Result<Option<usize>> {
        let mut base = value;
        for _ in 0..=MAX_DEPTH {
            if !self.flow.spend(1) {
                return Err(Diagnostic::unsupported(
                    "proof temporary shape budget exhausted",
                    value.span,
                ));
            }
            match &base.kind {
                ExprKind::TemporaryBorrow { id, .. } => return Ok(Some(*id)),
                ExprKind::Reborrow {
                    value: inner,
                    fields,
                    ..
                } if fields.is_empty() => base = inner,
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
