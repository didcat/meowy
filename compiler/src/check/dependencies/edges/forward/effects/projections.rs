use super::*;
use crate::check::dependencies::ProjectionStep;

impl Checker {
    pub(super) fn validate_borrow_projection(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof projection-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof projection-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. }
            | Port::Conversion { point, .. }
            | Port::Operation(point)
            | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.projections.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.projections.get(&id) else {
            return Ok(false);
        };
        if op.steps.len() > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                op.steps.len() * 6
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                    + 24,
            )
        {
            return Err(budget());
        }
        let site = op.site.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.mode.is_none()
            || site >= self.reborrows
            || op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.parent == id
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
        let mut edges = op.edges.iter();
        let mut edge = |expected| edges.next() == Some(&expected);
        if !edge(Edge::new(
            Port::Entry(id),
            Port::Entry(op.parent),
            Route::Next,
        )) {
            return Err(invalid());
        }
        let mut from = Port::Normal(op.parent);
        let mut loaded = false;
        let mut address = false;
        for (step, item) in op.steps.iter().enumerate() {
            match item {
                ProjectionStep::Materialize { local, statement } => {
                    let site = self.sites.get(statement).ok_or_else(invalid)?;
                    if step != 0
                        || *local >= reports.locals
                        || *statement >= self.statements
                        || point.site != Some(*statement)
                        || self.points[op.parent].site != point.site
                        || self.proofs.temporaries.get(local) != Some(statement)
                        || !site.complete
                        || site.owner != owner
                        || site.block != point.block
                        || !site
                            .point
                            .and_then(|id| self.points.get(id))
                            .is_some_and(|root| {
                                root.complete
                                    && root.kind == PointKind::Stmt
                                    && root.owner == owner
                                    && root.site == Some(*statement)
                                    && root.block == site.block
                                    && root.span == site.span
                            })
                    {
                        return Err(invalid());
                    }
                }
                ProjectionStep::Field {
                    index,
                    count,
                    narrow,
                } => {
                    if address
                        || (loaded && *narrow)
                        || index >= count
                        || *index >= crate::borrow_value::MAX_PARTS
                    {
                        return Err(invalid());
                    }
                }
                ProjectionStep::Load(_) => {
                    if address {
                        return Err(invalid());
                    }
                    loaded = true;
                }
                ProjectionStep::Address { index, count } => {
                    if index >= count || *index >= crate::borrow_value::MAX_PARTS {
                        return Err(invalid());
                    }
                    address = true;
                }
            }
            let to = Port::Projection { point: id, step };
            if !edge(Edge::new(from, to, Route::Next)) {
                return Err(invalid());
            }
            from = to;
            if matches!(item, ProjectionStep::Field { narrow: true, .. }) {
                let to = Port::Conversion {
                    point: id,
                    part: step,
                };
                if !edge(Edge::new(from, to, Route::Next)) {
                    return Err(invalid());
                }
                from = to;
            }
        }
        if !edge(Edge::new(from, Port::Operation(id), Route::Next))
            || !edge(Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next,
            ))
            || edges.next().is_some()
            || match port {
                Port::Projection { step, .. } => step >= op.steps.len(),
                Port::Conversion { part, .. } => !matches!(
                    op.steps.get(part),
                    Some(ProjectionStep::Field { narrow: true, .. })
                ),
                _ => false,
            }
        {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
