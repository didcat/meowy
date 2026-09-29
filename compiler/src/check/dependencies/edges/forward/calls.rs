mod components;
mod condensation;
pub(super) use components::Components;
pub(super) use condensation::Condensed;

use super::{
    effects::Effect,
    entries::{MAX_ENTRIES, MAX_REPORT_ITEMS, Reports},
    *,
};
use crate::hir;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CallGraph {
    pub(crate) nodes: BTreeMap<usize, Node>,
    pub(crate) sites: BTreeMap<hir::CallId, Call>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) body: hir::BlockId,
    pub(crate) targets: BTreeMap<usize, Vec<hir::CallId>>,
    pub(crate) missing: Vec<Port>,
    pub(crate) backedges: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) point: PointId,
    pub(crate) caller: usize,
    pub(crate) callee: usize,
}

impl Checker {
    pub(super) fn call_graph(&mut self, reports: &Reports, span: Span) -> Result<CallGraph> {
        self.call_graph_limited(reports, span, MAX_ENTRIES, MAX_EDGES, MAX_REPORT_ITEMS)
    }

    pub(self) fn call_graph_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        nodes: usize,
        sites: usize,
        mut boundaries: usize,
    ) -> Result<CallGraph> {
        let budget = || Diagnostic::unsupported("proof call-graph budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-graph identity mismatch", span);
        if reports.entries.len() > nodes
            || reports.effects.len() > MAX_EDGES
            || !self
                .flow
                .spend(reports.entries.len() + reports.effects.len() + 1)
        {
            return Err(budget());
        }
        if !reports.entries.contains_key(&0) {
            return Err(invalid());
        }
        let mut graph = CallGraph::default();
        for (&owner, (body, walk)) in &reports.entries {
            let count = walk.missing.len().saturating_add(walk.backedges.len());
            if count > boundaries
                || !self.flow.spend(
                    count
                        + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                        + graph.nodes.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 4,
                )
            {
                return Err(budget());
            }
            if !self
                .bodies
                .get(body)
                .is_some_and(|body| body.owner == owner && body.parent.is_none())
                || walk.ports.first() != Some(&Port::BlockEntry(*body))
            {
                return Err(invalid());
            }
            for &port in &walk.missing {
                if self.port_owner(port, span)? != owner {
                    return Err(invalid());
                }
            }
            for &position in &walk.backedges {
                let edge = reports.index.edges.get(position).ok_or_else(invalid)?.1;
                if edge.route != Route::Backedge
                    || self.port_owner(edge.from, span)? != owner
                    || self.port_owner(edge.to, span)? != owner
                {
                    return Err(invalid());
                }
            }
            boundaries -= count;
            graph.nodes.insert(
                owner,
                Node {
                    body: *body,
                    targets: BTreeMap::new(),
                    missing: walk.missing.clone(),
                    backedges: walk.backedges.clone(),
                },
            );
        }
        for (&point, (caller, effect)) in &reports.effects {
            let Effect::Call { site, function, .. } = effect else {
                continue;
            };
            if graph.sites.len() >= sites
                || !self.flow.spend(
                    graph.nodes.len().checked_ilog2().unwrap_or(0) as usize * 3
                        + graph.sites.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                        + reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                        + 8,
                )
            {
                return Err(budget());
            }
            let callee = function.checked_add(1).ok_or_else(invalid)?;
            let call = self.invocations.get(site).ok_or_else(invalid)?;
            if !graph.nodes.contains_key(&callee)
                || !graph.nodes.contains_key(caller)
                || reports.index.operations.get(&point) != Some(caller)
                || call.site != *site
                || call.point != point
                || call.owner != *caller
                || call.function != *function
                || !self.points.get(point).is_some_and(|point| {
                    point.complete && point.kind == PointKind::Expr && point.owner == *caller
                })
                || graph.sites.contains_key(site)
            {
                return Err(invalid());
            }
            let node = graph.nodes.get_mut(caller).expect("validated caller");
            if !self
                .flow
                .spend(node.targets.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
            {
                return Err(budget());
            }
            node.targets.entry(callee).or_default().push(*site);
            graph.sites.insert(
                *site,
                Call {
                    point,
                    caller: *caller,
                    callee,
                },
            );
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod tests;
