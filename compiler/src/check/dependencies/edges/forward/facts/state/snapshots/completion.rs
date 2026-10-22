use super::*;

impl Snapshots {
    pub(super) fn complete(
        &self,
        port: Port,
        before: Value,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if !flow.spend(5) {
            return Err(budget(span));
        }
        let (Port::Leave(block) | Port::BlockNormal(block) | Port::BlockResult(block)) = port
        else {
            return Err(invalid(span));
        };
        let Some(scope) = observations.scopes.get(&block) else {
            return Ok(Value::Missing);
        };
        let control = match port {
            Port::Leave(_) => observations
                .completion
                .leaves
                .get(&block)
                .copied()
                .flatten(),
            Port::BlockNormal(_) if scope.normal => scope.control,
            Port::BlockResult(_) if scope.result => scope.control,
            _ => return Err(invalid(span)),
        };
        let Value::Known(id) = before else {
            return Ok(Value::Missing);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != scope.owner {
            return Err(invalid(span));
        }
        Ok(if control.is_some() {
            before
        } else {
            Value::Missing
        })
    }
}

#[cfg(test)]
mod tests;
