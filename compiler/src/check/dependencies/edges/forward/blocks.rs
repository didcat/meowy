use super::{entries::Reports, *};
use crate::check::dependencies::{SequenceSource, sequences::MAX_ITEMS};
use std::collections::BTreeSet;

mod report;
pub(crate) use report::Blocks;

impl Checker {
    pub(super) fn validate_block_effect(
        &mut self,
        reports: &Reports,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<bool> {
        let id = match port {
            Port::BlockNormal(id) | Port::BlockResult(id) => id,
            _ => return Ok(false),
        };
        let budget = || Diagnostic::unsupported("proof block-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof block-effect identity mismatch", span);
        let work = [
            self.bodies.len(),
            self.sequences.len(),
            self.endpoints.len(),
            self.endpoints.len(),
            self.proofs.dispatches.len(),
            reports.entries.len(),
        ]
        .into_iter()
        .fold(32, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        let body = self.bodies.get(&id).ok_or_else(invalid)?;
        if self.proofs.dispatches.contains(&id) {
            return Ok(false);
        }
        let key = SequenceSource::Block(id);
        let sequence = self.sequences.get(&key).ok_or_else(invalid)?;
        if id >= self.block
            || body.owner != owner
            || body.span.start > body.span.end
            || !body.completion.valid()
            || (port == Port::BlockResult(id) && !body.completion.normal)
            || sequence.owner != owner
        {
            return Err(invalid());
        }
        if let Some(parent) = body.parent {
            let point = self.points.get(parent).ok_or_else(invalid)?;
            let links = [
                Edge::new(Port::Entry(parent), Port::BlockEntry(id), Route::Next),
                Edge::new(Port::BlockResult(id), Port::Normal(parent), Route::Result),
            ];
            if !point.complete
                || point.kind != PointKind::Expr
                || point.owner != owner
                || point.block == Some(id)
                || point.span.start > body.span.start
                || point.span.end < body.span.end
                || self
                    .endpoints
                    .get(&SequenceSource::Expr(parent))
                    .map(Vec::as_slice)
                    != Some(links.as_slice())
            {
                return Err(invalid());
            }
        } else if reports.entries.get(&owner).map(|(block, _)| *block) != Some(id) {
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
                || point.block != Some(id)
                || point.parent != body.parent
                || point.span.start > point.span.end
                || point.span.start < body.span.start
                || point.span.end > body.span.end
                || site >= self.statements
                || !seen.insert(item)
                || !self.sites.get(&site).is_some_and(|site| {
                    site.complete
                        && site.owner == owner
                        && site.block == Some(id)
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
        let mut ends = [Edge::new(Port::Leave(id), Port::BlockNormal(id), Route::Join); 4];
        let mut count = 1;
        if sequence.items.is_empty() {
            ends[count] = Edge::new(Port::BlockEntry(id), Port::BlockNormal(id), Route::Next);
            count += 1;
        } else {
            if let Some(Some(first)) = sequence.items.first() {
                ends[count] = Edge::new(Port::BlockEntry(id), Port::Entry(*first), Route::Next);
                count += 1;
            }
            if let Some(Some(last)) = sequence.items.last() {
                ends[count] = Edge::new(Port::Normal(*last), Port::BlockNormal(id), Route::Next);
                count += 1;
            }
        }
        if body.completion.normal {
            ends[count] = Edge::new(Port::BlockNormal(id), Port::BlockResult(id), Route::Result);
            count += 1;
        }
        if self.endpoints.get(&key).map(Vec::as_slice) != Some(&ends[..count]) {
            return Err(invalid());
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod validation;
