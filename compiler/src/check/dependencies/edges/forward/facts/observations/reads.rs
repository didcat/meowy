use super::*;
use domain::Domain;

impl Checker {
    pub(in super::super) fn scalar_read_observations<'a, 'd>(
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
        self.program_signatures(program, reports, span)?;
        let mut parts = parts.min(MAX_EDGES);
        for (&point, source) in &reports.effects {
            let (
                owner,
                effect @ Effect::Read {
                    local,
                    storage,
                    normal,
                    ..
                },
            ) = source
            else {
                continue;
            };
            if self.read_effect(reports, point, *owner, span)? != *effect {
                return Err(invalid(span));
            }
            if !self.flow.spend(
                observations
                    .definitions
                    .declarations
                    .len()
                    .checked_ilog2()
                    .unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + 6,
            ) {
                return Err(budget(span));
            }
            let Some(location) = observations.definitions.declarations.get(local) else {
                continue;
            };
            if location.version.owner != *owner || location.version.local != *local {
                return Err(invalid(span));
            }
            if local != storage
                || self.proofs.aliases.contains_key(local)
                || self.proofs.fields.contains(local)
                || self.proofs.receivers.contains(local)
                || self.proofs.temporaries.contains_key(local)
            {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(entry) =
                observations.declared.entry(*local)
            {
                let ty = program.locals.get(*local).ok_or_else(|| invalid(span))?;
                let Some(declared) = Domain::declared(ty, &mut self.flow, span)? else {
                    continue;
                };
                parts = parts
                    .checked_sub(1 + declared.weight())
                    .ok_or_else(|| budget(span))?;
                entry.insert(declared);
            }
            for port in [Port::Operation(point), Port::Normal(point)] {
                if !self.flow.spend(2) {
                    return Err(budget(span));
                }
                let node = graph.nodes.get(&port);
                if port == Port::Operation(point) && node.is_none()
                    || port == Port::Normal(point) && node.is_some() && !normal
                {
                    return Err(invalid(span));
                }
                if let Some(node) = node {
                    if node.site != self.scalar_port_site(port, *owner, span)? {
                        return Err(invalid(span));
                    }
                    parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                    if observations.sources.insert(port, source).is_some() {
                        return Err(invalid(span));
                    }
                }
            }
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
