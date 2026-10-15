use super::*;
use crate::check::dependencies::{bodies::Completion, operations::SharedRecord};

impl Checker {
    pub(super) fn shared_record_body(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        record: SharedRecord,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let invalid =
            || Diagnostic::unsupported("proof shared-local field identity mismatch", span);
        let block = if let Some(block) = self.qualified_slot_block(reports, input, owner, span)? {
            block
        } else if let Some(block) = self.record_dispatch_body(reports, input, owner, span)? {
            block
        } else {
            return Ok(None);
        };
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 5)
        {
            return Err(Diagnostic::unsupported(
                "proof shared-local field budget exhausted",
                span,
            ));
        }
        let body = &self.bodies[&block];
        let Layout::Slots(slots) = &body.layout else {
            return Ok(None);
        };
        if body.completion.result
            != (Shape::Record {
                fields: record.fields,
            })
            || slots.len() != record.fields + 1
        {
            return Err(invalid());
        }
        if !self.flow.spend(slots.len()) {
            return Err(Diagnostic::unsupported(
                "proof shared-local field budget exhausted",
                span,
            ));
        }
        for (index, slot) in slots.iter().enumerate() {
            if slot.mutable
                || slot.field.is_none() != (index == 0)
                || (index == 0 && slot.shape != Shape::SharedScalar(record.primary))
                || (index > 0 && matches!(slot.shape, Shape::SharedScalar(_) | Shape::Reference(_)))
                || !(Completion {
                    normal: slot.shape != Shape::Never,
                    result: slot.shape,
                })
                .valid()
            {
                return Err(invalid());
            }
        }
        Ok(Some(block))
    }
}
