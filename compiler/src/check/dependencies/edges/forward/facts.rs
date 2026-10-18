use super::entries::Reports;
use super::*;
use crate::hir;

mod domain;
mod literals;

impl Checker {
    pub(crate) fn qualify_scalar_seeds(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
    ) -> Result<()> {
        self.program_signatures(program, reports, span)?;
        self.scalar_literal_seeds(reports, span, MAX_EDGES)?;
        Ok(())
    }
}
