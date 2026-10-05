use super::*;
use std::collections::BTreeSet;

impl Checker {
    pub(super) fn receiver_scope(
        &mut self,
        mut id: PointId,
        target: crate::hir::BlockId,
        owner: usize,
        span: Span,
        limit: usize,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof receiver-scope budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof receiver-scope identity mismatch", span);
        let mut seen = BTreeSet::new();
        loop {
            if !self.flow.spend(
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.dispatches.len().checked_ilog2().unwrap_or(0) as usize
                    + seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 19,
            ) {
                return Err(budget());
            }
            if seen.contains(&id) {
                return Err(invalid());
            }
            if seen.len() >= limit.min(MAX_GROUPS) {
                return Err(budget());
            }
            seen.insert(id);
            let point = self.points.get(id).ok_or_else(invalid)?;
            let block = point.block.ok_or_else(invalid)?;
            let body = self.bodies.get(&block).ok_or_else(invalid)?;
            let next = point.parent.ok_or_else(invalid)?;
            let parent = self.points.get(next).ok_or_else(invalid)?;
            if !point.complete
                || point.owner != owner
                || body.owner != owner
                || point.span.start > point.span.end
                || body.span.start > point.span.start
                || body.span.end < point.span.end
                || !parent.complete
                || parent.owner != owner
                || parent.span.start > point.span.start
                || parent.span.end < point.span.end
            {
                return Err(invalid());
            }
            if body.parent == Some(next) {
                if parent.span.start > body.span.start
                    || parent.span.end < body.span.end
                    || parent.block == Some(block)
                {
                    return Err(invalid());
                }
                if block == target {
                    return Ok(());
                }
                if self.proofs.dispatches.contains(&block) {
                    return Err(invalid());
                }
            } else if parent.block != Some(block) {
                return Err(invalid());
            }
            id = next;
        }
    }
}
