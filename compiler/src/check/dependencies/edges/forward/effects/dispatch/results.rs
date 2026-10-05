use super::*;
use crate::check::dependencies::sequences::MAX_ITEMS;
use std::collections::BTreeSet;

impl Checker {
    pub(in super::super::super) fn dispatch_result_body(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<crate::hir::BlockId>> {
        let budget = || Diagnostic::unsupported("proof dispatch-result budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof dispatch-result identity mismatch", span);
        if !self.flow.spend(
            reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                + self.dispatch_ops.len().checked_ilog2().unwrap_or(0) as usize
                + 9,
        ) {
            return Err(budget());
        }
        let Some((reported, Effect::Dispatch(observed))) = reports.effects.get(&id) else {
            return Ok(None);
        };
        let op = self.dispatch_ops.get(&id).ok_or_else(invalid)?;
        if *reported != owner
            || op.owner != owner
            || observed.input != op.input
            || observed.local != op.local
            || observed.block != op.block
            || observed.normal != op.normal
            || observed.control != op.control
        {
            return Err(invalid());
        }
        let block = op.block;
        let port = if observed.result {
            Port::Normal(id)
        } else if observed.initialized {
            Port::Operation(id)
        } else {
            return Ok(None);
        };
        if !self.validate_dispatch_effect(reports, owner, port, span)? {
            return Err(invalid());
        }
        if !observed.result {
            return Ok(None);
        }
        if !self.flow.spend(
            self.bodies.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                + 7,
        ) {
            return Err(budget());
        }
        let body = self.bodies.get(&block).ok_or_else(invalid)?;
        let point = self.points.get(id).ok_or_else(invalid)?;
        let sequence = self
            .sequences
            .get(&SequenceSource::Block(block))
            .ok_or_else(invalid)?;
        if !body.completion.valid()
            || !body.completion.normal
            || body.span.start > body.span.end
            || point.span.start > point.span.end
            || body.span.start < point.span.start
            || body.span.end > point.span.end
            || !point
                .block
                .filter(|container| *container != block)
                .and_then(|container| self.bodies.get(&container))
                .is_some_and(|container| container.owner == owner)
        {
            return Err(invalid());
        }
        if sequence.items.len() > MAX_ITEMS
            || !self.flow.spend(
                sequence.items.len()
                    * (self.sites.len().checked_ilog2().unwrap_or(0) as usize
                        + sequence.items.len().checked_ilog2().unwrap_or(0) as usize
                        + 10)
                    + 8,
            )
        {
            return Err(budget());
        }
        let mut seen = BTreeSet::new();
        for &item in sequence.items.iter().flatten() {
            let point = self.points.get(item).ok_or_else(invalid)?;
            let site = point.site.ok_or_else(invalid)?;
            if !point.complete
                || point.kind != PointKind::Stmt
                || point.owner != owner
                || point.block != Some(block)
                || point.parent != Some(id)
                || point.span.start > point.span.end
                || point.span.start < body.span.start
                || point.span.end > body.span.end
                || site >= self.statements
                || !seen.insert(item)
                || !self.sites.get(&site).is_some_and(|site| {
                    site.complete
                        && site.owner == owner
                        && site.block == Some(block)
                        && site.point == Some(item)
                        && site.span == point.span
                })
            {
                return Err(invalid());
            }
        }
        let links = sequence.items.windows(2).filter_map(|pair| {
            Some(Edge::new(
                Port::Normal(pair[0]?),
                Port::Entry(pair[1]?),
                Route::Next,
            ))
        });
        if !links.eq(sequence.edges.iter().copied()) {
            return Err(invalid());
        }
        self.validate_result_layout(block, span)?;
        Ok(Some(block))
    }
}

#[cfg(test)]
mod tests;
