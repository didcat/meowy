use super::*;
use crate::check::dependencies::{bodies::completion::Shape, emissions::Projection};

impl Checker {
    pub(super) fn emission_slot_uses(
        &mut self,
        reports: &Reports,
        site: (PointId, usize),
        effect: &Effect,
        uses: &mut Uses,
        limit: usize,
        span: Span,
    ) -> Result<()> {
        let Effect::Emission(observed) = effect else {
            return Ok(());
        };
        let (id, owner) = site;
        self.validate_emission_report(reports, id, owner, observed, span)?;
        let Some(composed) = &observed.composed else {
            return Ok(());
        };
        let budget = || Diagnostic::unsupported("proof emission-slot budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof emission-slot identity mismatch", span);
        if !self.flow.spend(observed.initialized.len() * 2 + 1) {
            return Err(budget());
        }
        if !observed.initialized.iter().any(|initialized| *initialized) {
            return Ok(());
        }
        let Some(block) = self.slot_block(reports, observed.input, owner, span)? else {
            return Ok(());
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(budget());
        }
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(());
        };
        if !self.flow.spend(slots.len() + 1) {
            return Err(budget());
        }
        let (count, bytes) = body.layout.counts().ok_or_else(budget)?;
        if !self.flow.spend(bytes * 2 + count + 1) {
            return Err(budget());
        }
        if body.completion.result
            != (Shape::Record {
                fields: composed.count,
            })
            || count != observed.targets.len()
            || slots
                .iter()
                .zip(&observed.targets)
                .any(|(slot, target)| slot.field != target.field)
        {
            return Err(invalid());
        }
        for (target, initialized) in observed.targets.iter().zip(&observed.initialized) {
            if !initialized {
                continue;
            }
            let index = match target.projection {
                Projection::Primary => 0,
                Projection::Field(index) => index + 1,
                Projection::Value => return Err(invalid()),
            };
            self.record_slot_use(
                uses,
                Port::Emission(target.id),
                (owner, Slot { block, index }),
                limit,
                span,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
