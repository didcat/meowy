use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::field_results::lookup::Lookup, results::inputs::direct::Direct,
};

pub(crate) type Sources = BTreeMap<PointId, (usize, Direct)>;

mod qualified;

impl Checker {
    pub(in super::super) fn argument_sources(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        parts: usize,
    ) -> Result<(Sources, usize)> {
        let budget = || Diagnostic::unsupported("proof argument-source budget exhausted", span);
        self.validate_call_arguments(reports, span)?;
        if !self.flow.spend(reports.arguments.len() + 1) {
            return Err(budget());
        }
        let mut ctx = Lookup::new(reports, parts);
        let mut sources = Sources::new();
        for (&point, arg) in &reports.arguments {
            if !arg.normal {
                continue;
            }
            if sources.len() >= limit.min(MAX_EDGES)
                || !self
                    .flow
                    .spend(sources.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            {
                return Err(budget());
            }
            ctx.parts = ctx.parts.checked_sub(1).ok_or_else(budget)?;
            let direct = self.direct_source(&mut ctx, point, arg.owner, span)?;
            sources.insert(point, (arg.owner, direct));
        }
        Ok((sources, ctx.parts))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod retained;
