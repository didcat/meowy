use super::*;
use crate::check::dependencies::bodies::completion::Shape;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) consumer: PointId,
    pub(crate) slot: Slot,
}

impl Checker {
    pub(in super::super) fn scalar_block_source(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Source>> {
        let Some(consumer) = self.grouped_consumer(reports, input, owner, span)? else {
            return Ok(None);
        };
        let Some(block) = self.qualified_slot_block(reports, consumer, owner, span)? else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 3)
        {
            return Err(Diagnostic::unsupported(
                "proof scalar-block budget exhausted",
                span,
            ));
        }
        let body = &self.bodies[&block];
        if !matches!(body.completion.result, Shape::Scalar(_)) {
            return Ok(None);
        }
        let Layout::Slots(slots) = &body.layout else {
            return Err(Diagnostic::unsupported(
                "proof scalar-block identity mismatch",
                span,
            ));
        };
        if slots.len() != 1
            || slots[0].field.is_some()
            || slots[0].mutable
            || slots[0].shape != body.completion.result
        {
            return Err(Diagnostic::unsupported(
                "proof scalar-block identity mismatch",
                span,
            ));
        }
        Ok(Some(Source {
            consumer,
            slot: Slot { block, index: 0 },
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
