use super::*;
use crate::check::dependencies::edges::forward::controls;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super::super) enum Authority {
    Unknown,
    Dependent,
    Independent,
}

impl Checker {
    pub(super) fn call_entry_authority(
        &mut self,
        point: PointId,
        transfer: &Transfer<'_>,
        span: Span,
    ) -> Result<Authority> {
        let budget = || Diagnostic::unsupported("proof call-guard budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof call-guard identity mismatch", span);
        if !self.flow.spend(
            self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                + self.proofs.calls.len().checked_ilog2().unwrap_or(0) as usize
                + 7,
        ) {
            return Err(budget());
        }
        let call = self.invocations.get(&transfer.site).ok_or_else(invalid)?;
        let source = self.points.get(point).ok_or_else(invalid)?;
        if call.point != point
            || call.site != transfer.site
            || call.owner != transfer.call.owner
            || !source.complete
            || source.kind != PointKind::Expr
            || source.owner != call.owner
            || source.span != call.span
            || transfer.observed.is_some_and(|(operation, result)| {
                !operation && !result || result && !call.may_return
            })
        {
            return Err(invalid());
        }
        let Some(snapshot) = transfer.call.snapshot else {
            return Ok(Authority::Unknown);
        };
        let header = snapshot.header;
        if header.site != call.site
            || header.function != call.function
            || header.count != call.args.len()
            || self.proofs.calls.get(&call.site) != Some(&header.entered)
            || header.entered >> 1 >= self.flow.nodes.len()
        {
            return Err(invalid());
        }
        if !transfer.observed.is_some_and(|(operation, _)| operation)
            || snapshot.normal != header.count
            || transfer.unknown.is_none()
        {
            return Ok(Authority::Unknown);
        }
        let control = controls::dependency(
            point,
            call.owner,
            call.control,
            &self.points,
            &self.branch_inputs,
            &mut self.flow,
        )
        .map_err(|error| error.diagnostic("call-guard", span))?;
        Ok(match control {
            Some(false) => Authority::Independent,
            Some(true) => Authority::Dependent,
            None => Authority::Unknown,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
