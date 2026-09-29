use super::*;
use crate::hir::FoundationType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Operation,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) ty: FoundationType,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) ty: FoundationType,
    pub(crate) control: bool,
    pub(crate) operation: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(super) fn heap_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof heap-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof heap-effect identity mismatch", span);
        let (id, kind) = match port {
            Port::Operation(id) => (id, Kind::Operation),
            Port::Normal(id) => (id, Kind::Result),
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.heap_leaves.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(leaf) = self.heap_leaves.get(&id) else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 9)
        {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let edges = [
            Edge::new(Port::Entry(id), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ];
        if leaf.ty != FoundationType::Allocator
            || leaf.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != leaf.span
            || leaf.edges != edges
            || reports.index.operations.get(&id) != Some(&owner)
        {
            return Err(invalid());
        }
        Ok(Some(Stage {
            point: id,
            owner,
            ty: leaf.ty,
            control: leaf.control,
            kind,
        }))
    }

    pub(super) fn record_heap_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof heap-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof heap-effect identity mismatch", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 6)
        {
            return Err(budget());
        }
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Heap(prior) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner || prior.ty != stage.ty || prior.control != stage.control {
                return Err(invalid());
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        let (_, Effect::Heap(leaf)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Heap(Observed {
                    ty: stage.ty,
                    control: stage.control,
                    operation: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match stage.kind {
            Kind::Operation => leaf.operation = true,
            Kind::Result => leaf.result = true,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
