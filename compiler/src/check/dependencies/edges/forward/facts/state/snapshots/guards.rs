use super::super::super::guards::Input;
use super::*;
use domain::Values;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Authority {
    Unknown,
    Dependent,
    Bottom,
    Boolean(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub(super) input: Input,
    pub(super) authority: Authority,
}

impl Snapshots {
    pub(super) fn guard(
        &mut self,
        port: Port,
        input: &Input,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if self.guards.len() >= MAX_EDGES || !flow.spend(9) {
            return Err(budget(span));
        }
        let Port::Normal(point) = port else {
            return Err(invalid(span));
        };
        let std::collections::hash_map::Entry::Vacant(entry) = self.guards.entry(point) else {
            return Err(invalid(span));
        };
        let row = self.rows.get(&port).ok_or_else(|| invalid(span))?;
        let source = self.rows.get(&Port::Normal(input.source.input));
        let fact = self.expressions.get(&input.source.input);
        if point == input.point
            || point == input.source.input
            || !input.source.normal
            || input.source.control && input.control != Some(true)
            || row.owner != input.source.owner
            || row.before != row.after
            || source.is_some_and(|source| {
                source.owner != row.owner
                    || matches!(row.before, Value::Known(_)) && source.after != row.before
            })
            || fact.is_some_and(|fact| fact.owner != row.owner || source.is_none())
        {
            return Err(invalid(span));
        }
        let state = match row.before {
            Value::Missing => None,
            Value::Known(id) => {
                let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                if state.owner() != row.owner {
                    return Err(invalid(span));
                }
                Some(state)
            }
        };
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
        let authority = if input.control == Some(true) || value.is_some_and(|value| value.control) {
            Authority::Dependent
        } else if input.control.is_none() || state.is_none() {
            Authority::Unknown
        } else if state.is_some_and(|state| state.reach == FALSE) || values == Some(0) {
            Authority::Bottom
        } else {
            values.map_or(Authority::Unknown, Authority::Boolean)
        };
        let parts = self.parts.checked_sub(1).ok_or_else(|| budget(span))?;
        let row = self.rows.get_mut(&port).ok_or_else(|| invalid(span))?;
        let control = match authority {
            Authority::Dependent => Some(true),
            Authority::Boolean(_) => Some(false),
            Authority::Unknown | Authority::Bottom => None,
        };
        entry.insert(Fact {
            input: *input,
            authority,
        });
        row.control = history::combine(row.control, control);
        self.parts = parts;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
