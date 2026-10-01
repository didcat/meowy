use super::*;
use crate::hir::ReferenceMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) parent: PointId,
    pub(crate) site: crate::hir::ReborrowId,
    pub(crate) parent_mode: ReferenceMode,
    pub(crate) mode: ReferenceMode,
    pub(crate) control: bool,
    pub(crate) acquired: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn record_reborrow_effect(
        &mut self,
        owner: usize,
        port: Port,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof reborrow-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof reborrow-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.reborrow_ops.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 9,
        ) {
            return Err(budget());
        }
        let op = self.reborrow_ops.get(&id).ok_or_else(invalid)?;
        let site = op.site.ok_or_else(invalid)?;
        let parent_mode = op.parent_mode.ok_or_else(invalid)?;
        if op.owner != owner {
            return Err(invalid());
        }
        if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::Reborrow(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.parent != op.parent
                || prior.site != site
                || prior.parent_mode != parent_mode
                || prior.mode != op.mode
                || prior.control != op.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Reborrow(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::Reborrow(Observed {
                    parent: op.parent,
                    site,
                    parent_mode,
                    mode: op.mode,
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod limits;
