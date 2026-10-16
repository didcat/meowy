use super::*;
use std::collections::BTreeSet;

impl Checker {
    pub(in super::super) fn program_signatures<'a>(
        &mut self,
        program: &hir::Program,
        reports: &'a Reports,
        span: Span,
    ) -> Result<&'a Index> {
        let budget = || Diagnostic::unsupported("proof function-signature budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof function-signature identity mismatch", span);
        let index = &reports.signatures;
        if index.params.len() > MAX_EDGES / 2 {
            return Err(budget());
        }
        let root = self.signature_root(reports, program.functions.len(), span)?;
        if program.locals.len() != reports.locals
            || program.body.id != root
            || program.functions.len() != index.functions.len()
        {
            return Err(invalid());
        }
        let mut owners = BTreeSet::new();
        let mut count = 0usize;
        for function in &program.functions {
            if function.params.len() > MAX_ITEMS
                || !self.flow.spend(
                    function.params.len()
                        + owners.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 3,
                )
            {
                return Err(budget());
            }
            if !owners.insert(function.id) {
                return Err(invalid());
            }
            let header = self.checked_function_signature(reports, function.id, span)?;
            if header.body != function.body.id
                || header.params != function.params
                || header.shape != Invocation::result_shape(&function.result)
            {
                return Err(invalid());
            }
            count = count
                .checked_add(header.params.len())
                .filter(|count| *count <= index.params.len())
                .ok_or_else(invalid)?;
        }
        if count != index.params.len() {
            return Err(invalid());
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests;
