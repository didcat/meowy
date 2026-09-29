use super::*;
use crate::check::dependencies::IndexAccess;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Load,
    Snapshot,
    Read,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) receiver: PointId,
    pub(super) access: IndexAccess,
    pub(super) load: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

impl Checker {
    pub(super) fn index_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof index-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof index-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. } | Port::Snapshot(point) | Port::Operation(point) => {
                point
            }
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.indices.len().checked_ilog2().unwrap_or(0) as usize + 16)
        {
            return Err(budget());
        }
        let Some(op) = self.indices.get(&id) else {
            return Ok(None);
        };
        let access = op.access.ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || point.kind != PointKind::Expr
            || !point.complete
            || point.span != op.span
            || access.position == op.receiver
            || access.capacity > crate::list::MAX_CAPACITY
            || access.length.is_some_and(|length| length > access.capacity)
            || (access.normal && !access.may_return)
        {
            return Err(invalid());
        }
        for root in [op.receiver, access.position] {
            if !self.points.get(root).is_some_and(|input| {
                input.complete
                    && input.owner == owner
                    && input.parent == Some(id)
                    && input.block == point.block
                    && matches!(input.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            }) {
                return Err(invalid());
            }
        }
        let load = Port::Projection { point: id, step: 0 };
        let entry = Edge::new(Port::Entry(id), Port::Entry(op.receiver), Route::Next);
        let position = Edge::new(
            Port::Snapshot(id),
            Port::Entry(access.position),
            Route::Next,
        );
        let read = Edge::new(
            Port::Normal(access.position),
            Port::Operation(id),
            Route::Checked,
        );
        let result = Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
        let direct = [
            entry,
            Edge::new(Port::Normal(op.receiver), Port::Snapshot(id), Route::Next),
            position,
            read,
            result,
        ];
        let shared = [
            entry,
            Edge::new(Port::Normal(op.receiver), load, Route::Next),
            Edge::new(load, Port::Snapshot(id), Route::Next),
            position,
            read,
            result,
        ];
        let tail = usize::from(access.may_return) + usize::from(access.normal);
        let edges = if op.load {
            &shared[..4 + tail]
        } else {
            &direct[..3 + tail]
        };
        if op.edges.as_slice() != edges {
            return Err(invalid());
        }
        let kind = match port {
            Port::Projection { step: 0, .. } if op.load => Kind::Load,
            Port::Snapshot(_) => Kind::Snapshot,
            Port::Operation(_) if access.may_return => Kind::Read,
            _ => return Err(invalid()),
        };
        if kind == Kind::Read {
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
            receiver: op.receiver,
            access,
            load: op.load,
            control: op.control,
            kind,
        }))
    }
}

#[cfg(test)]
mod tests;
