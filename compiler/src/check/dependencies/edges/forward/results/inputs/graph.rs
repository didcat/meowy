use super::*;

mod walk;
pub(crate) use walk::Walk;

pub(crate) struct Graph<'a> {
    pub(super) results: &'a Results,
    pub(super) inputs: &'a Inputs,
}

impl Checker {
    pub(in super::super::super) fn candidate_walk_report(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<(Walk, usize)> {
        let (graph, parts) = self.candidate_graph(reports, span)?;
        graph.forest(&mut self.flow, span, parts)
    }

    pub(in super::super::super) fn candidate_graph<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
    ) -> Result<(Graph<'a>, usize)> {
        self.candidate_graph_limited(reports, span, reports.parts)
    }

    pub(super) fn candidate_graph_limited<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Graph<'a>, usize)> {
        let budget = || Diagnostic::unsupported("proof candidate-graph budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-graph identity mismatch", span);
        if reports.candidate_inputs.len() > MAX_EDGES || !self.flow.spend(reports.results.len() + 1)
        {
            return Err(budget());
        }
        let mut ctx = Context::new(reports, parts);
        let mut count = 0usize;
        for (&block, (owner, result)) in &reports.results {
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
                if !self
                    .flow
                    .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 3)
                {
                    return Err(budget());
                }
                let Layout::Slots(layout) = &self.bodies[&block].layout else {
                    return Err(invalid());
                };
                if layout[slot].mutable || !matches!(layout[slot].shape, Shape::Scalar(_)) {
                    return Err(invalid());
                }
                count = count
                    .checked_add(values.len())
                    .filter(|count| *count <= MAX_EDGES)
                    .ok_or_else(budget)?;
                if !self.flow.spend(values.len() + 1) {
                    return Err(budget());
                }
                for position in 0..values.len() {
                    if !self.flow.spend(
                        reports.candidate_inputs.len().checked_ilog2().unwrap_or(0) as usize + 2,
                    ) {
                        return Err(budget());
                    }
                    let key = (block, slot, position);
                    let observed = reports.candidate_inputs.get(&key).ok_or_else(invalid)?;
                    if observed.0 != *owner
                        || *observed != self.candidate_input(&mut ctx, key, span)?
                    {
                        return Err(invalid());
                    }
                }
            }
        }
        if count != reports.candidate_inputs.len() {
            return Err(invalid());
        }
        Ok((
            Graph {
                results: &reports.results,
                inputs: &reports.candidate_inputs,
            },
            ctx.parts,
        ))
    }
}

#[cfg(test)]
mod tests;
