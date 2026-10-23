use super::super::super::exits::Sources;
use super::*;
use std::collections::{HashSet, VecDeque, hash_map::Entry};

#[derive(Default)]
pub(super) struct Node {
    pub(self) left: usize,
    pub(self) next: Vec<Port>,
}

impl Checker {
    pub(super) fn scalar_absent_paths<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        sources: &Sources,
        mut observations: Observations<'a, 'd>,
        span: Span,
        mut parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        parts = parts.min(MAX_EDGES);
        if !self.flow.spend(1) {
            return Err(budget(span));
        }
        if observations.completion.bodies.is_empty() {
            return Ok((observations, parts));
        }
        let count = reports.index.edges.len();
        if count > MAX_EDGES || !self.flow.spend(count * 2 + 2) {
            return Err(budget(span));
        }
        parts = parts.checked_sub(count).ok_or_else(|| budget(span))?;
        let mut edges = self.edge_inventory(span)?;
        if edges != reports.index.edges || edges.as_slice() != graph.edges {
            return Err(invalid(span));
        }
        let mut owners = HashSet::new();
        for (&block, body) in &observations.completion.bodies {
            if !self.flow.spend(2) {
                return Err(budget(span));
            }
            if owners.insert(body.owner) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            }
            if !body.normal {
                if edges.len() >= MAX_EDGES {
                    return Err(budget(span));
                }
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                edges.push((
                    Family::Endpoint,
                    Edge::new(
                        Port::BlockNormal(block),
                        Port::BlockResult(block),
                        Route::Result,
                    ),
                ));
            }
        }
        let mut nodes = HashMap::new();
        for (_, edge) in edges {
            if !self.flow.spend(6) {
                return Err(budget(span));
            }
            if graph.nodes.contains_key(&edge.to) {
                continue;
            }
            let target = node(&mut nodes, edge.to, &mut parts, span)?;
            target.left = target
                .left
                .checked_add(1)
                .filter(|n| *n <= MAX_EDGES)
                .ok_or_else(|| budget(span))?;
            if edge.route != Route::Backedge && !graph.nodes.contains_key(&edge.from) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                node(&mut nodes, edge.from, &mut parts, span)?
                    .next
                    .push(edge.to);
            }
        }
        let mut queue = VecDeque::new();
        for (&port, &owner) in &observations.completion.absent {
            if !self.flow.spend(2) {
                return Err(budget(span));
            }
            if owners.contains(&owner) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                queue.push_back((port, owner));
            }
        }
        while let Some((port, owner)) = queue.pop_front() {
            if !self.flow.spend(2) {
                return Err(budget(span));
            }
            if graph.nodes.contains_key(&port) {
                return Err(invalid(span));
            }
            let site = self.scalar_port_site(port, owner, span)?;
            if graph
                .external
                .get(&port)
                .is_some_and(|found| *found != site)
            {
                return Err(invalid(span));
            }
            let Some(source) = nodes.remove(&port) else {
                continue;
            };
            for next in source.next {
                if !self.flow.spend(5) {
                    return Err(budget(span));
                }
                if let Some(found) = observations.completion.absent.get(&next) {
                    if *found != owner {
                        return Err(invalid(span));
                    }
                    continue;
                }
                let target = nodes.get_mut(&next).ok_or_else(|| invalid(span))?;
                target.left = target.left.checked_sub(1).ok_or_else(|| invalid(span))?;
                if target.left != 0
                    || matches!(next, Port::BlockNormal(block) if !observations.completion.bodies.contains_key(&block))
                    || matches!(next, Port::Leave(block) if sources.unknown.contains(&block))
                {
                    continue;
                }
                self.scalar_port_site(next, owner, span)?;
                parts = parts.checked_sub(2).ok_or_else(|| budget(span))?;
                observations.completion.absent.insert(next, owner);
                queue.push_back((next, owner));
            }
        }
        Ok((observations, parts))
    }
}

pub(super) fn node<'a>(
    nodes: &'a mut HashMap<Port, Node>,
    port: Port,
    parts: &mut usize,
    span: Span,
) -> Result<&'a mut Node> {
    match nodes.entry(port) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => {
            *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            Ok(entry.insert(Node::default()))
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
