use super::*;
use std::collections::BTreeSet;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Walk {
    pub(crate) ports: Vec<Port>,
    pub(crate) forward: Vec<usize>,
    pub(crate) backedges: Vec<usize>,
    pub(crate) missing: Vec<Port>,
}

impl Walk {
    pub(super) fn len(&self) -> usize {
        self.ports.len() + self.forward.len() + self.backedges.len() + self.missing.len()
    }
}

impl ForwardIndex {
    #[cfg(test)]
    pub(crate) fn walk(
        &self,
        start: Port,
        flow: &mut crate::flow::Flow,
        span: Span,
    ) -> Result<Walk> {
        self.walk_limited(start, flow, span, MAX_EDGES * 3 + 2)
    }

    pub(super) fn walk_limited(
        &self,
        start: Port,
        flow: &mut crate::flow::Flow,
        span: Span,
        items: usize,
    ) -> Result<Walk> {
        let budget = || Diagnostic::unsupported("proof structural-walk budget exhausted", span);
        if self.edges.len() > MAX_EDGES || items == 0 || !flow.spend(1) {
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
                if walk.len() >= items {
                    return Err(budget());
                }
                walk.missing.push(port);
                continue;
            };
            for &position in links.forward.iter().chain(&links.backedges) {
                if walk.forward.len() + walk.backedges.len() >= self.edges.len()
                    || walk.len() >= items
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
                    if walk.ports.len() >= limit || walk.len() >= items {
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod coverage;
