use super::*;
use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};

impl Checker {
    pub(super) fn receiver_primary_slot(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        ty: ScalarKind,
        span: Span,
    ) -> Result<Option<Slot>> {
        let invalid = || Diagnostic::unsupported("proof receiver-primary identity mismatch", span);
        let Some(block) = self.record_receiver_body(reports, input, owner, span)? else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 6)
        {
            return Err(Diagnostic::unsupported(
                "proof receiver-primary budget exhausted",
                span,
            ));
        }
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(None);
        };
        let primary = slots.first().ok_or_else(invalid)?;
        if !matches!(body.completion.result, Shape::Record { .. })
            || primary.field.is_some()
            || primary.mutable
        {
            return Err(invalid());
        }
        let Shape::Scalar(actual) = primary.shape else {
            return Ok(None);
        };
        if actual != ty {
            return Err(invalid());
        }
        Ok(Some(Slot { block, index: 0 }))
    }

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
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(Diagnostic::unsupported(
                "proof primary-slot budget exhausted",
                span,
            ));
        }
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

#[cfg(test)]
mod coercions;

#[cfg(test)]
mod coercions_limits;

#[cfg(test)]
mod dispatch_unary;

#[cfg(test)]
mod dispatch_binary;

#[cfg(test)]
mod dispatch_coercions;

#[cfg(test)]
mod receiver_coercions;
