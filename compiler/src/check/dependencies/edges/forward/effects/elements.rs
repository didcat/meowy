use super::*;
use crate::check::dependencies::ElementSource;

impl Checker {
    pub(super) fn validate_element_borrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof element-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof element-effect identity mismatch", span);
        let id = match port {
            Port::Address { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.elements.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.elements.get(&id) else {
            return Ok(false);
        };
        if !self
            .flow
            .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 20)
        {
            return Err(budget());
        }
        let access = op.access.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let address = Port::Address { point: id, step: 0 };
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(op.parent), Route::Next),
            Edge::new(Port::Normal(op.parent), address, Route::Next),
            Edge::new(address, Port::Entry(access.position), Route::Next),
            Edge::new(
                Port::Normal(access.position),
                Port::Operation(id),
                Route::Checked,
            ),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || access.position == op.parent
            || access.capacity > crate::list::MAX_CAPACITY
            || access.length.is_some_and(|length| length > access.capacity)
            || access.site >= self.reborrows
            || op.edges.as_slice() != &edges[..3 + usize::from(access.may_return) * 2]
            || reports.index.operations.get(&id) != access.may_return.then_some(&owner)
            || match port {
                Port::Address { step, .. } => step != 0,
                _ => !access.may_return,
            }
        {
            return Err(invalid());
        }
        for root in [op.parent, access.position] {
            if root == id
                || !self.points.get(root).is_some_and(|child| {
                    child.complete
                        && child.owner == owner
                        && child.parent == Some(id)
                        && child.block == point.block
                        && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
            {
                return Err(invalid());
            }
        }
        match &op.source {
            ElementSource::Stopped => return Err(invalid()),
            ElementSource::View => {}
            ElementSource::Place {
                place,
                storage,
                counts,
            } => {
                if place.fields.len() > crate::list::MAX_WRITE_PATH
                    || !self.flow.spend(
                        place.fields.len() * 2
                            + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                            + 6,
                    )
                {
                    return Err(budget());
                }
                let root = self
                    .proofs
                    .aliases
                    .get(&place.root)
                    .map_or(place.root, |alias| alias.root);
                if place.root >= reports.locals
                    || *storage >= reports.locals
                    || *storage != root
                    || counts.len() != place.fields.len()
                    || place
                        .fields
                        .iter()
                        .zip(counts)
                        .any(|(field, count)| field >= count)
                {
                    return Err(invalid());
                }
            }
            ElementSource::Temporary { local, statement } => {
                if !self.flow.spend(
                    self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                        + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                        + 16,
                ) {
                    return Err(budget());
                }
                let site = self.sites.get(statement).ok_or_else(invalid)?;
                if *local >= reports.locals
                    || *statement >= self.statements
                    || point.site != Some(*statement)
                    || self.points[op.parent].site != point.site
                    || self.proofs.temporaries.get(local) != Some(statement)
                    || site.owner != owner
                    || !site.complete
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
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
