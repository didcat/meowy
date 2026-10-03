use super::*;

impl Checker {
    pub(in super::super::super) fn validate_output_report(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        output: &Observed,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof output-effect identity mismatch", span);
        let budget = || Diagnostic::unsupported("proof output-effect budget exhausted", span);
        if output.parts.len() > crate::check::dependencies::sequences::MAX_ITEMS
            || !self.flow.spend(
                self.outputs.len().checked_ilog2().unwrap_or(0) as usize
                    + output.parts.len() * 4
                    + 7,
            )
        {
            return Err(budget());
        }
        let op = self.outputs.get(&id).ok_or_else(invalid)?;
        if op.owner != owner
            || output.panic != op.panic
            || output.control != op.control
            || output.total != op.parts.len()
            || output.stopped != op.stopped
            || output.parts.len() > output.total
            || (!output.prefix && !output.terminal && output.parts.is_empty())
        {
            return Err(invalid());
        }
        self.validate_output_edges(id, owner, span)?;
        if output.prefix {
            self.output_effect_stage(owner, Port::Prefix(id), span)?
                .ok_or_else(invalid)?;
        }
        if output.terminal {
            if !self
                .flow
                .spend(reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            if reports.index.operations.get(&id) != Some(&owner) {
                return Err(invalid());
            }
            self.output_effect_stage(owner, Port::Operation(id), span)?
                .ok_or_else(invalid)?;
        }
        for (&part, found) in &output.parts {
            if !found.projection && !found.output {
                return Err(invalid());
            }
            for (port, seen) in [
                (
                    Port::Projection {
                        point: id,
                        step: part,
                    },
                    found.projection,
                ),
                (Port::Output { point: id, part }, found.output),
            ] {
                if !seen {
                    continue;
                }
                let stage = self
                    .output_effect_stage(owner, port, span)?
                    .ok_or_else(invalid)?;
                let input = match stage.kind {
                    Kind::Projection { input, .. } => Some(input),
                    Kind::Part { input, .. } => input,
                    _ => return Err(invalid()),
                };
                if input != found.input {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
