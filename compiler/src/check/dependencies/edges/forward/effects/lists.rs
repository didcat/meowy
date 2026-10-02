use super::*;
use crate::check::dependencies::{CoercionKind, SequenceSource};
use std::collections::BTreeSet;

mod report;
pub(crate) use report::Observed;

impl Checker {
    pub(super) fn validate_list_construction(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof list-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof list-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. }
            | Port::Conversion { point, .. }
            | Port::Operation(point)
            | Port::Normal(point) => point,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.lists.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.lists.get(&id) else {
            return Ok(false);
        };
        if op.count > crate::list::MAX_CAPACITY
            || !self.flow.spend(
                op.count * (op.count.checked_ilog2().unwrap_or(0) as usize + 12)
                    + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize
                    + self.list_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                    + 16,
            )
        {
            return Err(budget());
        }
        let key = SequenceSource::Expr(id);
        let point = self.points.get(id).ok_or_else(invalid)?;
        let sequence = self.sequences.get(&key).ok_or_else(invalid)?;
        let ends = self.endpoints.get(&key).ok_or_else(invalid)?;
        let inputs = self.list_inputs.get(&id);
        if op.owner != owner
            || point.owner != owner
            || sequence.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.capacity > crate::list::MAX_CAPACITY
            || op.count > op.capacity
            || op.count != sequence.items.len()
            || op.contextual != inputs.is_some()
            || inputs.is_some_and(|inputs| inputs.len() != op.count)
        {
            return Err(invalid());
        }
        let mut seen = BTreeSet::new();
        for (part, child) in sequence.items.iter().enumerate() {
            let child = child.ok_or_else(invalid)?;
            if child == id
                || !seen.insert(child)
                || inputs.is_some_and(|inputs| inputs[part].point != child)
                || !self.points.get(child).is_some_and(|child| {
                    child.complete
                        && child.owner == owner
                        && child.parent == Some(id)
                        && child.block == point.block
                        && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                })
            {
                return Err(invalid());
            }
        }
        let mut edges = sequence.edges.iter();
        let mut ends = ends.iter();
        let mut from = Port::Entry(id);
        let mut observed = false;
        let mut stopped = false;
        if let Some(inputs) = inputs {
            if (!op.normal
                && !inputs
                    .iter()
                    .any(|input| input.kind == CoercionKind::Stopped))
                || (op.normal
                    && inputs
                        .iter()
                        .any(|input| input.kind == CoercionKind::Stopped && !input.primary))
            {
                return Err(invalid());
            }
            for (part, input) in inputs.iter().enumerate() {
                let edge = Edge::new(from, Port::Entry(input.point), Route::Next);
                if (if part == 0 { ends.next() } else { edges.next() }) != Some(&edge) {
                    return Err(invalid());
                }
                from = Port::Normal(input.point);
                if input.primary {
                    let to = Port::Projection {
                        point: id,
                        step: part,
                    };
                    if ends.next() != Some(&Edge::new(from, to, Route::Next)) {
                        return Err(invalid());
                    }
                    observed |= port == to;
                    from = to;
                }
                if input.kind == CoercionKind::Stopped {
                    stopped = true;
                    break;
                }
                if input.kind == CoercionKind::Convert {
                    let to = Port::Conversion { point: id, part };
                    if ends.next() != Some(&Edge::new(from, to, Route::Next)) {
                        return Err(invalid());
                    }
                    observed |= port == to;
                    from = to;
                }
            }
        } else {
            for (part, child) in sequence.items.iter().enumerate() {
                let child = child.unwrap();
                let edge = Edge::new(from, Port::Entry(child), Route::Next);
                if (if part == 0 { ends.next() } else { edges.next() }) != Some(&edge) {
                    return Err(invalid());
                }
                from = Port::Normal(child);
            }
            if op.count == 0 && !op.normal {
                return Err(invalid());
            }
        }
        if op.normal && !stopped {
            if ends.next() != Some(&Edge::new(from, Port::Operation(id), Route::Next))
                || ends.next()
                    != Some(&Edge::new(
                        Port::Operation(id),
                        Port::Normal(id),
                        Route::Next,
                    ))
                || reports.index.operations.get(&id) != Some(&owner)
            {
                return Err(invalid());
            }
            observed |= matches!(port, Port::Operation(_) | Port::Normal(_));
        }
        if edges.next().is_some() || ends.next().is_some() || !observed {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod validation;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
