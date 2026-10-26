use super::*;
use crate::check::dependencies::{SequenceSource, bodies::Scalar};

mod primary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct Scope {
    pub(in super::super) owner: usize,
    pub(in super::super) parent: Option<PointId>,
    pub(in super::super) scalar: Scalar,
    pub(in super::super) normal: bool,
    pub(in super::super) result: bool,
    pub(in super::super) control: Option<bool>,
}

impl Scope {
    pub(in super::super) fn declared(
        self,
        flow: &mut crate::flow::Flow,
        span: Span,
    ) -> Result<domain::Domain> {
        if self.scalar.shape().is_none() {
            return Err(invalid(span));
        }
        let value =
            domain::Domain::scalar(self.scalar.kind, flow, span)?.ok_or_else(|| invalid(span))?;
        if self.scalar.nullable {
            value.join(&domain::Domain::NULL, flow, span)
        } else {
            Ok(value)
        }
    }
}

impl Checker {
    pub(in super::super) fn scalar_scope_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if reports.blocks.len() > MAX_EDGES || !self.flow.spend(reports.blocks.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        for (&id, &(owner, observed)) in &reports.blocks {
            if !self
                .flow
                .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 5)
            {
                return Err(budget(span));
            }
            let body = self.bodies.get(&id).ok_or_else(|| invalid(span))?;
            if owner != body.owner
                || observed.parent != body.parent
                || observed.span != body.span
                || observed.completion != body.completion
                || observed.scalar != body.scalar
            {
                return Err(invalid(span));
            }
            let Some(scalar) = self.scalar_scope_type(id, span)? else {
                continue;
            };
            if observed.scalar.is_some() && scalar.shape() != Some(observed.completion.result) {
                return Err(invalid(span));
            }
            if !observed.normal && !observed.result
                || observed.normal != graph.nodes.contains_key(&Port::BlockNormal(id))
                || observed.result != graph.nodes.contains_key(&Port::BlockResult(id))
                || self.proofs.dispatches.contains(&id)
                || observed.parent.is_none()
                    && reports.entries.get(&owner).map(|entry| entry.0) != Some(id)
            {
                return Err(invalid(span));
            }
            self.validate_result_layout(id, span)?;
            let key = SequenceSource::Block(id);
            if !self.flow.spend(
                self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize
                    + 10,
            ) {
                return Err(budget(span));
            }
            let sequence = self.sequences.get(&key).ok_or_else(|| invalid(span))?;
            let mut ends = vec![Edge::new(
                Port::Leave(id),
                Port::BlockNormal(id),
                Route::Join,
            )];
            if sequence.items.is_empty() {
                ends.push(Edge::new(
                    Port::BlockEntry(id),
                    Port::BlockNormal(id),
                    Route::Next,
                ));
            } else {
                if let Some(Some(first)) = sequence.items.first() {
                    ends.push(Edge::new(
                        Port::BlockEntry(id),
                        Port::Entry(*first),
                        Route::Next,
                    ));
                }
                if let Some(Some(last)) = sequence.items.last() {
                    ends.push(Edge::new(
                        Port::Normal(*last),
                        Port::BlockNormal(id),
                        Route::Next,
                    ));
                }
            }
            ends.push(Edge::new(
                Port::BlockNormal(id),
                Port::BlockResult(id),
                Route::Result,
            ));
            if sequence.owner != owner || self.endpoints.get(&key) != Some(&ends) {
                return Err(invalid(span));
            }
            for port in [Port::BlockNormal(id), Port::BlockResult(id)] {
                self.scalar_scope_edges(graph, owner, port, Family::Endpoint, &ends, span)?;
            }
            let control = if let Some(parent) = observed.parent {
                let edge = Edge::new(Port::BlockResult(id), Port::Normal(parent), Route::Result);
                let links = [
                    Edge::new(Port::Entry(parent), Port::BlockEntry(id), Route::Next),
                    edge,
                ];
                if !self
                    .flow
                    .spend(self.endpoints.len().checked_ilog2().unwrap_or(0) as usize + 3)
                {
                    return Err(budget(span));
                }
                if self
                    .endpoints
                    .get(&SequenceSource::Expr(parent))
                    .map(Vec::as_slice)
                    != Some(links.as_slice())
                {
                    return Err(invalid(span));
                }
                self.scalar_scope_edges(graph, owner, edge.to, Family::Endpoint, &[edge], span)?;
                if graph.nodes.contains_key(&edge.to) {
                    if !self.flow.spend(6) {
                        return Err(budget(span));
                    }
                    if observations.sources.contains_key(&edge.to)
                        || observations.logical.contains_key(&edge.to)
                        || observations.forwarded.contains_key(&edge.to)
                        || observations.records.contains_key(&parent)
                    {
                        return Err(invalid(span));
                    }
                    parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                    if observed.scalar.is_none() {
                        if observations.records.insert(parent, id).is_some() {
                            return Err(invalid(span));
                        }
                    } else if observations
                        .forwarded
                        .insert(
                            edge.to,
                            Forward {
                                owner,
                                input: edge.from,
                                control: false,
                            },
                        )
                        .is_some()
                    {
                        return Err(invalid(span));
                    }
                }
                super::super::super::controls::dependency(
                    parent,
                    owner,
                    false,
                    &self.points,
                    &self.branch_inputs,
                    &mut self.flow,
                )
                .map_err(|error| error.diagnostic("scalar-scope", span))?
            } else {
                Some(false)
            };
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if observations
                .scopes
                .insert(
                    id,
                    Scope {
                        owner,
                        parent: observed.parent,
                        scalar,
                        normal: observed.normal,
                        result: observed.result,
                        control,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok((observations, parts))
    }

    pub(super) fn scalar_scope_edges(
        &mut self,
        graph: &graph::Graph<'_>,
        owner: usize,
        port: Port,
        family: Family,
        edges: &[Edge],
        span: Span,
    ) -> Result<()> {
        if edges.len() > MAX_EDGES || !self.flow.spend(edges.len() + 2) {
            return Err(budget(span));
        }
        let Some(node) = graph.nodes.get(&port) else {
            return Ok(());
        };
        if node.site != self.scalar_port_site(port, owner, span)?
            || node.incoming.len() != edges.iter().filter(|edge| edge.to == port).count()
        {
            return Err(invalid(span));
        }
        for edge in edges.iter().filter(|edge| edge.to == port) {
            if !self.flow.spend(node.incoming.len() * 3 + 3) {
                return Err(budget(span));
            }
            let source = graph.nodes.get(&edge.from);
            let site = source
                .map(|node| node.site)
                .or_else(|| graph.external.get(&edge.from).copied());
            if site != Some(self.scalar_port_site(edge.from, owner, span)?)
                || node
                    .incoming
                    .iter()
                    .filter(|input| {
                        graph.edges.get(input.edge) == Some(&(family, *edge))
                            && Some(input.source) == site
                            && input.reached == source.is_some()
                    })
                    .count()
                    != 1
            {
                return Err(invalid(span));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod nullable;
