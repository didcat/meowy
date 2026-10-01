use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) local: crate::hir::LocalId,
    pub(crate) statement: crate::hir::StatementId,
    pub(crate) control: bool,
    pub(crate) acquired: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn record_temporary_effect(
        &mut self,
        owner: usize,
        port: Port,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof temporary-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof temporary-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.temporary_borrows.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 8,
        ) {
            return Err(budget());
        }
        let op = self.temporary_borrows.get(&id).ok_or_else(invalid)?;
        let cell = op.cell.ok_or_else(invalid)?;
        if op.owner != owner {
            return Err(invalid());
        }
        if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::Temporary(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.input != op.input
                || prior.local != cell.local
                || prior.statement != cell.statement
                || prior.control != op.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Temporary(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::Temporary(Observed {
                    input: op.input,
                    local: cell.local,
                    statement: cell.statement,
                    control: op.control,
                    acquired: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match port {
            Port::Operation(_) => observed.acquired = true,
            Port::Normal(_) => observed.result = true,
            _ => unreachable!(),
        }
        Ok(())
    }

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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;
