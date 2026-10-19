use super::*;
use std::collections::BTreeSet;

pub(super) type Orders = BTreeMap<usize, Vec<Port>>;

impl Checker {
    pub(super) fn scalar_schedule(
        &mut self,
        graph: &graph::Graph<'_>,
        entries: &state::Entries,
        span: Span,
        parts: usize,
    ) -> Result<(Orders, usize)> {
        if graph.nodes.len() > MAX_EDGES
            || entries.len() > super::super::entries::MAX_ENTRIES
            || !self.flow.spend(graph.nodes.len() + entries.len() + 1)
        {
            return Err(budget(span));
        }
        let parts = parts
            .min(MAX_EDGES)
            .checked_sub(graph.nodes.len() * 3 + entries.len())
            .ok_or_else(|| budget(span))?;
        let mut pending = HashMap::new();
        for (&port, node) in &graph.nodes {
            if node.incoming.len() > MAX_EDGES || !self.flow.spend(node.incoming.len() * 5 + 1) {
                return Err(budget(span));
            }
            let mut count = 0usize;
            for input in &node.incoming {
                let edge = graph.edges.get(input.edge).ok_or_else(|| invalid(span))?.1;
                let source = graph.nodes.get(&edge.from);
                let site = source
                    .map(|node| &node.site)
                    .or_else(|| graph.external.get(&edge.from));
                if edge.to != port
                    || input.reached != source.is_some()
                    || site != Some(&input.source)
                    || input.source.owner != node.site.owner
                {
                    return Err(invalid(span));
                }
                count += usize::from(input.reached && edge.route != Route::Backedge);
            }
            pending.insert(port, count);
        }
        let mut ready = BTreeSet::new();
        let mut orders = Orders::new();
        for (&owner, (port, state)) in entries {
            if !self
                .flow
                .spend(entries.len().checked_ilog2().unwrap_or(0) as usize * 2 + 5)
            {
                return Err(budget(span));
            }
            let node = graph.nodes.get(port).ok_or_else(|| invalid(span))?;
            if *port != Port::BlockEntry(node.site.block)
                || node.site.owner != owner
                || state.owner() != owner
                || pending.get(port) != Some(&0)
            {
                return Err(invalid(span));
            }
            ready.insert(key(*port, node.site));
            orders.insert(owner, Vec::new());
        }
        let mut seen = 0;
        while !ready.is_empty() {
            if !self.flow.spend(
                ready.len().checked_ilog2().unwrap_or(0) as usize
                    + orders.len().checked_ilog2().unwrap_or(0) as usize
                    + 4,
            ) {
                return Err(budget(span));
            }
            let (owner, _, _, port) = ready.pop_first().ok_or_else(|| invalid(span))?;
            let node = graph.nodes.get(&port).ok_or_else(|| invalid(span))?;
            orders
                .get_mut(&owner)
                .ok_or_else(|| invalid(span))?
                .push(port);
            seen += 1;
            for &position in &node.outgoing {
                if !self
                    .flow
                    .spend(ready.len().checked_ilog2().unwrap_or(0) as usize + 8)
                {
                    return Err(budget(span));
                }
                let edge = graph.edges.get(position).ok_or_else(|| invalid(span))?.1;
                if edge.from != port {
                    return Err(invalid(span));
                }
                if edge.route == Route::Backedge {
                    continue;
                }
                let count = pending.get_mut(&edge.to).ok_or_else(|| invalid(span))?;
                *count = count.checked_sub(1).ok_or_else(|| invalid(span))?;
                if *count == 0 {
                    let next = graph.nodes.get(&edge.to).ok_or_else(|| invalid(span))?;
                    ready.insert(key(edge.to, next.site));
                }
            }
        }
        if seen != graph.nodes.len() {
            return Err(invalid(span));
        }
        Ok((orders, parts))
    }
}

pub(super) fn key(port: Port, site: ports::Site) -> (usize, usize, usize, Port) {
    (site.owner, site.span.start, site.span.end, port)
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-schedule budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-schedule identity mismatch", span)
}

#[cfg(test)]
mod tests;
