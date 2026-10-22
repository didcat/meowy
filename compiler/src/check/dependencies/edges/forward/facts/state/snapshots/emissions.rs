use super::*;
use expressions::{Fact, Scalar};

impl Snapshots {
    pub(super) fn emit(
        &mut self,
        id: hir::EmitId,
        before: Value,
        input: &observations::Emission,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Value> {
        if self.emissions.len() >= MAX_EDGES || !flow.spend(7) {
            return Err(budget(span));
        }
        let source = self.rows.get(&Port::Normal(input.input));
        let fact = self.expressions.get(&input.input);
        if self.emissions.contains_key(&id)
            || input.input == input.statement
            || input.reached != source.is_some()
            || source.is_some_and(|row| {
                row.owner != input.owner || row.before != row.after || row.after != before
            })
            || source.is_none() && before != Value::Missing
            || fact.is_some_and(|fact| fact.owner != input.owner || source.is_none())
        {
            return Err(invalid(span));
        }
        let state = match before {
            Value::Missing => None,
            Value::Known(id) => {
                let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                if state.owner() != input.owner {
                    return Err(invalid(span));
                }
                Some(state)
            }
        };
        let value = match state {
            None => None,
            Some(state) if state.reach == FALSE => Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: input.control.unwrap_or(true),
            }),
            Some(_) => {
                let weight = fact.map_or(1, Fact::weight);
                if weight > self.room() || !flow.spend(weight + 2) {
                    return Err(budget(span));
                }
                match fact.and_then(|fact| fact.value.as_ref()) {
                    None => None,
                    Some(value) => {
                        if !value.domain.valid() {
                            return Err(invalid(span));
                        }
                        match (input.control, value.control) {
                            (None, false) => None,
                            (control, dependent) => {
                                let mut value = value.clone();
                                value.control = dependent || control == Some(true);
                                Some(value)
                            }
                        }
                    }
                }
            }
        };
        let after = if value.is_some() {
            before
        } else {
            Value::Missing
        };
        let fact = Fact {
            owner: input.owner,
            value,
        };
        let weight = fact.weight();
        if !flow.spend(weight + 2) {
            return Err(budget(span));
        }
        let parts = self.parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        self.emissions.insert(id, fact);
        self.parts = parts;
        Ok(after)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
