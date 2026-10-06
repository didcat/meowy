use super::*;
use crate::check::dependencies::bodies::completion::Shape;

impl Checker {
    pub(super) fn field_slot(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<Option<Slot>> {
        let Effect::Field {
            input,
            index,
            load,
            normal,
            operation,
            ..
        } = effect
        else {
            return Ok(None);
        };
        let invalid = || Diagnostic::unsupported("proof field-slot identity mismatch", span);
        self.validate_field_report(reports, id, owner, effect, span)?;
        if *load || !operation {
            return Ok(None);
        }
        let (block, scalar, receiver) =
            if let Some(block) = self.slot_block(reports, *input, owner, span)? {
                (block, false, false)
            } else if let Some(block) = self.record_dispatch_body(reports, *input, owner, span)? {
                (block, true, false)
            } else if let Some(block) = self.record_receiver_body(reports, *input, owner, span)? {
                (block, true, true)
            } else {
                return Ok(None);
            };
        if !self.flow.spend(
            self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + self.fields.len().checked_ilog2().unwrap_or(0) as usize
                + 3,
        ) {
            return Err(Diagnostic::unsupported(
                "proof field-slot budget exhausted",
                span,
            ));
        }
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(None);
        };
        let Shape::Record { fields } = body.completion.result else {
            return Err(invalid());
        };
        let slot = index.checked_add(1).ok_or_else(invalid)?;
        let selected = slots.get(slot).ok_or_else(invalid)?;
        if self.fields[&id].count != fields
            || selected.field.is_none()
            || *normal != (selected.shape != Shape::Never)
            || (receiver && selected.mutable)
        {
            return Err(invalid());
        }
        if scalar && !matches!(selected.shape, Shape::Scalar(_)) {
            return Ok(None);
        }
        Ok(Some(Slot { block, index: slot }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod dispatch;

#[cfg(test)]
mod receivers;
