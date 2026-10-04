use super::*;

impl Checker {
    pub(super) fn candidate_source_slot(
        &mut self,
        ctx: &mut Context<'_>,
        owner: usize,
        input: &Input,
        span: Span,
    ) -> Result<Option<Slot>> {
        let budget = || Diagnostic::unsupported("proof candidate-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof candidate-source identity mismatch", span);
        let reports = ctx.reports;
        if !self.flow.spend(
            reports.slot_uses.len().checked_ilog2().unwrap_or(0) as usize
                + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                + ctx.sources.len().checked_ilog2().unwrap_or(0) as usize * 2
                + ctx.emissions.len().checked_ilog2().unwrap_or(0) as usize
                + 9,
        ) {
            return Err(budget());
        }
        let Some(&(source_owner, slot)) = reports
            .slot_uses
            .get(&Port::Emission(input.candidate.emission))
        else {
            return Ok(None);
        };
        let index = match input.projection {
            Projection::Value => return Err(invalid()),
            Projection::Primary => 0,
            Projection::Field(index) => index.checked_add(1).ok_or_else(invalid)?,
        };
        let statement = input.candidate.statement;
        let (reported_owner, effect @ Effect::Emission(observed)) =
            reports.effects.get(&statement).ok_or_else(invalid)?
        else {
            return Err(invalid());
        };
        let target = observed
            .targets
            .get(input.candidate.target)
            .ok_or_else(invalid)?;
        if source_owner != owner
            || *reported_owner != owner
            || !ctx.emissions.contains(&statement)
            || input.point != observed.input
            || observed.composed.is_none()
            || target.id != input.candidate.emission
            || target.projection != input.projection
            || observed.initialized.get(input.candidate.target) != Some(&true)
            || slot.index != index
        {
            return Err(invalid());
        }
        let block = if let Some(&block) = ctx.sources.get(&statement) {
            block
        } else {
            if ctx.parts == 0 {
                return Err(budget());
            }
            let block = self
                .emission_source_block(reports, owner, effect, span)?
                .ok_or_else(invalid)?;
            ctx.parts -= 1;
            ctx.sources.insert(statement, block);
            block
        };
        if slot.block != block {
            return Err(invalid());
        }
        Ok(Some(slot))
    }
}

#[cfg(test)]
mod tests;
