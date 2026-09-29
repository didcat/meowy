use super::*;

impl Checker {
    pub(super) fn deref_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Effect> {
        let budget = || Diagnostic::unsupported("proof dereference-effect budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof dereference-effect identity mismatch", span);
        if !self.flow.spend(
            self.derefs.len().checked_ilog2().unwrap_or(0) as usize
                + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + 10,
        ) {
            return Err(budget());
        }
        let deref = self.derefs.get(&id).ok_or_else(invalid)?;
        let mode = deref.mode.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(deref.input), Route::Next),
            Edge::new(Port::Normal(deref.input), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if deref.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != deref.span
            || reports.index.operations.get(&id) != Some(&owner)
            || !self.points.get(deref.input).is_some_and(|input| {
                input.parent == Some(id)
                    && input.owner == owner
                    && input.block == point.block
                    && input.complete
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
            || deref.edges.as_slice() != &edges[..2 + usize::from(deref.normal)]
        {
            return Err(invalid());
        }
        Ok(Effect::Deref {
            input: deref.input,
            mode,
            normal: deref.normal,
            control: deref.control,
        })
    }
}

#[cfg(test)]
mod tests;
