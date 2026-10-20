use super::*;
use domain::Values;
use observations::Logical;

impl Snapshots {
    pub(super) fn logical_expression(
        &mut self,
        point: PointId,
        logical: &Logical,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if !flow.spend(10) {
            return Err(budget(span));
        }
        let guard = self
            .guards
            .get(&logical.condition)
            .ok_or_else(|| invalid(span))?;
        let input = guard.input;
        let row = self
            .rows
            .get(&Port::Normal(point))
            .ok_or_else(|| invalid(span))?;
        if input.point != point
            || input.source.owner != row.owner
            || !input.source.normal
            || input.source.control && input.control != Some(true)
            || row.before != row.after
            || [point, logical.condition, input.source.input].contains(&logical.right)
        {
            return Err(invalid(span));
        }
        let left =
            self.logical_operand(input.source.input, logical.condition, row.owner, flow, span)?;
        let right = self.logical_operand(
            logical.right,
            input.arms[usize::from(logical.or)],
            row.owner,
            flow,
            span,
        )?;
        let control = input.control == Some(true)
            || left.is_some_and(|(_, control)| control)
            || right.is_some_and(|(_, control)| control);
        let values = match row.before {
            Value::Missing => None,
            Value::Known(id) => {
                let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                if state.owner() != row.owner {
                    return Err(invalid(span));
                }
                if state.reach == FALSE {
                    Some(0)
                } else if input.control.is_none() {
                    None
                } else {
                    left.and_then(|(values, _)| {
                        let bypass = 1 << u8::from(logical.or);
                        let skipped = values & bypass;
                        if values & (3 ^ bypass) == 0 {
                            Some(skipped)
                        } else {
                            right.map(|(values, _)| skipped | values)
                        }
                    })
                }
            }
        };
        self.record_expression(
            point,
            Fact {
                owner: row.owner,
                value: values.map(|values| Scalar {
                    domain: Domain {
                        values: (values != 0).then_some(Values::Bool(values)),
                        null: false,
                    },
                    local: None,
                    version: None,
                    origin: (values != 0).then_some(Origin::Expression(row.owner, point)),
                    control,
                }),
            },
            flow,
            span,
        )
    }

    pub(self) fn logical_operand(
        &self,
        point: PointId,
        region: PointId,
        owner: usize,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<(u8, bool)>> {
        if !flow.spend(8) {
            return Err(budget(span));
        }
        let row = self.rows.get(&Port::Normal(region));
        let source = self.rows.get(&Port::Normal(point));
        let fact = self.expressions.get(&point);
        if row.is_some_and(|row| row.owner != owner || row.before != row.after)
            || source.is_some_and(|source| source.owner != owner || source.before != source.after)
            || fact.is_some_and(|fact| fact.owner != owner || source.is_none())
            || row.is_some_and(|row| {
                source.map_or(row.after != Value::Missing, |source| {
                    row.after != source.after
                })
            })
        {
            return Err(invalid(span));
        }
        let Some(Value::Known(id)) = row.map(|row| row.after) else {
            return Ok(None);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != owner {
            return Err(invalid(span));
        }
        let value = fact.and_then(|fact| fact.value.as_ref());
        let values = match value.map(|value| &value.domain) {
            Some(domain) if *domain == Domain::BOTTOM => Some(0),
            Some(Domain {
                values: Some(Values::Bool(values)),
                null: false,
            }) if (1..=3).contains(values) => Some(*values),
            None => None,
            _ => return Err(invalid(span)),
        };
        let values = if state.reach == FALSE {
            Some(0)
        } else {
            values
        };
        Ok(values.map(|values| (values, value.is_some_and(|value| value.control))))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
