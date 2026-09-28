use super::*;
use crate::hir;

impl Checker {
    pub(super) fn call_effect_index(
        &mut self,
        span: Span,
    ) -> Result<BTreeMap<PointId, hir::CallId>> {
        self.call_effect_index_limited(span, MAX_EDGES)
    }

    pub(self) fn call_effect_index_limited(
        &mut self,
        span: Span,
        limit: usize,
    ) -> Result<BTreeMap<PointId, hir::CallId>> {
        let budget = || Diagnostic::unsupported("proof call-effect index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if self.invocations.len() > limit || !self.flow.spend(self.invocations.len() + 1) {
            return Err(budget());
        }
        let mut calls = BTreeMap::new();
        for (&site, call) in &self.invocations {
            if !self.flow.spend(
                calls.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                    + 6,
            ) {
                return Err(budget());
            }
            if site != call.site
                || site >= self.calls
                || !self.proofs.calls.contains_key(&site)
                || !self.points.get(call.point).is_some_and(|point| {
                    point.kind == PointKind::Expr
                        && point.complete
                        && point.owner == call.owner
                        && point.span == call.span
                })
                || calls.insert(call.point, site).is_some()
            {
                return Err(invalid());
            }
        }
        Ok(calls)
    }
}

#[cfg(test)]
mod tests;
