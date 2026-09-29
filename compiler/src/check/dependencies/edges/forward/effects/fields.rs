use super::*;

impl Checker {
    pub(super) fn field_effect(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Effect> {
        let budget = || Diagnostic::unsupported("proof field-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-effect identity mismatch", span);
        if !self.flow.spend(
            self.fields.len().checked_ilog2().unwrap_or(0) as usize
                + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                + 12,
        ) {
            return Err(budget());
        }
        let field = self.fields.get(&id).ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let entry = Edge::new(Port::Entry(id), Port::Entry(field.input), Route::Next);
        let result = Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
        let stage = Port::Projection { point: id, step: 0 };
        let direct = [
            entry,
            Edge::new(Port::Normal(field.input), Port::Operation(id), Route::Next),
            result,
        ];
        let shared = [
            entry,
            Edge::new(Port::Normal(field.input), stage, Route::Next),
            Edge::new(stage, Port::Operation(id), Route::Next),
            result,
        ];
        let edges = if field.load {
            &shared[..3 + usize::from(field.normal)]
        } else {
            &direct[..2 + usize::from(field.normal)]
        };
        if field.index >= field.count
            || field.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != field.span
            || reports.index.operations.get(&id) != Some(&owner)
            || !self.points.get(field.input).is_some_and(|input| {
                input.parent == Some(id)
                    && input.owner == owner
                    && input.block == point.block
                    && input.complete
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
            || field.edges.as_slice() != edges
        {
            return Err(invalid());
        }
        Ok(Effect::Field {
            input: field.input,
            index: field.index,
            load: field.load,
            normal: field.normal,
            control: field.control,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
