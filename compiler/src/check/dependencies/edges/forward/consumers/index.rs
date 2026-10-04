use super::*;

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
            self.validate_result_report(reports, id, *owner, result, span)?;
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
