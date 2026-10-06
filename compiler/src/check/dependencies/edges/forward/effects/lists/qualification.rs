use super::*;

impl Checker {
    pub(in super::super::super) fn validate_list_report(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        observed: &Observed,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof list-effect identity mismatch", span);
        let budget = || Diagnostic::unsupported("proof list-effect budget exhausted", span);
        if observed.inputs.len() > crate::list::MAX_CAPACITY
            || !self.flow.spend(
                observed.inputs.len() * 6
                    + self.lists.len().checked_ilog2().unwrap_or(0) as usize
                    + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                    + self.list_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + 6,
            )
        {
            return Err(budget());
        }
        let checked = self
            .validate_list_producer(reports, owner, id, span)?
            .ok_or_else(invalid)?;
        let op = self.lists.get(&id).ok_or_else(invalid)?;
        let sequence = self
            .sequences
            .get(&SequenceSource::Expr(id))
            .ok_or_else(invalid)?;
        let inputs = self.list_inputs.get(&id);
        if observed.capacity != op.capacity
            || observed.contextual != op.contextual
            || observed.normal != op.normal
            || observed.control != op.control
            || observed.inputs.len() != op.count
            || observed.inputs.iter().enumerate().any(|(part, input)| {
                Some(input.point) != sequence.items[part]
                    || input.plan != inputs.map(|inputs| (inputs[part].primary, inputs[part].kind))
                    || input.source != inputs.and_then(|inputs| inputs[part].source)
            })
        {
            return Err(invalid());
        }
        let mut any = observed.constructed || observed.result;
        for (part, input) in observed.inputs.iter().enumerate() {
            if input.projected {
                self.select_list_stage(
                    &checked,
                    Port::Projection {
                        point: id,
                        step: part,
                    },
                    span,
                )?;
                any = true;
            }
            if input.converted {
                self.select_list_stage(&checked, Port::Conversion { point: id, part }, span)?;
                any = true;
            }
        }
        for (port, seen) in [
            (Port::Operation(id), observed.constructed),
            (Port::Normal(id), observed.result),
        ] {
            if seen {
                self.select_list_stage(&checked, port, span)?;
            }
        }
        if !any {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
