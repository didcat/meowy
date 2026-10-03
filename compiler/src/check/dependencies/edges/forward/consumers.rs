use super::{effects::Effect, entries::Reports, *};
use crate::{check::dependencies::bodies::Layout, hir};

mod fields;
mod grouped;
mod index;
mod primary;

pub(crate) type Index = BTreeMap<PointId, (usize, hir::BlockId)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Slot {
    pub(crate) block: hir::BlockId,
    pub(crate) index: usize,
}

pub(crate) type Uses = BTreeMap<Port, (usize, Slot)>;

impl Checker {
    pub(super) fn slot_uses(&mut self, reports: &Reports, span: Span) -> Result<Uses> {
        self.slot_uses_limited(reports, span, MAX_EDGES)
    }

    pub(self) fn slot_uses_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Uses> {
        let budget = || Diagnostic::unsupported("proof slot-use budget exhausted", span);
        let limit = limit
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut uses = Uses::new();
        for (&id, (owner, effect)) in &reports.effects {
            let mut slots = [None; 3];
            slots[0] = self
                .field_slot(reports, id, *owner, effect, span)?
                .map(|slot| (Port::Operation(id), slot));
            for (step, input) in self
                .primary_effect_inputs(reports, id, *owner, effect, span)?
                .into_iter()
                .enumerate()
            {
                if let Some(input) = input {
                    slots[step + 1] = self
                        .primary_slot(reports, input, *owner, span)?
                        .map(|slot| (Port::Projection { point: id, step }, slot));
                }
            }
            for (port, slot) in slots.into_iter().flatten() {
                if uses.len() >= limit
                    || !self
                        .flow
                        .spend(uses.len().checked_ilog2().unwrap_or(0) as usize + 2)
                {
                    return Err(budget());
                }
                uses.insert(port, (*owner, slot));
            }
        }
        Ok(uses)
    }

    pub(self) fn slot_block(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let Some(input) = self.grouped_consumer(reports, input, owner, span)? else {
            return Ok(None);
        };
        let budget = || Diagnostic::unsupported("proof slot-use budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof slot-use identity mismatch", span);
        if !self.flow.spend(
            reports.consumers.len().checked_ilog2().unwrap_or(0) as usize
                + reports.results.len().checked_ilog2().unwrap_or(0) as usize
                + reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 6,
        ) {
            return Err(budget());
        }
        let Some(&(source_owner, block)) = reports.consumers.get(&input) else {
            return Ok(None);
        };
        if source_owner != owner
            || !self.validate_block_effect(reports, owner, Port::BlockResult(block), span)?
        {
            return Err(invalid());
        }
        let body = &self.bodies[&block];
        let (result_owner, result) = reports.results.get(&block).ok_or_else(invalid)?;
        let (block_owner, observed) = reports.blocks.get(&block).ok_or_else(invalid)?;
        if *result_owner != owner
            || *block_owner != owner
            || !observed.result
            || observed.parent != body.parent
            || observed.span != body.span
            || observed.completion != body.completion
            || body.parent != Some(input)
            || result.consumer != Some(input)
            || !match (&body.layout, &result.slots) {
                (Layout::Unknown, None) => true,
                (Layout::Slots(layout), Some(slots)) => layout.len() == slots.len(),
                _ => false,
            }
        {
            return Err(invalid());
        }
        Ok(Some(block))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
