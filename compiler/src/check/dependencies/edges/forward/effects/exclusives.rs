use super::*;
use std::collections::BTreeSet;

impl Checker {
    pub(super) fn validate_exclusive_borrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof exclusive-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof exclusive-effect identity mismatch", span);
        let id = match port {
            Port::Address { point, .. }
            | Port::Reserve { point, .. }
            | Port::Operation(point)
            | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.exclusives.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.exclusives.get(&id) else {
            return Ok(false);
        };
        let len = op.steps.len();
        let total = len.saturating_add(op.place.fields.len());
        if total > crate::list::MAX_WRITE_PATH
            || op.counts.len() > total
            || op.access.len() > len
            || !self.flow.spend(
                total * 2
                    + len * (len.checked_ilog2().unwrap_or(0) as usize + 10)
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + 16,
            )
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&op.place.root)
            .map_or(op.place.root, |alias| alias.root);
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.place.root >= reports.locals
            || op.storage >= reports.locals
            || op.storage != storage
            || !matches!(op.steps.first(), Some(PathStep::Index { .. }))
            || reports.index.operations.get(&id) != op.normal.then_some(&owner)
        {
            return Err(invalid());
        }
        let mut counts = op.counts.iter();
        for field in &op.place.fields {
            if field >= counts.next().ok_or_else(invalid)? {
                return Err(invalid());
            }
        }
        let mut edges = op.edges.iter();
        let mut edge = |expected| edges.next() == Some(&expected);
        let address = |step| Port::Address { point: id, step };
        if !edge(Edge::new(Port::Entry(id), address(0), Route::Next)) {
            return Err(invalid());
        }
        let mut inputs = op.access.iter();
        let mut seen = BTreeSet::new();
        let mut stop = len;
        for (step, part) in op.steps.iter().enumerate() {
            match part {
                PathStep::Field(field) => {
                    if field >= counts.next().ok_or_else(invalid)?
                        || !edge(Edge::new(address(step), address(step + 1), Route::Next))
                    {
                        return Err(invalid());
                    }
                }
                PathStep::Index {
                    point: child,
                    capacity,
                    span: site,
                } => {
                    let input = inputs.next().ok_or_else(invalid)?;
                    if *child == id
                        || *capacity > crate::list::MAX_CAPACITY
                        || input.length.is_some_and(|length| length > *capacity)
                        || site.start > site.end
                        || site.start < op.span.start
                        || site.end > op.span.end
                        || !seen.insert(*child)
                        || !self.points.get(*child).is_some_and(|child| {
                            child.complete
                                && child.owner == owner
                                && child.parent == Some(id)
                                && child.block == point.block
                                && child.span.start >= site.start
                                && child.span.end <= site.end
                                && matches!(
                                    child.kind,
                                    PointKind::Expr | PointKind::And | PointKind::Or
                                )
                        })
                    {
                        return Err(invalid());
                    }
                    let reserve = Port::Reserve { point: id, step };
                    if !edge(Edge::new(address(step), reserve, Route::Next))
                        || !edge(Edge::new(reserve, Port::Entry(*child), Route::Next))
                    {
                        return Err(invalid());
                    }
                    if input.normal {
                        if !edge(Edge::new(
                            Port::Normal(*child),
                            address(step + 1),
                            Route::Checked,
                        )) {
                            return Err(invalid());
                        }
                    } else {
                        stop = stop.min(step);
                    }
                }
            }
        }
        if counts.next().is_some() || inputs.next().is_some() || op.normal != (stop == len) {
            return Err(invalid());
        }
        if op.normal
            && (!edge(Edge::new(address(len), Port::Operation(id), Route::Next))
                || !edge(Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Next,
                )))
        {
            return Err(invalid());
        }
        if edges.next().is_some()
            || match port {
                Port::Address { step, .. } => step > stop,
                Port::Reserve { step, .. } => {
                    step > stop || !matches!(op.steps.get(step), Some(PathStep::Index { .. }))
                }
                _ => !op.normal,
            }
        {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
