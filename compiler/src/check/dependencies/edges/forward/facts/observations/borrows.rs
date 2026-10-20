use super::*;
use crate::check::dependencies::ScalarKind;
use crate::hir::ReferenceMode;

impl Checker {
    pub(in super::super) fn scalar_borrow_observations<'a, 'd>(
        &mut self,
        program: &hir::Program,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        for (&point, source) in &reports.effects {
            let (owner, Effect::Borrow(observed)) = source else {
                continue;
            };
            if !self.validate_place_borrow(reports, *owner, Port::Normal(point), span)? {
                return Err(invalid(span));
            }
            if !self.flow.spend(
                self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                    + 9,
            ) {
                return Err(budget(span));
            }
            let op = &self.place_borrows[&point];
            if observed.place != op.place
                || observed.storage != op.storage
                || observed.mode != op.mode
                || observed.control != op.control
                || observed.addresses.len() != op.place.fields.len() + 1
            {
                return Err(invalid(span));
            }
            let local = op.place.root;
            if !op.place.fields.is_empty()
                || local != op.storage
                || self.proofs.aliases.contains_key(&local)
                || self.proofs.fields.contains(&local)
                || self.proofs.receivers.contains(&local)
                || self.proofs.temporaries.contains_key(&local)
            {
                continue;
            }
            let Some(location) = observations.definitions.declarations.get(&local) else {
                continue;
            };
            let ty = program.locals.get(local).ok_or_else(|| invalid(span))?;
            if location.version.owner != *owner
                || location.version.local != local
                || op.scalar != ScalarKind::of(ty)
                || op.mode == ReferenceMode::Exclusive && !self.proofs.mutable.contains(&local)
            {
                return Err(invalid(span));
            }
            if domain::Domain::declared(ty, &mut self.flow, span)?.is_none() {
                continue;
            }
            let edges = [op.edges[0], op.edges[1], op.edges[2]];
            for (edge, reached) in
                edges
                    .into_iter()
                    .zip([observed.addresses[0], observed.acquired, observed.result])
            {
                if !self.flow.spend(7) {
                    return Err(budget(span));
                }
                let node = graph.nodes.get(&edge.to);
                if reached != node.is_some() {
                    return Err(invalid(span));
                }
                let Some(node) = node else {
                    continue;
                };
                let [input] = node.incoming.as_slice() else {
                    return Err(invalid(span));
                };
                let from = graph.nodes.get(&edge.from);
                let site = from
                    .map(|node| node.site)
                    .or_else(|| graph.external.get(&edge.from).copied());
                if node.site != self.scalar_port_site(edge.to, *owner, span)?
                    || Some(input.source) != site
                    || input.source != node.site
                    || input.reached != from.is_some()
                    || graph.edges.get(input.edge) != Some(&(Family::PlaceBorrow, edge))
                {
                    return Err(invalid(span));
                }
                parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                if observations.sources.insert(edge.to, source).is_some() {
                    return Err(invalid(span));
                }
            }
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
