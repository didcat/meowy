use super::*;
use crate::check::dependencies::bodies::completion::Shape;

impl Checker {
    pub(super) fn primary_slot(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Slot>> {
        let invalid = || Diagnostic::unsupported("proof primary-slot identity mismatch", span);
        let Some(block) = self.slot_block(reports, input, owner, span)? else {
            return Ok(None);
        };
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(None);
        };
        if !matches!(body.completion.result, Shape::Record { .. })
            || slots.first().is_none_or(|slot| slot.field.is_some())
        {
            return Err(invalid());
        }
        Ok(Some(Slot { block, index: 0 }))
    }
}

#[cfg(test)]
mod tests;
