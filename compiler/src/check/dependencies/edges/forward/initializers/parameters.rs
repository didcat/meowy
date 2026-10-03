use super::*;
use std::collections::BTreeSet;

impl Checker {
    pub(in super::super) fn initializer_parameters(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
    ) -> Result<BTreeSet<hir::LocalId>> {
        let budget =
            || Diagnostic::unsupported("proof initializer parameter budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof initializer parameter identity mismatch", span);
        if program.functions.len() >= super::super::entries::MAX_ENTRIES
            || !self.flow.spend(program.functions.len() + 1)
        {
            return Err(budget());
        }
        if program.locals.len() != reports.locals
            || reports.entries.len() != program.functions.len() + 1
        {
            return Err(invalid());
        }
        let limit = reports.locals.min(MAX_EDGES);
        let mut params = BTreeSet::new();
        let mut owners = BTreeSet::new();
        let entries = std::iter::once((None, program.body.id, &[][..])).chain(
            program.functions.iter().map(|function| {
                (
                    Some(function.id),
                    function.body.id,
                    function.params.as_slice(),
                )
            }),
        );
        for (function, block, inputs) in entries {
            if !self.flow.spend(
                reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + owners.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 6,
            ) {
                return Err(budget());
            }
            let owner = match function {
                Some(id) => id.checked_add(1).ok_or_else(invalid)?,
                None => 0,
            };
            if !reports
                .entries
                .get(&owner)
                .is_some_and(|(entry, _)| *entry == block)
                || !self
                    .bodies
                    .get(&block)
                    .is_some_and(|body| body.owner == owner)
                || !owners.insert(owner)
            {
                return Err(invalid());
            }
            for &local in inputs {
                if !self
                    .flow
                    .spend(params.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
                {
                    return Err(budget());
                }
                if local >= reports.locals || params.contains(&local) {
                    return Err(invalid());
                }
                if params.len() >= limit {
                    return Err(budget());
                }
                params.insert(local);
            }
        }
        Ok(params)
    }
}

#[cfg(test)]
mod tests;
