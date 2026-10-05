use super::*;

impl Checker {
    pub(super) fn slot_block(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let Some(input) = self.grouped_consumer(reports, input, owner, span)? else {
            return Ok(None);
        };
        self.qualified_slot_block(reports, input, owner, span)
    }

    pub(super) fn qualified_slot_block(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
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
