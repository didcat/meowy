use super::{effects::Effect, entries::Reports, *};
use crate::{check::dependencies::bodies::Layout, hir};

mod emissions;
#[cfg(test)]
mod field_results;
mod fields;
mod grouped;
mod index;
mod primary;

#[cfg(test)]
mod outputs;

#[cfg(test)]
mod lists;

pub(crate) type Index = BTreeMap<PointId, (usize, hir::BlockId)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .and_then(|room| room.checked_sub(reports.initializers.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut uses = Uses::new();
        for (&id, (owner, effect)) in &reports.effects {
            if matches!(effect, Effect::Emission(_)) {
                self.emission_slot_uses(reports, (id, *owner), effect, &mut uses, limit, span)?;
                continue;
            }
            if let Effect::Output(output) = effect {
                self.validate_output_report(reports, id, *owner, output, span)?;
                if !self.flow.spend(output.parts.len() * 2 + 1) {
                    return Err(budget());
                }
                for (&part, observed) in &output.parts {
                    if !observed.projection {
                        continue;
                    }
                    let input = observed.input.ok_or_else(|| {
                        Diagnostic::unsupported("proof output-effect identity mismatch", span)
                    })?;
                    if let Some(slot) = self.primary_slot(reports, input.point, *owner, span)? {
                        let port = Port::Projection {
                            point: id,
                            step: part,
                        };
                        self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
                    }
                }
                continue;
            }
            if let Effect::List(list) = effect {
                self.validate_list_report(reports, id, *owner, list, span)?;
                if !self.flow.spend(list.inputs.len() * 2 + 1) {
                    return Err(budget());
                }
                for (step, input) in list.inputs.iter().enumerate() {
                    if !input.projected {
                        continue;
                    }
                    if let Some(slot) = self.primary_slot(reports, input.point, *owner, span)? {
                        let port = Port::Projection { point: id, step };
                        self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
                    }
                }
                continue;
            }
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
                self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
            }
        }
        Ok(uses)
    }

    pub(super) fn record_slot_use(
        &mut self,
        uses: &mut Uses,
        port: Port,
        value: (usize, Slot),
        limit: usize,
        span: Span,
    ) -> Result<()> {
        if uses.len() >= limit
            || !self
                .flow
                .spend(uses.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(Diagnostic::unsupported(
                "proof slot-use budget exhausted",
                span,
            ));
        }
        uses.insert(port, value);
        Ok(())
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
