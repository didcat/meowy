use super::*;
use guards::Authority;

impl Snapshots {
    pub(super) fn branch(
        &mut self,
        edge: Edge,
        before: Value,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if !flow.spend(5) {
            return Err(budget(span));
        }
        let Port::Normal(point) = edge.from else {
            return Ok(before);
        };
        let Some(fact) = self.guards.get(&point).copied() else {
            return Ok(Value::Missing);
        };
        let (arm, bit) = match edge.route {
            Route::True => (0, 2),
            Route::False => (1, 1),
            _ => return Err(invalid(span)),
        };
        let row = self.rows.get(&edge.from).ok_or_else(|| invalid(span))?;
        if edge.to != Port::Entry(fact.input.arms[arm])
            || row.owner != fact.input.source.owner
            || row.before != row.after
            || row.after != before
        {
            return Err(invalid(span));
        }
        let Value::Known(id) = before else {
            return Ok(before);
        };
        let state = self.states.get(id).ok_or_else(|| invalid(span))?;
        if state.owner() != row.owner {
            return Err(invalid(span));
        }
        if state.reach == FALSE {
            return Ok(before);
        }
        let allowed = match fact.authority {
            Authority::Unknown => return Ok(Value::Missing),
            Authority::Dependent => {
                return Ok(if fact.input.control == Some(true) {
                    before
                } else {
                    Value::Missing
                });
            }
            Authority::Bottom => false,
            Authority::Boolean(values) => {
                if !(1..=3).contains(&values)
                    || fact.input.control != Some(false)
                    || fact.input.source.control
                {
                    return Err(invalid(span));
                }
                values & bit != 0
            }
        };
        if allowed {
            return Ok(before);
        }
        let state = state.restrict(FALSE, flow, span, self.room())?;
        self.keep(state, span)
    }
}

#[cfg(test)]
mod tests;
