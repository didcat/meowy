use super::*;
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

impl Checker {
    pub(in super::super::super) fn field_results(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Uses, usize)> {
        self.field_results_limited(reports, span, MAX_EDGES, reports.parts)
    }

    pub(super) fn field_results_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<(Uses, usize)> {
        let budget = || Diagnostic::unsupported("proof field-result budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-result identity mismatch", span);
        let limit = limit
            .min(MAX_EDGES)
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .and_then(|room| room.checked_sub(reports.initializers.len()))
            .and_then(|room| room.checked_sub(reports.slot_uses.len()))
            .and_then(|room| room.checked_sub(reports.candidate_inputs.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut uses = Uses::new();
        let mut roots = None;
        let mut parts = parts.min(MAX_EDGES);
        for (&id, (owner, effect)) in &reports.effects {
            let Some(slot) = self.field_result_slot(reports, id, *owner, effect, span)? else {
                continue;
            };
            if roots.is_none() {
                let (index, remaining) = self.field_result_roots(reports, span, parts)?;
                roots = Some(index);
                parts = remaining;
            }
            let roots = roots.as_ref().unwrap();
            if !self
                .flow
                .spend(roots.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            if roots.get(&slot) != Some(owner) {
                return Err(invalid());
            }
            self.record_slot_use(&mut uses, Port::Normal(id), (*owner, slot), limit, span)?;
        }
        Ok((uses, parts))
    }

    pub(super) fn field_result_roots(
        &mut self,
        reports: &Reports,
        span: Span,
        mut parts: usize,
    ) -> Result<(BTreeMap<Slot, usize>, usize)> {
        let budget = || Diagnostic::unsupported("proof field-result budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof field-result identity mismatch", span);
        if reports.candidate_walk.visits.len() > MAX_EDGES
            || !self.flow.spend(reports.candidate_walk.visits.len() + 1)
        {
            return Err(budget());
        }
        let mut roots = BTreeMap::new();
        for visit in &reports.candidate_walk.visits {
            let Visit::Root(root) = visit else {
                continue;
            };
            if !self.flow.spend(
                reports.results.len().checked_ilog2().unwrap_or(0) as usize
                    + roots.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 4,
            ) {
                return Err(budget());
            }
            let (owner, result) = reports.results.get(&root.slot.block).ok_or_else(invalid)?;
            if *owner != root.owner
                || result
                    .slots
                    .as_ref()
                    .and_then(|slots| slots.get(root.slot.index))
                    .is_none()
                || roots.contains_key(&root.slot)
            {
                return Err(invalid());
            }
            parts = parts.checked_sub(1).ok_or_else(budget)?;
            roots.insert(root.slot, root.owner);
        }
        Ok((roots, parts))
    }
}

#[cfg(test)]
mod tests;
