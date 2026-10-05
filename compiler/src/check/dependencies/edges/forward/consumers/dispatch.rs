use super::*;
use crate::check::dependencies::bodies::completion::Shape;

mod primary;
mod records;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    pub(crate) point: PointId,
    pub(crate) slot: Slot,
}

impl Checker {
    pub(in super::super) fn scalar_dispatch_source(
        &mut self,
        reports: &Reports,
        input: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<Source>> {
        let invalid = || Diagnostic::unsupported("proof scalar-dispatch identity mismatch", span);
        let Some(point) = self.dispatch_consumer(reports, input, owner, span)? else {
            return Ok(None);
        };
        let Some(block) = self.dispatch_result_body(reports, point, owner, span)? else {
            return Ok(None);
        };
        if !self.flow.spend(
            reports.results.len().checked_ilog2().unwrap_or(0) as usize
                + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                + 5,
        ) {
            return Err(Diagnostic::unsupported(
                "proof scalar-dispatch budget exhausted",
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
        let body = &self.bodies[&block];
        if !matches!(body.completion.result, Shape::Scalar(_)) {
            return Ok(None);
        }
        let Layout::Slots(slots) = &body.layout else {
            return Err(invalid());
        };
        if slots.len() != 1
            || slots[0].field.is_some()
            || slots[0].mutable
            || slots[0].shape != body.completion.result
        {
            return Err(invalid());
        }
        Ok(Some(Source {
            point,
            slot: Slot { block, index: 0 },
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
