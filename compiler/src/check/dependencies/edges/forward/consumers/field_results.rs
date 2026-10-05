use super::*;

impl Checker {
    pub(super) fn field_result_slot(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        effect: &Effect,
        span: Span,
    ) -> Result<Option<Slot>> {
        let Effect::Field {
            load,
            normal,
            operation,
            result,
            ..
        } = effect
        else {
            return Ok(None);
        };
        self.validate_field_report(reports, id, owner, effect, span)?;
        if *load || !normal || !operation || !result {
            return Ok(None);
        }
        if !self
            .flow
            .spend(reports.slot_uses.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(Diagnostic::unsupported(
                "proof field-result budget exhausted",
                span,
            ));
        }
        let Some(&(source_owner, slot)) = reports.slot_uses.get(&Port::Operation(id)) else {
            return Ok(None);
        };
        if source_owner != owner || self.field_slot(reports, id, owner, effect, span)? != Some(slot)
        {
            return Err(Diagnostic::unsupported(
                "proof field-result identity mismatch",
                span,
            ));
        }
        Ok(Some(slot))
    }
}

#[cfg(test)]
mod tests;
