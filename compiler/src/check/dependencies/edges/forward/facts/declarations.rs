use super::*;
use domain::Domain;

#[derive(Debug)]
pub(super) struct Declared<'a> {
    pub(super) ty: &'a hir::Type,
    pub(super) domain: Option<Domain>,
}

#[derive(Debug, Default)]
pub(super) struct Declarations<'a> {
    pub(super) params: BTreeMap<hir::LocalId, (hir::FunctionId, usize, Declared<'a>)>,
    pub(super) results: BTreeMap<hir::FunctionId, Declared<'a>>,
}

impl Checker {
    pub(super) fn scalar_declared_seeds<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &Reports,
        span: Span,
        parts: usize,
    ) -> Result<(Declarations<'a>, usize)> {
        let budget =
            || Diagnostic::unsupported("proof declared-scalar-seed budget exhausted", span);
        self.program_signatures(program, reports, span)?;
        let mut seeds = Declarations::default();
        let mut parts = parts.min(MAX_EDGES);
        for function in &program.functions {
            if !self
                .flow
                .spend(seeds.results.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            let result = self.scalar_declaration(&function.result, span, &mut parts)?;
            seeds.results.insert(function.id, result);
            for (position, &local) in function.params.iter().enumerate() {
                if !self
                    .flow
                    .spend(seeds.params.len().checked_ilog2().unwrap_or(0) as usize + 1)
                {
                    return Err(budget());
                }
                let value = self.scalar_declaration(&program.locals[local], span, &mut parts)?;
                seeds.params.insert(local, (function.id, position, value));
            }
        }
        Ok((seeds, parts))
    }

    pub(self) fn scalar_declaration<'a>(
        &mut self,
        ty: &'a hir::Type,
        span: Span,
        parts: &mut usize,
    ) -> Result<Declared<'a>> {
        let mut value = Declared { ty, domain: None };
        value.domain = Domain::declared(value.ty, &mut self.flow, span)?;
        let weight = 1 + value.domain.as_ref().map_or(0, Domain::weight);
        *parts = parts.checked_sub(weight).ok_or_else(|| {
            Diagnostic::unsupported("proof declared-scalar-seed budget exhausted", span)
        })?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests;
