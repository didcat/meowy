use super::*;
use observations::Forward;

impl Snapshots {
    pub(super) fn forward_expression(
        &mut self,
        point: PointId,
        forward: &Forward,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if !flow.spend(5) {
            return Err(budget(span));
        }
        let Port::Normal(input) = forward.input else {
            return Err(invalid(span));
        };
        let row = self
            .rows
            .get(&Port::Normal(point))
            .ok_or_else(|| invalid(span))?;
        if row.owner != forward.owner || row.before != row.after || point == input {
            return Err(invalid(span));
        }
        let source = self.rows.get(&forward.input);
        let fact = self.expressions.get(&input);
        if source.is_some_and(|source| {
            source.owner != forward.owner
                || matches!(row.before, Value::Known(_)) && source.after != row.before
        }) || fact.is_some_and(|fact| fact.owner != forward.owner || source.is_none())
        {
            return Err(invalid(span));
        }
        let value = match row.before {
            Value::Missing => None,
            Value::Known(id) => {
                if self
                    .states
                    .get(id)
                    .is_none_or(|state| state.owner() != forward.owner)
                {
                    return Err(invalid(span));
                }
                let weight = fact.map_or(1, Fact::weight);
                if weight > self.parts || !flow.spend(weight) {
                    return Err(budget(span));
                }
                fact.and_then(|fact| fact.value.clone()).map(|mut value| {
                    value.control |= forward.control;
                    value
                })
            }
        };
        self.record_expression(
            point,
            Fact {
                owner: forward.owner,
                value,
            },
            flow,
            span,
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod coercions;

#[cfg(test)]
mod narrowing;

#[cfg(test)]
mod ascriptions;

#[cfg(test)]
mod histories;
