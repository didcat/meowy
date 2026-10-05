use super::*;
use crate::check::dependencies::ScalarKind;

impl Checker {
    pub(in super::super) fn dispatch_primary_slot(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        ty: ScalarKind,
        span: Span,
    ) -> Result<Option<Slot>> {
        let invalid = || Diagnostic::unsupported("proof dispatch-primary identity mismatch", span);
        let Some(block) = self.record_dispatch_body(reports, input, owner, span)? else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 5)
        {
            return Err(Diagnostic::unsupported(
                "proof dispatch-primary budget exhausted",
                span,
            ));
        }
        let Layout::Slots(slots) = &self.bodies[&block].layout else {
            return Err(invalid());
        };
        let primary = slots.first().ok_or_else(invalid)?;
        if primary.field.is_some() || primary.mutable {
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
}

#[cfg(test)]
mod tests;
