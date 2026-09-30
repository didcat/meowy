use super::*;

impl Checker {
    pub(super) fn validate_place_borrow(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof place-borrow-effect budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof place-borrow-effect identity mismatch", span);
        let id = match port {
            Port::Address { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.place_borrows.get(&id) else {
            return Ok(false);
        };
        let len = op.place.fields.len();
        if len > crate::list::MAX_WRITE_PATH
            || !self.flow.spend(
                len * 3
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + 20,
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
            || op.counts.len() != len
            || op.edges.len() != len + 3
            || reports.index.operations.get(&id) != Some(&owner)
            || !point
                .block
                .and_then(|id| self.bodies.get(&id))
                .is_some_and(|body| body.owner == owner)
            || point.parent.is_some_and(|parent| {
                parent == id
                    || !self.points.get(parent).is_some_and(|parent| {
                        parent.complete && parent.owner == owner && parent.block == point.block
                    })
            })
            || matches!(port, Port::Address { step, .. } if step > len)
        {
            return Err(invalid());
        }
        let address = |step| Port::Address { point: id, step };
        if op.edges[0] != Edge::new(Port::Entry(id), address(0), Route::Next)
            || op.edges[len + 1] != Edge::new(address(len), Port::Operation(id), Route::Next)
            || op.edges[len + 2] != Edge::new(Port::Operation(id), Port::Normal(id), Route::Next)
        {
            return Err(invalid());
        }
        for (step, (&field, &count)) in op.place.fields.iter().zip(&op.counts).enumerate() {
            if field >= count
                || op.edges[step + 1] != Edge::new(address(step), address(step + 1), Route::Next)
            {
                return Err(invalid());
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;
