use super::*;
use crate::hir::ReferenceMode;

impl Checker {
    pub(super) fn validate_reborrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof reborrow-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof reborrow-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.reborrow_ops.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.reborrow_ops.get(&id) else {
            return Ok(false);
        };
        if !self
            .flow
            .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 14)
        {
            return Err(budget());
        }
        let site = op.site.ok_or_else(invalid)?;
        let mode = op.parent_mode.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(op.parent), Route::Next),
            Edge::new(Port::Normal(op.parent), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if site >= self.reborrows
            || (op.mode == ReferenceMode::Exclusive && mode != ReferenceMode::Exclusive)
            || op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.parent == id
            || op.edges.as_slice() != edges
            || reports.index.operations.get(&id) != Some(&owner)
            || !self.points.get(op.parent).is_some_and(|parent| {
                parent.complete
                    && parent.owner == owner
                    && parent.parent == Some(id)
                    && parent.block == point.block
                    && matches!(
                        parent.kind,
                        PointKind::Expr | PointKind::And | PointKind::Or
                    )
            })
        {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
