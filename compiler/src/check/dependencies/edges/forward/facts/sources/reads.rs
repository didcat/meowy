use super::*;
use versions::Definition;

impl Checker {
    pub(in super::super) fn scalar_read_version(
        &mut self,
        reports: &Reports,
        versions: &Versions,
        point: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Version>> {
        if !self.flow.spend(
            reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                + versions.len().checked_ilog2().unwrap_or(0) as usize
                + reports.eligible.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.mutable.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.fields.len().checked_ilog2().unwrap_or(0) as usize
                + 7,
        ) {
            return Err(budget(span));
        }
        let Some((
            reported,
            effect @ Effect::Read {
                local,
                storage,
                normal,
                ..
            },
        )) = reports.effects.get(&point)
        else {
            return Ok(None);
        };
        if *reported != owner || self.read_effect(reports, point, owner, span)? != *effect {
            return Err(invalid(span));
        }
        if !normal
            || local != storage
            || !reports.eligible.contains(local)
            || self.proofs.variable(*local)
        {
            return Ok(None);
        }
        let Some(slot) = versions.get(local) else {
            return Ok(None);
        };
        if slot.version.owner != owner || slot.version.local != *local {
            return Err(invalid(span));
        }
        if matches!(slot.version.definition, Definition::Write(_)) {
            return Ok(None);
        }
        if let Definition::Bind(statement) = slot.version.definition {
            let input = self.read_initializer_input(reports, point, owner, span)?;
            if !self
                .flow
                .spend(reports.initializers.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget(span));
            }
            if !reports
                .initializers
                .get(local)
                .is_some_and(|init| init.statement == statement && init.input == input)
            {
                return Err(invalid(span));
            }
        }
        Ok(slot.declared.as_ref().map(|_| slot.version))
    }
}
