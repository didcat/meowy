use super::*;

impl Snapshots {
    pub(super) fn enter_slot(
        &mut self,
        block: hir::BlockId,
        before: Value,
        slot: &observations::Slot,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if !flow.spend(2) {
            return Err(budget(span));
        }
        let Value::Known(id) = before else {
            return Ok(Value::Missing);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        let Some(next) = state.enter_slot(block, slot, flow, span, self.room())? else {
            return Ok(Value::Missing);
        };
        self.keep(next, span)
    }
}

#[cfg(test)]
mod tests;
