use super::*;
use crate::hir::Program;

mod access;

pub(in super::super) use access::Access;
pub(in super::super) type Accesses = BTreeMap<hir::LocalId, Access>;

impl Checker {
    pub(in super::super) fn declared_access(
        &mut self,
        program: &Program,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Accesses, usize)> {
        let budget = || Diagnostic::unsupported("proof declared-access budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof declared-access identity mismatch", span);
        let params = &self.program_signatures(program, reports, span)?.params;
        let count = params.len();
        let parts = parts.min(MAX_EDGES).checked_sub(count).ok_or_else(budget)?;
        if !self
            .flow
            .spend(count.saturating_mul(count.checked_ilog2().unwrap_or(0) as usize + 2) + 1)
        {
            return Err(budget());
        }
        let mut access = Accesses::new();
        for &param in params.keys() {
            let ty = program.locals.get(param).ok_or_else(invalid)?;
            access.insert(param, access::classify(ty, &mut self.flow, span)?);
        }
        Ok((access, parts))
    }
}

#[cfg(test)]
mod tests;
