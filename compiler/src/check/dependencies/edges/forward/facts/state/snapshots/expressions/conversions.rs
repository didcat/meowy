use super::*;
use crate::check::dependencies::Coercion;

impl Snapshots {
    pub(super) fn converted_expression(
        &mut self,
        point: PointId,
        source: &(usize, Effect),
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let value = self.converted_value(point, source, flow, span)?;
        self.record_expression(
            point,
            Fact {
                owner: source.0,
                value,
            },
            flow,
            span,
        )
    }

    pub(self) fn converted_value(
        &self,
        point: PointId,
        source: &(usize, Effect),
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<Scalar>> {
        if !flow.spend(14) {
            return Err(budget(span));
        }
        let (owner, Effect::Coercion(op)) = source else {
            return Err(invalid(span));
        };
        let types = op.scalar.ok_or_else(|| invalid(span))?;
        if !Coercion::valid_scalars(op.op, op.primary, op.scalar)
            || op.projected
            || op.source.is_some()
            || op.fields.is_some()
            || !op.result
            || point == op.input
        {
            return Err(invalid(span));
        }
        let row = self
            .rows
            .get(&Port::Normal(point))
            .ok_or_else(|| invalid(span))?;
        let operation = self.rows.get(&Port::Operation(point));
        let input = self.rows.get(&Port::Normal(op.input));
        let fact = self.expressions.get(&op.input);
        if row.owner != *owner
            || row.before != row.after
            || op.operation != operation.is_some()
            || input.is_some_and(|input| input.owner != *owner)
            || operation.is_some_and(|operation| {
                operation.owner != *owner
                    || operation.before != input.map_or(Value::Missing, |input| input.after)
                    || operation.before != operation.after
                    || operation.control != input.and_then(|input| input.control)
            })
            || row.before != operation.map_or(Value::Missing, |operation| operation.after)
            || row.control != operation.and_then(|operation| operation.control)
            || fact.is_some_and(|fact| fact.owner != *owner || input.is_none())
        {
            return Err(invalid(span));
        }
        let Value::Known(id) = row.after else {
            return Ok(None);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != *owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: op.control || row.control.unwrap_or(true),
            }));
        }
        if row.control.is_none() {
            return Ok(None);
        }
        let Some(value) = fact.and_then(|fact| fact.value.as_ref()) else {
            return Ok(None);
        };
        let source = Domain::scalar(types.source.kind, flow, span)?.ok_or_else(|| invalid(span))?;
        let target = Domain::scalar(types.target.kind, flow, span)?
            .ok_or_else(|| invalid(span))?
            .join(&Domain::NULL, flow, span)?;
        if !value.domain.fits(&source, flow, span)? || !value.domain.fits(&target, flow, span)? {
            return Ok(None);
        }
        let weight = fact.unwrap().weight();
        if weight > self.parts || !flow.spend(weight) {
            return Err(budget(span));
        }
        let mut value = value.clone();
        value.control |= op.control;
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests;
