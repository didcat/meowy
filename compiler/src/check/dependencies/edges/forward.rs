use super::{inventory::Family, *};
use std::collections::BTreeMap;

mod entries;
mod walk;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Links {
    pub(crate) forward: Vec<usize>,
    pub(crate) backedges: Vec<usize>,
}

#[derive(Debug)]
pub(crate) struct ForwardIndex {
    pub(crate) edges: Vec<(Family, Edge)>,
    pub(crate) outgoing: BTreeMap<Port, Links>,
    pub(crate) operations: BTreeMap<PointId, usize>,
}

impl ForwardIndex {
    pub(self) fn build(
        edges: Vec<(Family, Edge)>,
        flow: &mut crate::flow::Flow,
        span: Span,
    ) -> Result<Self> {
        let budget = || Diagnostic::unsupported("proof forward-index budget exhausted", span);
        if edges.len() > MAX_EDGES || !flow.spend(edges.len() + 1) {
            return Err(budget());
        }
        let mut index = Self {
            edges,
            outgoing: BTreeMap::new(),
            operations: BTreeMap::new(),
        };
        for (position, (_, edge)) in index.edges.iter().enumerate() {
            if !flow.spend(index.outgoing.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3) {
                return Err(budget());
            }
            let links = index.outgoing.entry(edge.from).or_default();
            if edge.route == Route::Backedge {
                links.backedges.push(position);
            } else {
                links.forward.push(position);
            }
        }
        Ok(index)
    }
}

impl Checker {
    pub(crate) fn forward_index(&mut self, span: Span) -> Result<ForwardIndex> {
        let edges = self.edge_inventory(span)?;
        let operations = self.validate_edge_ports(&edges, span)?;
        let mut index = ForwardIndex::build(edges, &mut self.flow, span)?;
        index.operations = operations;
        Ok(index)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod coverage;
