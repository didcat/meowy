use super::*;

impl Checker {
    pub(super) fn signature_root(
        &mut self,
        reports: &Reports,
        count: usize,
        span: Span,
    ) -> Result<hir::BlockId> {
        let budget = || Diagnostic::unsupported("proof function-signature budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof function-signature identity mismatch", span);
        if count >= MAX_ENTRIES
            || !self.flow.spend(
                count
                    + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                    + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                    + 5,
            )
        {
            return Err(budget());
        }
        let root = reports.entries.get(&0).ok_or_else(invalid)?.0;
        if reports.entries.len() != count + 1
            || !self
                .bodies
                .get(&root)
                .is_some_and(|body| body.owner == 0 && body.parent.is_none())
        {
            return Err(invalid());
        }
        Ok(root)
    }

    pub(in super::super) fn validate_function_signatures(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof function-signature budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof function-signature identity mismatch", span);
        let index = &reports.signatures;
        if index.params.len() > MAX_EDGES / 2 {
            return Err(budget());
        }
        self.signature_root(reports, index.functions.len(), span)?;
        let mut count = 0usize;
        for &function in index.functions.keys() {
            let header = self.checked_function_signature(reports, function, span)?;
            count = count
                .checked_add(header.params.len())
                .filter(|count| *count <= index.params.len())
                .ok_or_else(invalid)?;
        }
        if count != index.params.len() {
            return Err(invalid());
        }
        Ok(())
    }

    pub(in super::super) fn checked_function_signature<'a>(
        &mut self,
        reports: &'a Reports,
        function: hir::FunctionId,
        span: Span,
    ) -> Result<&'a Signature> {
        let budget = || Diagnostic::unsupported("proof function-signature budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof function-signature identity mismatch", span);
        let index = &reports.signatures;
        if !self.flow.spend(
            index.functions.len().checked_ilog2().unwrap_or(0) as usize
                + reports.entries.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 8,
        ) {
            return Err(budget());
        }
        let header = index.functions.get(&function).ok_or_else(invalid)?;
        if header.params.len() > MAX_ITEMS
            || !self.flow.spend(
                header.params.len()
                    * (index.params.len().checked_ilog2().unwrap_or(0) as usize
                        + self.proofs.bindings.len().checked_ilog2().unwrap_or(0) as usize
                        + 4),
            )
        {
            return Err(budget());
        }
        let owner = function.checked_add(1).ok_or_else(invalid)?;
        if !(Completion {
            normal: header.shape != Shape::Never,
            result: header.shape,
        })
        .valid()
            || reports.entries.get(&owner).map(|(body, _)| *body) != Some(header.body)
            || !self
                .bodies
                .get(&header.body)
                .is_some_and(|body| body.owner == owner && body.parent.is_none())
        {
            return Err(invalid());
        }
        for (position, local) in header.params.iter().enumerate() {
            if *local >= reports.locals
                || !self.proofs.bindings.contains_key(local)
                || index.params.get(local) != Some(&(function, position))
            {
                return Err(invalid());
            }
        }
        Ok(header)
    }

    pub(in super::super) fn validate_signature_calls(
        &mut self,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof call-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-effect identity mismatch", span);
        if self.invocations.len() > MAX_EDGES || !self.flow.spend(self.invocations.len() + 1) {
            return Err(budget());
        }
        for call in self.invocations.values() {
            if call.args.len() > MAX_ITEMS
                || !self.flow.spend(
                    reports
                        .signatures
                        .functions
                        .len()
                        .checked_ilog2()
                        .unwrap_or(0) as usize
                        + 3,
                )
            {
                return Err(budget());
            }
            let header = reports
                .signatures
                .functions
                .get(&call.function)
                .ok_or_else(invalid)?;
            if header.params.len() != call.args.len() || header.shape != call.shape {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
