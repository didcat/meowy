use super::*;
use crate::check::dependencies::CoercionKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Projection,
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) input: PointId,
    pub(super) op: CoercionKind,
    pub(super) primary: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) input: PointId,
    pub(crate) op: CoercionKind,
    pub(crate) primary: bool,
    pub(crate) control: bool,
    pub(crate) projected: bool,
    pub(crate) operation: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn coercion_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof coercion-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof coercion-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. } | Port::Operation(point) | Port::Normal(point) => point,
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.coercions.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.coercions.get(&id) else {
            return Ok(None);
        };
        if !self.flow.spend(14) {
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
                    && input.span == op.span
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
        {
            return Err(invalid());
        }
        let mut edges = [Edge::new(Port::Entry(id), Port::Entry(op.input), Route::Next); 4];
        let mut len = 1;
        let mut from = Port::Normal(op.input);
        if op.primary {
            let projection = Port::Projection { point: id, step: 0 };
            edges[len] = Edge::new(from, projection, Route::Next);
            len += 1;
            from = projection;
        }
        if op.kind == CoercionKind::Convert {
            edges[len] = Edge::new(from, Port::Operation(id), Route::Next);
            len += 1;
            from = Port::Operation(id);
        }
        if op.kind != CoercionKind::Stopped {
            edges[len] = Edge::new(from, Port::Normal(id), Route::Next);
            len += 1;
        }
        if op.edges.as_slice() != &edges[..len] {
            return Err(invalid());
        }
        let kind = match port {
            Port::Projection { step: 0, .. } if op.primary => Kind::Projection,
            Port::Operation(_) if op.kind == CoercionKind::Convert => Kind::Operation,
            Port::Normal(_) if op.kind != CoercionKind::Stopped => Kind::Result,
            _ => return Err(invalid()),
        };
        if kind != Kind::Projection && op.kind == CoercionKind::Convert {
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
            op: op.kind,
            primary: op.primary,
            control: op.control,
            kind,
        }))
    }

    pub(super) fn record_coercion_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof coercion-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof coercion-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 9)
        {
            return Err(budget());
        }
        let valid = match stage.kind {
            Kind::Projection => stage.primary,
            Kind::Operation => stage.op == CoercionKind::Convert,
            Kind::Result => stage.op != CoercionKind::Stopped,
        };
        if !valid {
            return Err(invalid());
        }
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Coercion(prior) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || prior.input != stage.input
                || prior.op != stage.op
                || prior.primary != stage.primary
                || prior.control != stage.control
            {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Coercion(op)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Coercion(Observed {
                    input: stage.input,
                    op: stage.op,
                    primary: stage.primary,
                    control: stage.control,
                    projected: false,
                    operation: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match stage.kind {
            Kind::Projection => op.projected = true,
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
