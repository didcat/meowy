use super::*;

impl Checker {
    pub(in super::super::super) fn argument_source_view<'a>(
        &mut self,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(&'a Sources, usize)> {
        let budget =
            || Diagnostic::unsupported("proof argument-source view budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof argument-source view identity mismatch", span);
        if reports.argument_sources.len() > MAX_EDGES {
            return Err(budget());
        }
        self.validate_call_arguments(reports, span)?;
        if !self.flow.spend(reports.arguments.len() + 1) {
            return Err(budget());
        }
        let mut ctx = Lookup::new(reports, parts);
        let mut count = 0;
        for (&point, arg) in &reports.arguments {
            if !arg.normal {
                continue;
            }
            if !self
                .flow
                .spend(reports.argument_sources.len().checked_ilog2().unwrap_or(0) as usize + 3)
            {
                return Err(budget());
            }
            let &(owner, direct) = reports.argument_sources.get(&point).ok_or_else(invalid)?;
            if owner != arg.owner
                || direct.point != point
                || direct != self.direct_source(&mut ctx, point, owner, span)?
            {
                return Err(invalid());
            }
            count += 1;
        }
        if count != reports.argument_sources.len() {
            return Err(invalid());
        }
        Ok((&reports.argument_sources, ctx.parts))
    }
}

#[cfg(test)]
mod tests;
