use super::*;
use crate::check::dependencies::MethodKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Load,
    Snapshot,
    Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) receiver: PointId,
    pub(super) method: MethodKind,
    pub(super) load: bool,
    pub(super) control: bool,
    pub(super) kind: Kind,
}

impl Checker {
    pub(super) fn method_effect_stage(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let budget = || Diagnostic::unsupported("proof method-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof method-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. } | Port::Snapshot(point) | Port::Operation(point) => {
                point
            }
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.methods.len().checked_ilog2().unwrap_or(0) as usize + 18)
        {
            return Err(budget());
        }
        let Some(op) = self.methods.get(&id) else {
            return Ok(None);
        };
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.kind == MethodKind::Stopped
            || (op.load && op.kind == MethodKind::StringSize)
        {
            return Err(invalid());
        }
        let item = if let MethodKind::Add {
            item,
            capacity,
            length,
            ..
        } = op.kind
        {
            if item == op.receiver
                || capacity > crate::list::MAX_CAPACITY
                || length.is_some_and(|length| length > capacity)
            {
                return Err(invalid());
            }
            Some(item)
        } else {
            None
        };
        for root in std::iter::once(op.receiver).chain(item) {
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
        let entry = Edge::new(Port::Entry(id), Port::Entry(op.receiver), Route::Next);
        let mut edges = [entry; 6];
        let mut count = 1;
        let mut from = Port::Normal(op.receiver);
        if op.load {
            let load = Port::Projection { point: id, step: 0 };
            edges[count] = Edge::new(from, load, Route::Next);
            count += 1;
            from = load;
        }
        match op.kind {
            MethodKind::Add {
                item, may_return, ..
            } => {
                edges[count] = Edge::new(from, Port::Snapshot(id), Route::Next);
                edges[count + 1] = Edge::new(Port::Snapshot(id), Port::Entry(item), Route::Next);
                count += 2;
                if may_return {
                    edges[count] =
                        Edge::new(Port::Normal(item), Port::Operation(id), Route::Checked);
                    edges[count + 1] =
                        Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
                    count += 2;
                }
            }
            MethodKind::ListSize | MethodKind::StringSize => {
                edges[count] = Edge::new(from, Port::Operation(id), Route::Next);
                edges[count + 1] = Edge::new(Port::Operation(id), Port::Normal(id), Route::Next);
                count += 2;
            }
            MethodKind::Stopped => return Err(invalid()),
        }
        if op.edges.as_slice() != &edges[..count] {
            return Err(invalid());
        }
        let kind = match port {
            Port::Projection { step: 0, .. } if op.load => Kind::Load,
            Port::Snapshot(_) if item.is_some() => Kind::Snapshot,
            Port::Operation(_)
                if !matches!(
                    op.kind,
                    MethodKind::Add {
                        may_return: false,
                        ..
                    }
                ) =>
            {
                Kind::Finish
            }
            _ => return Err(invalid()),
        };
        if kind == Kind::Finish {
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
            method: op.kind,
            load: op.load,
            control: op.control,
            kind,
        }))
    }
}

#[cfg(test)]
mod tests;
