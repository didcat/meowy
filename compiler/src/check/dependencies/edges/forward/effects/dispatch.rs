use super::*;
use crate::check::dependencies::{SequenceSource, bodies::Fact};

mod report;
pub(crate) use report::Observed;

#[cfg(test)]
mod results;

impl Checker {
    pub(super) fn validate_dispatch_effect(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let budget = || Diagnostic::unsupported("proof dispatch-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof dispatch-effect identity mismatch", span);
        let id = match port {
            Port::Operation(id) | Port::Normal(id) => id,
            _ => return Ok(false),
        };
        if !self
            .flow
            .spend(self.dispatch_ops.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some(op) = self.dispatch_ops.get(&id) else {
            return Ok(false);
        };
        let work = [
            self.bodies.len(),
            self.sequences.len(),
            self.endpoints.len(),
            self.endpoints.len(),
            self.proofs.dispatches.len(),
            self.proofs.receivers.len(),
            self.sites.len(),
            self.sites.len(),
            reports.index.operations.len(),
        ]
        .into_iter()
        .fold(40, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        let body = self.bodies.get(&op.block).ok_or_else(invalid)?;
        let key = SequenceSource::Block(op.block);
        let sequence = self.sequences.get(&key).ok_or_else(invalid)?;
        let ends = self.endpoints.get(&key).ok_or_else(invalid)?;
        if !op.input_normal
            || (!op.normal && port == Port::Normal(id))
            || op.owner != owner
            || point.owner != owner
            || !point.complete
            || point.kind != PointKind::Expr
            || point.span != op.span
            || op.input == id
            || op.local >= reports.locals
            || body.owner != owner
            || body.parent != Some(id)
            || !self.proofs.dispatches.contains(&op.block)
            || !self.proofs.receivers.contains(&op.local)
            || !body
                .facts
                .first()
                .is_some_and(|(fact, _)| matches!(fact, Fact::Bind(local) if *local == op.local))
            || body.links.first() != Some(&None)
            || body.sources.first() != Some(&None)
            || body.storage.first() != Some(&Some(op.local))
            || body.links.len() != body.facts.len()
            || body.sources.len() != body.facts.len()
            || body.storage.len() != body.facts.len()
            || sequence.owner != owner
            || sequence.items.first() != Some(&None)
            || self.endpoints.contains_key(&SequenceSource::Expr(id))
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
        for id in [
            sequence.items.get(1).copied().flatten(),
            sequence.items.last().copied().flatten(),
        ]
        .into_iter()
        .flatten()
        {
            let point = self.points.get(id).ok_or_else(invalid)?;
            let site = point.site.ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || point.kind != PointKind::Stmt
                || point.block != Some(op.block)
                || site >= self.statements
                || !self.sites.get(&site).is_some_and(|site| {
                    site.complete
                        && site.owner == owner
                        && site.block == Some(op.block)
                        && site.point == Some(id)
                        && site.span == point.span
                })
            {
                return Err(invalid());
            }
        }
        let mut ends = ends.iter();
        if ends.next()
            != Some(&Edge::new(
                Port::Leave(op.block),
                Port::BlockNormal(op.block),
                Route::Join,
            ))
        {
            return Err(invalid());
        }
        if let Some(Some(last)) = sequence.items.last()
            && ends.next()
                != Some(&Edge::new(
                    Port::Normal(*last),
                    Port::BlockNormal(op.block),
                    Route::Next,
                ))
        {
            return Err(invalid());
        }
        if op.normal
            && ends.next()
                != Some(&Edge::new(
                    Port::BlockNormal(op.block),
                    Port::BlockResult(op.block),
                    Route::Result,
                ))
        {
            return Err(invalid());
        }
        if ends.next().is_some() {
            return Err(invalid());
        }
        let mut edges = op.edges.iter();
        for edge in [
            Edge::new(Port::Entry(id), Port::BlockEntry(op.block), Route::Next),
            Edge::new(
                Port::BlockEntry(op.block),
                Port::Entry(op.input),
                Route::Next,
            ),
            Edge::new(Port::Normal(op.input), Port::Operation(id), Route::Next),
        ] {
            if edges.next() != Some(&edge) {
                return Err(invalid());
            }
        }
        let next = match sequence.items.get(1) {
            Some(Some(next)) => Some(Port::Entry(*next)),
            None => Some(Port::BlockNormal(op.block)),
            Some(None) => None,
        };
        if let Some(next) = next
            && edges.next() != Some(&Edge::new(Port::Operation(id), next, Route::Next))
        {
            return Err(invalid());
        }
        if op.normal
            && edges.next()
                != Some(&Edge::new(
                    Port::BlockResult(op.block),
                    Port::Normal(id),
                    Route::Result,
                ))
        {
            return Err(invalid());
        }
        if edges.next().is_some() {
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

#[cfg(test)]
mod limits;
