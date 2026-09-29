use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) input: PointId,
    pub(super) changed: bool,
    pub(super) normal: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) changed: bool,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) operation: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn narrowing_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof narrowing-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof narrowing-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.narrowings.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.narrowings.get(&id) else {
            return Ok(None);
        };
        if !self.flow.spend(12) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.input == id
            || !self.points.get(op.input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && input.kind == PointKind::Expr
                    && input.span == op.span
            })
        {
            return Err(invalid());
        }
        let next = if op.changed {
            Port::Operation(id)
        } else {
            Port::Normal(id)
        };
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
            Edge::new(Port::Normal(op.input), next, Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if op.edges.as_slice() != &edges[..1 + usize::from(op.changed) + usize::from(op.normal)] {
            return Err(invalid());
        }
        let kind = match port {
            Port::Operation(_) if op.changed => Kind::Operation,
            Port::Normal(_) if op.normal => Kind::Result,
            _ => return Err(invalid()),
        };
        if op.changed {
            if !self
                .flow
                .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            if reports.index.operations.get(&id) != Some(&owner) {
                return Err(invalid());
            }
        }
        Ok(Some(Stage {
            point: id,
            owner,
            input: op.input,
            changed: op.changed,
            normal: op.normal,
            control: op.control,
            kind,
        }))
    }

    pub(super) fn record_narrowing_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof narrowing-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof narrowing-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 8)
        {
            return Err(budget());
        }
        if (stage.kind == Kind::Operation && !stage.changed)
            || (stage.kind == Kind::Result && !stage.normal)
        {
            return Err(invalid());
        }
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Narrowing(prior) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || prior.input != stage.input
                || prior.changed != stage.changed
                || prior.normal != stage.normal
                || prior.control != stage.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Narrowing(op)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Narrowing(Observed {
                    input: stage.input,
                    changed: stage.changed,
                    normal: stage.normal,
                    control: stage.control,
                    operation: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match stage.kind {
            Kind::Operation => op.operation = true,
            Kind::Result => op.result = true,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
