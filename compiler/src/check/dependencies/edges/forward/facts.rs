use super::entries::Reports;
use super::*;
use crate::hir;

mod declarations;
mod domain;
mod literals;

impl Checker {
    pub(crate) fn qualify_scalar_seeds(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        self.scalar_seed_context(program, reports, span, MAX_EDGES)?;
        Ok(())
    }

    pub(self) fn scalar_seed_context<'a>(
        &mut self,
        program: &'a hir::Program,
        reports: &'a Reports,
        span: Span,
        parts: usize,
    ) -> Result<(
        literals::Literals<'a>,
        declarations::Declarations<'a>,
        usize,
    )> {
        let (declared, parts) = self.scalar_declared_seeds(program, reports, span, parts)?;
        let (literals, parts) = self.scalar_literal_seeds(reports, span, parts)?;
        Ok((literals, declared, parts))
    }
}
