use super::*;
use crate::check::dependencies::TypedKind;

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
    pub(super) op: TypedKind,
    pub(super) normal: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) op: TypedKind,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) operation: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn typed_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof typed-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof typed-effect identity mismatch", span);
        let (id, kind) = match port {
            Port::Operation(id) => (id, Kind::Operation),
            Port::Normal(id) => (id, Kind::Result),
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.typed_ops.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.typed_ops.get(&id) else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 12)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next),
            Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if !op.normal
            || op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.input == id
            || op.edges.as_slice() != edges
            || reports.index.operations.get(&id) != Some(&owner)
            || !self.points.get(op.input).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        Ok(Some(Stage {
            point: id,
            owner,
            input: op.input,
            op: op.kind,
            normal: op.normal,
            control: op.control,
            kind,
        }))
    }

    pub(super) fn record_typed_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof typed-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof typed-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 8)
        {
            return Err(budget());
        }
        if !stage.normal {
            return Err(invalid());
        }
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Typed(prior) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || prior.input != stage.input
                || prior.op != stage.op
                || prior.normal != stage.normal
                || prior.control != stage.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Typed(op)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Typed(Observed {
                    input: stage.input,
                    op: stage.op,
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

#[cfg(test)]
mod boundaries;
