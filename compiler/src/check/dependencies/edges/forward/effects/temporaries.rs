use super::*;

impl Checker {
    pub(super) fn validate_temporary_borrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof temporary-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof temporary-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.temporary_borrows.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.temporary_borrows.get(&id) else {
            return Ok(false);
        };
        if !self.flow.spend(
            self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + 22,
        ) {
            return Err(budget());
        }
        let cell = op.cell.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let site = self.sites.get(&cell.statement).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
            Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || point.site != Some(cell.statement)
            || cell.local >= reports.locals
            || cell.statement >= self.statements
            || self.proofs.temporaries.get(&cell.local) != Some(&cell.statement)
            || site.owner != owner
            || !site.complete
            || site.block != point.block
            || op.input == id
            || op.edges.as_slice() != edges
            || reports.index.operations.get(&id) != Some(&owner)
            || !site
                .point
                .and_then(|id| self.points.get(id))
                .is_some_and(|root| {
                    root.complete
                        && root.kind == PointKind::Stmt
                        && root.owner == owner
                        && root.site == Some(cell.statement)
                        && root.block == site.block
                        && root.span == site.span
                })
            || !self.points.get(op.input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && input.site == point.site
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
