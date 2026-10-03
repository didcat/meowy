use super::*;

mod initializers;

impl Checker {
    pub(super) fn read_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Effect> {
        let budget = || Diagnostic::unsupported("proof read-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof read-effect identity mismatch", span);
        if !self.flow.spend(
            self.local_reads.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + 8,
        ) {
            return Err(budget());
        }
        let read = self.local_reads.get(&id).ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let storage = self
            .proofs
            .aliases
            .get(&read.local)
            .map_or(read.local, |alias| alias.root);
        let edges = [
            Edge::new(Port::Entry(id), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if read.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != read.span
            || reports.index.operations.get(&id) != Some(&owner)
            || read.local >= reports.locals
            || read.storage >= reports.locals
            || read.storage != storage
            || read.edges.as_slice() != &edges[..1 + usize::from(read.normal)]
        {
            return Err(invalid());
        }
        Ok(Effect::Read {
            local: read.local,
            storage: read.storage,
            normal: read.normal,
            control: read.control,
        })
    }
}

#[cfg(test)]
mod tests;
