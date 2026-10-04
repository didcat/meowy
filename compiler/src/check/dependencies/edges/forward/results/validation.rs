use super::*;

impl Checker {
    pub(in super::super) fn validate_result_report(
        &mut self,
        reports: &Reports,
        id: hir::BlockId,
        owner: usize,
        result: &Observed,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof result-consumer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-consumer identity mismatch", span);
        if !self.flow.spend(
            reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 5,
        ) {
            return Err(budget());
        }
        if !self.validate_block_effect(reports, owner, Port::BlockResult(id), span)? {
            return Err(invalid());
        }
        let body = self.bodies.get(&id).ok_or_else(invalid)?;
        let (block_owner, block) = reports.blocks.get(&id).ok_or_else(invalid)?;
        if *block_owner != owner
            || !block.result
            || block.parent != body.parent
            || block.span != body.span
            || block.completion != body.completion
            || result.consumer != body.parent
            || !match (&body.layout, &result.slots) {
                (Layout::Unknown, None) => true,
                (Layout::Slots(layout), Some(slots)) => layout.len() == slots.len(),
                _ => false,
            }
        {
            return Err(invalid());
        }
        Ok(())
    }
}
