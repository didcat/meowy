use super::*;

impl Checker {
    #[cfg(test)]
    pub(in super::super) fn validate_writes(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof write budget exhausted",
                span,
            ));
        }
        for (&point, (owner, effect)) in &reports.effects {
            if matches!(
                effect,
                Effect::Storage {
                    kind: OperationKind::Write,
                    ..
                }
            ) {
                self.write_effect(reports, point, *owner, span)?;
            }
        }
        Ok(())
    }

    pub(in super::super) fn write_effect(
        &mut self,
        reports: &Reports,
        point: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Binding>> {
        let budget = || Diagnostic::unsupported("proof write budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof write identity mismatch", span);
        let Some(binding) =
            self.storage_effect(reports, point, owner, OperationKind::Write, span)?
        else {
            return Ok(None);
        };
        if !self.flow.spend(
            self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize * 2
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 6,
        ) {
            return Err(budget());
        }
        if !self.proofs.mutable.contains(&binding.local) {
            return Err(invalid());
        }
        if let Some(alias) = self.proofs.aliases.get(&binding.local) {
            let root = self
                .proofs
                .aliases
                .get(&binding.storage)
                .ok_or_else(invalid)?;
            if !self.flow.spend(
                alias
                    .field
                    .len()
                    .saturating_add(root.field.len())
                    .saturating_add(7),
            ) {
                return Err(budget());
            }
            if !alias.mutable
                || root.root != binding.storage
                || alias.target != root.target
                || alias.field != root.field
                || !self
                    .bodies
                    .get(&alias.target)
                    .is_some_and(|body| body.owner == owner)
            {
                return Err(invalid());
            }
        }
        Ok(Some(binding))
    }
}

#[cfg(test)]
mod tests;
