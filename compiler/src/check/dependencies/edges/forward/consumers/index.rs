use super::*;
use crate::check::dependencies::bodies::Layout;

impl Checker {
    pub(in super::super) fn result_consumers(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<Index> {
        self.result_consumers_limited(reports, span, MAX_EDGES)
    }

    pub(super) fn result_consumers_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Index> {
        let budget = || Diagnostic::unsupported("proof result-consumer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-consumer identity mismatch", span);
        let limit = limit
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.results.len() + 1) {
            return Err(budget());
        }
        let mut index = Index::new();
        for (&id, (owner, result)) in &reports.results {
            if !self.flow.spend(
                reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 5,
            ) {
                return Err(budget());
            }
            if !self.validate_block_effect(reports, *owner, Port::BlockResult(id), span)? {
                return Err(invalid());
            }
            let body = self.bodies.get(&id).ok_or_else(invalid)?;
            let (block_owner, block) = reports.blocks.get(&id).ok_or_else(invalid)?;
            if *block_owner != *owner
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
            let Some(consumer) = result.consumer else {
                continue;
            };
            if index.len() >= limit
                || !self
                    .flow
                    .spend(index.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            {
                return Err(budget());
            }
            if index.insert(consumer, (*owner, id)).is_some() {
                return Err(invalid());
            }
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests;
