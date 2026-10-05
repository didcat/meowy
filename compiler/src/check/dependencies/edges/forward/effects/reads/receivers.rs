use super::*;
use crate::check::dependencies::{bodies::completion::Shape, grouped::MAX_GROUPS};

mod scope;

impl Checker {
    pub(in super::super::super) fn read_receiver_input(
        &mut self,
        reports: &Reports,
        id: PointId,
        owner: usize,
        span: Span,
    ) -> Result<Option<PointId>> {
        let budget = || Diagnostic::unsupported("proof read-receiver budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof read-receiver identity mismatch", span);
        if !self
            .flow
            .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget());
        }
        let Some((
            reported,
            effect @ Effect::Read {
                local,
                storage,
                normal,
                ..
            },
        )) = reports.effects.get(&id)
        else {
            return Ok(None);
        };
        if *reported != owner || self.read_effect(reports, id, owner, span)? != *effect {
            return Err(invalid());
        }
        if !normal || local != storage {
            return Ok(None);
        }
        let work = [
            reports.receivers.len(),
            reports.effects.len(),
            self.dispatch_ops.len(),
            self.proofs.receivers.len(),
            self.proofs.aliases.len(),
            self.proofs.temporaries.len(),
        ]
        .into_iter()
        .fold(14, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        let Some(&(indexed, dispatch)) = reports.receivers.get(local) else {
            return Ok(None);
        };
        let work = [
            self.fields.len(),
            self.group_inputs.len(),
            self.coercions.len(),
            self.narrowings.len(),
            self.typed_ops.len(),
            self.dispatch_ops.len(),
            reports.consumers.len(),
        ]
        .into_iter()
        .fold(7, |work, len| {
            work + len.checked_ilog2().unwrap_or(0) as usize
        });
        if !self.flow.spend(work) {
            return Err(budget());
        }
        if self.fields.contains_key(&id)
            || self.group_inputs.contains_key(&id)
            || self.coercions.contains_key(&id)
            || self.narrowings.contains_key(&id)
            || self.typed_ops.contains_key(&id)
            || self.dispatch_ops.contains_key(&id)
            || reports.consumers.contains_key(&id)
        {
            return Err(invalid());
        }
        let op = self.dispatch_ops.get(&dispatch).ok_or_else(invalid)?;
        if indexed != owner
            || op.owner != owner
            || op.local != *local
            || !self.proofs.receivers.contains(local)
            || self.proofs.aliases.contains_key(local)
            || self.proofs.temporaries.contains_key(local)
        {
            return Err(invalid());
        }
        let Some((reported, Effect::Dispatch(observed))) = reports.effects.get(&dispatch) else {
            return Ok(None);
        };
        if *reported != owner
            || observed.input != op.input
            || observed.local != op.local
            || observed.block != op.block
            || observed.receiver != op.receiver
            || observed.normal != op.normal
            || observed.control != op.control
        {
            return Err(invalid());
        }
        if !observed.initialized || !matches!(op.receiver, Shape::Scalar(_)) {
            return Ok(None);
        }
        let (input, block, source) = (op.input, op.block, op.span);
        if !self.validate_dispatch_effect(reports, owner, Port::Operation(dispatch), span)? {
            return Err(invalid());
        }
        if !self
            .flow
            .spend(self.bodies.len().checked_ilog2().unwrap_or(0) as usize + 4)
        {
            return Err(budget());
        }
        let input_span = self.points[input].span;
        if input_span.start < source.start
            || input_span.start > input_span.end
            || input_span.end > self.bodies[&block].span.start
        {
            return Err(invalid());
        }
        self.receiver_scope(id, block, owner, span, MAX_GROUPS)?;
        Ok(Some(input))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;

#[cfg(test)]
mod guards;
