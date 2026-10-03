use super::*;

impl Checker {
    pub(in super::super::super) fn validate_emission_report(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        observed: &Observed,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof result-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-source identity mismatch", span);
        let len = observed.targets.len();
        if len > MAX_TARGETS || !self.flow.spend(len + 1) {
            return Err(budget());
        }
        let bytes = observed.targets.iter().try_fold(0usize, |bytes, target| {
            bytes
                .checked_add(target.field.as_ref().map_or(0, String::len))
                .ok_or_else(budget)
        })?;
        if bytes > MAX_EDGES || !self.flow.spend(bytes + len * 3 + 8) {
            return Err(budget());
        }
        if self.emission_effect_stage(reports, owner, Port::Normal(id), span)? != Some((id, None)) {
            return Err(invalid());
        }
        let op = self.emissions.get(&id).ok_or_else(invalid)?;
        if observed.input != op.input
            || observed.composed != op.composed
            || observed.targets != op.targets
            || observed.control != op.control
            || observed.initialized.len() != len
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
