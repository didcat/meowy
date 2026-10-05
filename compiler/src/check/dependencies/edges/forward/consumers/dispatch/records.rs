use super::*;

impl Checker {
    pub(in super::super) fn record_dispatch_body(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<hir::BlockId>> {
        let invalid = || Diagnostic::unsupported("proof record-dispatch identity mismatch", span);
        let Some(point) = self.dispatch_consumer(reports, input, owner, span)? else {
            return Ok(None);
        };
        let Some(block) = self.dispatch_result_body(reports, point, owner, span)? else {
            return Ok(None);
        };
        if !self.flow.spend(
            reports.results.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 4,
        ) {
            return Err(Diagnostic::unsupported(
                "proof record-dispatch budget exhausted",
                span,
            ));
        }
        let Some((reported, result)) = reports.results.get(&block) else {
            return Ok(None);
        };
        if *reported != owner || result.dispatch != Some(point) {
            return Err(invalid());
        }
        self.validate_result_report(reports, block, owner, result, span)?;
        Ok(matches!(self.bodies[&block].completion.result, Shape::Record { .. }).then_some(block))
    }
}

#[cfg(test)]
mod tests;
