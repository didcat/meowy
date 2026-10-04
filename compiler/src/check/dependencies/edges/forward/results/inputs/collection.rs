use super::*;

impl Checker {
    pub(in super::super::super) fn candidate_inputs(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Inputs, usize)> {
        self.candidate_inputs_limited(reports, span, MAX_EDGES, reports.parts)
    }

    pub(super) fn candidate_inputs_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<(Inputs, usize)> {
        let budget = || Diagnostic::unsupported("proof candidate-input budget exhausted", span);
        let limit = limit
            .min(MAX_EDGES)
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .and_then(|room| room.checked_sub(reports.initializers.len()))
            .and_then(|room| room.checked_sub(reports.slot_uses.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.results.len() + 1) {
            return Err(budget());
        }
        let mut inputs = Inputs::new();
        let mut ctx = Context::new(reports, parts);
        for (&block, (_, result)) in &reports.results {
            self.candidate_result(&mut ctx, block, span)?;
            let Some(slots) = &result.slots else {
                continue;
            };
            if !self.flow.spend(slots.len() + 1) {
                return Err(budget());
            }
            for (slot, source) in slots.iter().enumerate() {
                let Sources::Candidates(values) = source else {
                    continue;
                };
                if !self.flow.spend(values.len() + 1) {
                    return Err(budget());
                }
                for position in 0..values.len() {
                    if inputs.len() >= limit
                        || !self
                            .flow
                            .spend(inputs.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
                    {
                        return Err(budget());
                    }
                    let key = (block, slot, position);
                    let input = self.candidate_input(&mut ctx, key, span)?;
                    inputs.insert(key, input);
                }
            }
        }
        Ok((inputs, ctx.parts))
    }
}

#[cfg(test)]
mod tests;
