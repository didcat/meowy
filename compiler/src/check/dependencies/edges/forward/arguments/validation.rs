use super::*;

impl Checker {
    pub(in super::super) fn validate_call_arguments(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        let budget =
            || Diagnostic::unsupported("proof call-argument report budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof call-argument report identity mismatch", span);
        if reports.arguments.len() > MAX_EDGES || !self.flow.spend(reports.arguments.len() + 1) {
            return Err(budget());
        }
        if reports.arguments.is_empty() {
            return Ok(());
        }
        let (observed, _) = self.call_arguments(reports, span, MAX_EDGES, MAX_EDGES)?;
        for (point, arg) in &reports.arguments {
            if !self
                .flow
                .spend(observed.len().checked_ilog2().unwrap_or(0) as usize + 8)
            {
                return Err(budget());
            }
            let actual = observed.get(point).ok_or_else(invalid)?;
            if arg.site != actual.site
                || arg.position != actual.position
                || arg.param != actual.param
                || arg.owner != actual.owner
                || (!arg.entry && !arg.normal)
                || (arg.entry && !actual.entry)
                || (arg.normal && !actual.normal)
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
