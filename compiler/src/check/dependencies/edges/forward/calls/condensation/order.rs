use super::*;

impl Condensed {
    pub(crate) fn analysis_order(&self, flow: &mut Flow, span: Span) -> Result<Vec<usize>> {
        self.order_limited(flow, span, MAX_ENTRIES, MAX_EDGES)
    }

    pub(self) fn order_limited(
        &self,
        flow: &mut Flow,
        span: Span,
        nodes: usize,
        sites: usize,
    ) -> Result<Vec<usize>> {
        let budget = || Diagnostic::unsupported("proof call-order budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-order identity mismatch", span);
        let log = self.nodes.len().checked_ilog2().unwrap_or(0) as usize;
        if self.nodes.len() > nodes || !flow.spend(self.nodes.len() * (log * 2 + 4) + 1) {
            return Err(budget());
        }
        let mut remaining = vec![0usize; self.nodes.len()];
        let mut callers = vec![Vec::new(); self.nodes.len()];
        let mut seen = BTreeSet::new();
        for (caller, node) in self.nodes.iter().enumerate() {
            check_sites(&node.internal, &mut seen, sites, flow, span)?;
            for (&target, ids) in &node.targets {
                if target == caller || target >= self.nodes.len() || ids.is_empty() {
                    return Err(invalid());
                }
                check_sites(ids, &mut seen, sites, flow, span)?;
                remaining[caller] += 1;
                callers[target].push(caller);
            }
        }
        let mut ready: BTreeSet<_> = remaining
            .iter()
            .enumerate()
            .filter_map(|(id, count)| (*count == 0).then_some(id))
            .collect();
        let mut order = Vec::new();
        while let Some(id) = ready.pop_first() {
            if !flow.spend(log * 2 + 3) {
                return Err(budget());
            }
            order.push(id);
            for &caller in &callers[id] {
                if !flow.spend(log + 2) {
                    return Err(budget());
                }
                remaining[caller] = remaining[caller].checked_sub(1).ok_or_else(invalid)?;
                if remaining[caller] == 0 {
                    ready.insert(caller);
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(Diagnostic::unsupported("proof call-order cycle", span));
        }
        Ok(order)
    }
}

pub(super) fn check_sites(
    ids: &[hir::CallId],
    seen: &mut BTreeSet<hir::CallId>,
    limit: usize,
    flow: &mut Flow,
    span: Span,
) -> Result<()> {
    let budget = || Diagnostic::unsupported("proof call-order budget exhausted", span);
    if ids.len() > limit.saturating_sub(seen.len()) || !flow.spend(ids.len() + 1) {
        return Err(budget());
    }
    for site in ids {
        if !flow.spend(seen.len().checked_ilog2().unwrap_or(0) as usize + 1) {
            return Err(budget());
        }
        if !seen.insert(*site) {
            return Err(Diagnostic::unsupported(
                "proof call-order duplicate site",
                span,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
