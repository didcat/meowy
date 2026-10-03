use super::*;
use crate::check::dependencies::grouped::MAX_GROUPS;
use std::collections::BTreeSet;

impl Checker {
    pub(super) fn grouped_consumer(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        self.grouped_consumer_limited(reports, input, owner, span, MAX_GROUPS)
    }

    pub(super) fn grouped_consumer_limited(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
        limit: usize,
    ) -> Result<Option<PointId>> {
        let budget = || Diagnostic::unsupported("proof group-consumer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof group-consumer identity mismatch", span);
        if self.group_inputs.len() > MAX_GROUPS || !self.flow.spend(1) {
            return Err(budget());
        }
        let limit = limit.min(MAX_GROUPS);
        let mut seen = BTreeSet::new();
        let mut current = input;
        loop {
            if !self.flow.spend(
                reports.consumers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + self.coercions.len().checked_ilog2().unwrap_or(0) as usize
                    + 6,
            ) {
                return Err(budget());
            }
            let point = self.points.get(current).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.span.start > point.span.end
                || !matches!(point.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            {
                return Err(invalid());
            }
            let group = self.group_inputs.get(&current).copied();
            let coercion = self.coercions.contains_key(&current);
            if let Some(&(source_owner, _)) = reports.consumers.get(&current) {
                if group.is_some()
                    || coercion
                    || source_owner != owner
                    || point.kind != PointKind::Expr
                {
                    return Err(invalid());
                }
                return Ok(Some(current));
            }
            let block = point.block;
            let next = if let Some(group) = group {
                if coercion {
                    return Err(invalid());
                }
                if !self
                    .flow
                    .spend(self.region_edges.len().checked_ilog2().unwrap_or(0) as usize + 10)
                {
                    return Err(budget());
                }
                let child = self.points.get(group.input).ok_or_else(invalid)?;
                let edges = [
                    Edge::new(Port::Entry(current), Port::Entry(group.input), Route::Next),
                    Edge::new(
                        Port::Normal(group.input),
                        Port::Normal(current),
                        Route::Next,
                    ),
                ];
                if point.kind != PointKind::Expr
                    || group.owner != owner
                    || group.span != point.span
                    || group.block != point.block
                    || group.input == current
                    || !child.complete
                    || child.owner != owner
                    || child.parent != Some(current)
                    || child.block != point.block
                    || child.span.start > child.span.end
                    || child.span.start < point.span.start
                    || child.span.end > point.span.end
                    || !matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                    || self.region_edges.get(&current) != Some(&edges)
                {
                    return Err(invalid());
                }
                group.input
            } else {
                let Some(input) = self.forward_coercion_input(reports, current, owner, span)?
                else {
                    return Ok(None);
                };
                input
            };
            if !self.flow.spend(
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 6,
            ) {
                return Err(budget());
            }
            if !block
                .and_then(|block| self.bodies.get(&block))
                .is_some_and(|body| body.owner == owner)
                || seen.contains(&current)
            {
                return Err(invalid());
            }
            if seen.len() >= limit {
                return Err(budget());
            }
            seen.insert(current);
            current = next;
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod validation;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod mixed;

#[cfg(test)]
mod forward;
