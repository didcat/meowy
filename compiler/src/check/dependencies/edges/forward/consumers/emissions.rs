use super::*;
use crate::check::dependencies::{
    bodies::completion::Shape,
    emissions::{Projection, Target},
};

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
        let Some(block) = self.emission_source_block(reports, owner, effect, span)? else {
            return Ok(());
        };
        let invalid = || Diagnostic::unsupported("proof emission-slot identity mismatch", span);
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

    pub(in super::super) fn emission_source_block(
        &mut self,
        reports: &Reports,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let Effect::Emission(observed) = effect else {
            return Ok(None);
        };
        let Some(composed) = &observed.composed else {
            return Ok(None);
        };
        let budget = || Diagnostic::unsupported("proof emission-slot budget exhausted", span);
        if !self.flow.spend(observed.initialized.len() * 2 + 1) {
            return Err(budget());
        }
        if !observed.initialized.iter().any(|initialized| *initialized) {
            return Ok(None);
        }
        let Some(block) = self.slot_block(reports, observed.input, owner, span)? else {
            return Ok(None);
        };
        self.emission_source_layout(block, composed.count, &observed.targets, span)
    }

    pub(self) fn emission_source_layout(
        &mut self,
        block: hir::BlockId,
        fields: usize,
        targets: &[Target],
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let budget = || Diagnostic::unsupported("proof emission-slot budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof emission-slot identity mismatch", span);
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(budget());
        }
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(None);
        };
        if !self.flow.spend(slots.len() + 1) {
            return Err(budget());
        }
        let (count, bytes) = body.layout.counts().ok_or_else(budget)?;
        if !self.flow.spend(bytes * 2 + count + 1) {
            return Err(budget());
        }
        if body.completion.result != (Shape::Record { fields })
            || count != targets.len()
            || slots
                .iter()
                .zip(targets)
                .any(|(slot, target)| slot.field != target.field)
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
