use super::*;
use std::collections::BTreeSet;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Walk {
    pub(crate) ports: Vec<Port>,
    pub(crate) forward: Vec<usize>,
    pub(crate) backedges: Vec<usize>,
    pub(crate) missing: Vec<Port>,
}

impl ForwardIndex {
    pub(crate) fn walk(
        &self,
        start: Port,
        flow: &mut crate::flow::Flow,
        span: Span,
    ) -> Result<Walk> {
        let budget = || Diagnostic::unsupported("proof structural-walk budget exhausted", span);
        if self.edges.len() > MAX_EDGES || !flow.spend(1) {
            return Err(budget());
        }
        let limit = self.edges.len() + 1;
        let mut seen = BTreeSet::from([start]);
        let mut walk = Walk {
            ports: vec![start],
            ..Walk::default()
        };
        let mut cursor = 0;
        while cursor < walk.ports.len() {
            let port = walk.ports[cursor];
            if !flow.spend(self.outgoing.len().checked_ilog2().unwrap_or(0) as usize + 2) {
                return Err(budget());
            }
            cursor += 1;
            let Some(links) = self.outgoing.get(&port) else {
                walk.missing.push(port);
                continue;
            };
            for &position in links.forward.iter().chain(&links.backedges) {
                if walk.forward.len() + walk.backedges.len() >= self.edges.len()
                    || !flow.spend(seen.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
                {
                    return Err(budget());
                }
                let edge = self.edges[position].1;
                if edge.route == Route::Backedge {
                    walk.backedges.push(position);
                    continue;
                }
                walk.forward.push(position);
                if !seen.contains(&edge.to) {
                    if walk.ports.len() >= limit {
                        return Err(budget());
                    }
                    seen.insert(edge.to);
                    walk.ports.push(edge.to);
                }
            }
        }
        Ok(walk)
    }
}

impl Checker {
    pub(crate) fn structural_walk(
        &mut self,
        block: crate::hir::BlockId,
        span: Span,
    ) -> Result<Walk> {
        self.forward_index(span)?
            .walk(Port::BlockEntry(block), &mut self.flow, span)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod coverage;
