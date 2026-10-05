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
            let guarded = point.kind == PointKind::Then && parent.kind == PointKind::Match;
            if !point.complete
                || point.owner != owner
                || body.owner != owner
                || point.span.start > point.span.end
                || body.span.start > point.span.start
                || body.span.end < point.span.end
                || !parent.complete
                || parent.owner != owner
                || (!guarded
                    && (parent.span.start > point.span.start || parent.span.end < point.span.end))
            {
                return Err(invalid());
            }
            if guarded {
                if !self.flow.spend(
                    self.branch_edges.len().checked_ilog2().unwrap_or(0) as usize
                        + self.sites.len().checked_ilog2().unwrap_or(0) as usize
                        + 20,
                ) {
                    return Err(budget());
                }
                let edges = self.branch_edges.get(&next).ok_or_else(invalid)?;
                let Port::Entry(condition) = edges[0].to else {
                    return Err(invalid());
                };
                let condition_point = self.points.get(condition).ok_or_else(invalid)?;
                let site_id = point.site.ok_or_else(invalid)?;
                let site = self.sites.get(&site_id).ok_or_else(invalid)?;
                if parent.site != Some(site_id)
                    || condition_point.site != Some(site_id)
                    || site_id >= self.statements
                    || !site.complete
                    || site.owner != owner
                    || site.block != Some(block)
                    || site.span.start > point.span.start
                    || site.span.end < point.span.end
                    || site.span.start > parent.span.start
                    || site.span.end < parent.span.end
                    || !condition_point.complete
                    || condition_point.kind != PointKind::Condition
                    || condition_point.owner != owner
                    || condition_point.block != Some(block)
                    || condition_point.parent != Some(next)
                    || edges[0] != Edge::new(Port::Entry(next), Port::Entry(condition), Route::Next)
                    || edges[1] != Edge::new(Port::Normal(condition), Port::Entry(id), Route::True)
                {
                    return Err(invalid());
                }
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
