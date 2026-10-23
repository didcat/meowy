use super::*;

mod absent;
mod bodies;
mod regions;

#[derive(Debug, Default, PartialEq, Eq)]
pub(in super::super) struct Completion {
    pub(in super::super) absent: HashMap<Port, usize>,
    pub(in super::super) leaves: HashMap<hir::BlockId, Option<bool>>,
    pub(in super::super) bodies: HashMap<hir::BlockId, bodies::Body>,
}

impl Checker {
    pub(in super::super) fn scalar_completion_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        let (sources, mut parts) = self.scalar_exit_sources(span, parts)?;
        let mut groups = HashMap::new();
        for (&point, leave) in &sources.leaves {
            let edge = Edge::new(Port::Entry(point), Port::Leave(leave.target), Route::Exit);
            self.scalar_completion_edge(reports, Family::ScopeExit, edge, span)?;
            if !self.flow.spend(5) {
                return Err(budget(span));
            }
            let port = Port::Normal(point);
            if graph.nodes.contains_key(&port) || observations.completion.absent.contains_key(&port)
            {
                return Err(invalid(span));
            }
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            observations.completion.absent.insert(port, leave.owner);
            if let Some(region) =
                self.scalar_stopped_region(reports, graph, point, leave.owner, span)?
            {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                if observations
                    .completion
                    .absent
                    .insert(Port::Normal(region), leave.owner)
                    .is_some()
                {
                    return Err(invalid(span));
                }
            }
            let control = super::super::super::controls::dependency(
                point,
                leave.owner,
                leave.control,
                &self.points,
                &self.branch_inputs,
                &mut self.flow,
            )
            .map_err(|error| error.diagnostic("scalar-completion", span))?;
            if !groups.contains_key(&leave.target) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            }
            let (count, combined) = groups.entry(leave.target).or_insert((0usize, Some(false)));
            *count += 1;
            *combined = match (*combined, control) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
        }
        for &target in &sources.targets {
            if !self.flow.spend(4) {
                return Err(budget(span));
            }
            let port = Port::Leave(target);
            let Some(node) = graph.nodes.get(&port) else {
                continue;
            };
            let owner = node.site.owner;
            if node.site != self.scalar_port_site(port, owner, span)? {
                return Err(invalid(span));
            }
            let control =
                if sources.unknown.contains(&target) || sources.incomplete.contains(&owner) {
                    None
                } else {
                    let &(count, control) = groups.get(&target).ok_or_else(|| invalid(span))?;
                    if node.incoming.len() != count {
                        return Err(invalid(span));
                    }
                    for input in &node.incoming {
                        if !self
                            .flow
                            .spend(sources.leaves.len().checked_ilog2().unwrap_or(0) as usize + 6)
                        {
                            return Err(budget(span));
                        }
                        let (family, edge) =
                            graph.edges.get(input.edge).ok_or_else(|| invalid(span))?;
                        let Port::Entry(point) = edge.from else {
                            return Err(invalid(span));
                        };
                        let leave = sources.leaves.get(&point).ok_or_else(|| invalid(span))?;
                        if *family != Family::ScopeExit
                            || *edge != Edge::new(Port::Entry(point), port, Route::Exit)
                            || leave.owner != owner
                            || leave.target != target
                            || input.source != self.scalar_port_site(edge.from, owner, span)?
                            || input.reached != graph.nodes.contains_key(&edge.from)
                        {
                            return Err(invalid(span));
                        }
                    }
                    control
                };
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if observations
                .completion
                .leaves
                .insert(target, control)
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        if !self.flow.spend(observations.scopes.len() + 1) {
            return Err(budget(span));
        }
        for (&id, scope) in &observations.scopes {
            if !self.flow.spend(4) {
                return Err(budget(span));
            }
            if !scope.normal
                || sources.targets.contains(&id)
                || sources.incomplete.contains(&scope.owner)
            {
                continue;
            }
            let port = Port::Leave(id);
            if graph.nodes.contains_key(&port) || observations.completion.absent.contains_key(&port)
            {
                return Err(invalid(span));
            }
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            observations.completion.absent.insert(port, scope.owner);
        }
        let (observations, parts) =
            self.scalar_completion_bodies(reports, graph, &sources, observations, span, parts)?;
        self.scalar_absent_paths(reports, graph, &sources, observations, span, parts)
    }

    pub(super) fn scalar_completion_edge(
        &mut self,
        reports: &Reports,
        family: Family,
        edge: Edge,
        span: Span,
    ) -> Result<()> {
        if !self.flow.spend(4) {
            return Err(budget(span));
        }
        let links = reports
            .index
            .outgoing
            .get(&edge.from)
            .ok_or_else(|| invalid(span))?;
        let [position] = links.forward.as_slice() else {
            return Err(invalid(span));
        };
        if !links.backedges.is_empty()
            || reports.index.edges.get(*position) != Some(&(family, edge))
        {
            return Err(invalid(span));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
