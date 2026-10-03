use super::*;

impl Checker {
    pub(in super::super::super) fn read_initializer_input(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        let budget = || Diagnostic::unsupported("proof read-initializer budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof read-initializer identity mismatch", span);
        if !self
            .flow
            .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some((
            reported,
            effect @ Effect::Read {
                local,
                storage,
                normal,
                ..
            },
        )) = reports.effects.get(&id)
        else {
            return Ok(None);
        };
        if *reported != owner || self.read_effect(reports, id, owner, span)? != *effect {
            return Err(invalid());
        }
        if !normal || local != storage {
            return Ok(None);
        }
        if !self.flow.spend(
            reports.eligible.len().checked_ilog2().unwrap_or(0) as usize
                + reports.initializers.len().checked_ilog2().unwrap_or(0) as usize
                + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.receivers.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.temporaries.len().checked_ilog2().unwrap_or(0) as usize
                + 9,
        ) {
            return Err(budget());
        }
        if !reports.eligible.contains(local)
            || self.proofs.aliases.contains_key(local)
            || self.proofs.receivers.contains(local)
            || self.proofs.temporaries.contains_key(local)
        {
            return Ok(None);
        }
        let Some(init) = reports.initializers.get(local) else {
            return Ok(None);
        };
        if init.owner != owner || !reports.entries.contains_key(&owner) {
            return Err(invalid());
        }
        let binding = self
            .binding_effect(reports, init.statement, owner, span)?
            .ok_or_else(invalid)?;
        if binding.local != *local || binding.storage != *storage || binding.input != init.input {
            return Err(invalid());
        }
        Ok(init.input)
    }
}

#[cfg(test)]
mod tests;
