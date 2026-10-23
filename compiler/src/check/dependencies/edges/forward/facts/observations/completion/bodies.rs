use super::super::super::exits::Sources;
use super::*;
use crate::check::dependencies::{SequenceSource, sequences::MAX_ITEMS};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super::super) struct Body {
    pub(in super::super::super) owner: usize,
    pub(in super::super::super) normal: bool,
    pub(in super::super::super) control: Option<bool>,
}

impl Checker {
    pub(super) fn scalar_completion_bodies<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        sources: &Sources,
        mut observations: Observations<'a, 'd>,
        span: Span,
        mut parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if !self.flow.spend(1) {
            return Err(budget(span));
        }
        if sources.leaves.is_empty() {
            return Ok((observations, parts));
        }
        let mut owners = HashSet::new();
        for leave in sources.leaves.values() {
            if !self.flow.spend(2) {
                return Err(budget(span));
            }
            if !sources.incomplete.contains(&leave.owner) && owners.insert(leave.owner) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            }
        }
        let mut ids = Vec::new();
        for (&id, body) in &self.bodies {
            if !self.flow.spend(1) {
                return Err(budget(span));
            }
            if owners.contains(&body.owner) {
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                ids.push(id);
            }
        }
        for id in ids {
            if !self.flow.spend(
                self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + self.endpoints.len().checked_ilog2().unwrap_or(0) as usize
                    + 5,
            ) {
                return Err(budget(span));
            }
            let body = self.bodies.get(&id).ok_or_else(|| invalid(span))?;
            let (owner, parent, normal) = (body.owner, body.parent, body.completion.normal);
            if !body.completion.valid() {
                return Err(invalid(span));
            }
            let key = SequenceSource::Block(id);
            let Some(sequence) = self.sequences.get(&key) else {
                continue;
            };
            if sequence.items.len() > MAX_ITEMS || !self.flow.spend(sequence.items.len() * 3 + 3) {
                return Err(budget(span));
            }
            if sequence.owner != owner {
                return Err(invalid(span));
            }
            if sequence.items.iter().any(Option::is_none) {
                continue;
            }
            if sequence.edges.len() != sequence.items.len().saturating_sub(1) {
                return Err(invalid(span));
            }
            for (pair, edge) in sequence.items.windows(2).zip(&sequence.edges) {
                if *edge
                    != Edge::new(
                        Port::Normal(pair[0].unwrap()),
                        Port::Entry(pair[1].unwrap()),
                        Route::Next,
                    )
                {
                    return Err(invalid(span));
                }
            }
            let mut edges = vec![Edge::new(
                Port::Leave(id),
                Port::BlockNormal(id),
                Route::Join,
            )];
            if let (Some(Some(first)), Some(Some(last))) =
                (sequence.items.first(), sequence.items.last())
            {
                edges.push(Edge::new(
                    Port::BlockEntry(id),
                    Port::Entry(*first),
                    Route::Next,
                ));
                edges.push(Edge::new(
                    Port::Normal(*last),
                    Port::BlockNormal(id),
                    Route::Next,
                ));
            } else {
                edges.push(Edge::new(
                    Port::BlockEntry(id),
                    Port::BlockNormal(id),
                    Route::Next,
                ));
            }
            if normal {
                edges.push(Edge::new(
                    Port::BlockNormal(id),
                    Port::BlockResult(id),
                    Route::Result,
                ));
            }
            let Some(stored) = self.endpoints.get(&key) else {
                continue;
            };
            if *stored != edges {
                return Err(invalid(span));
            }
            for edge in edges {
                self.scalar_completion_edge(reports, Family::Endpoint, edge, span)?;
            }
            let control = if let Some(parent) = parent {
                if !self
                    .flow
                    .spend(self.endpoints.len().checked_ilog2().unwrap_or(0) as usize + 3)
                {
                    return Err(budget(span));
                }
                let point = self.points.get(parent).ok_or_else(|| invalid(span))?;
                let edges = [
                    Edge::new(Port::Entry(parent), Port::BlockEntry(id), Route::Next),
                    Edge::new(Port::BlockResult(id), Port::Normal(parent), Route::Result),
                ];
                if point.owner != owner
                    || !point.complete
                    || point.kind != PointKind::Expr
                    || self
                        .endpoints
                        .get(&SequenceSource::Expr(parent))
                        .map(Vec::as_slice)
                        != Some(&edges)
                {
                    return Err(invalid(span));
                }
                for edge in edges {
                    self.scalar_completion_edge(reports, Family::Endpoint, edge, span)?;
                }
                super::super::super::super::controls::dependency(
                    parent,
                    owner,
                    false,
                    &self.points,
                    &self.branch_inputs,
                    &mut self.flow,
                )
                .map_err(|error| error.diagnostic("scalar-completion", span))?
            } else {
                if reports.entries.get(&owner).map(|entry| entry.0) != Some(id) {
                    continue;
                }
                Some(false)
            };
            let body = Body {
                owner,
                normal,
                control,
            };
            if !body.normal && graph.nodes.contains_key(&Port::BlockResult(id)) {
                return Err(invalid(span));
            }
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if observations.completion.bodies.insert(id, body).is_some() {
                return Err(invalid(span));
            }
            let port = Port::Leave(id);
            if !sources.targets.contains(&id) {
                if graph.nodes.contains_key(&port) {
                    return Err(invalid(span));
                }
                match observations.completion.absent.get(&port) {
                    Some(found) if *found != owner => return Err(invalid(span)),
                    Some(_) => (),
                    None => {
                        parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                        observations.completion.absent.insert(port, owner);
                    }
                }
            }
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
