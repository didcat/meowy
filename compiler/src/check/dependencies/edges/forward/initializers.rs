use super::{effects::Effect, entries::Reports, *};
use crate::hir;

mod parameters;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Initializer {
    pub(crate) statement: PointId,
    pub(crate) owner: usize,
    pub(crate) input: Option<PointId>,
}

pub(crate) type Initializers = BTreeMap<hir::LocalId, Initializer>;

impl Checker {
    pub(super) fn binding_initializers(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
    ) -> Result<Initializers> {
        self.binding_initializers_limited(program, reports, span, MAX_EDGES)
    }

    pub(super) fn binding_initializers_limited(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Initializers> {
        let budget = || Diagnostic::unsupported("proof initializer-index budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof initializer-index identity mismatch", span);
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let limit = limit
            .min(MAX_EDGES)
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.slot_uses.len()))
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .ok_or_else(budget)?;
        let params = self.initializer_parameters(program, reports, span)?;
        let mut index = Initializers::new();
        for (&id, (owner, effect)) in &reports.effects {
            if !matches!(effect, Effect::Storage { .. }) {
                continue;
            }
            let Some(binding) = self.binding_effect(reports, id, *owner, span)? else {
                continue;
            };
            if !self.flow.spend(
                reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.eligible.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                    + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                    + params.len().checked_ilog2().unwrap_or(0) as usize
                    + 7,
            ) {
                return Err(budget());
            }
            if !reports.entries.contains_key(owner) {
                return Err(invalid());
            }
            let local = binding.local;
            if !reports.eligible.contains(&local)
                || local != binding.storage
                || self.proofs.aliases.contains_key(&local)
                || self.proofs.receivers.contains(&local)
                || self.proofs.temporaries.contains_key(&local)
                || params.contains(&local)
            {
                continue;
            }
            if !self
                .flow
                .spend(index.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            {
                return Err(budget());
            }
            if index.contains_key(&local) {
                return Err(invalid());
            }
            if index.len() >= limit {
                return Err(budget());
            }
            index.insert(
                local,
                Initializer {
                    statement: id,
                    owner: *owner,
                    input: binding.input,
                },
            );
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests;
