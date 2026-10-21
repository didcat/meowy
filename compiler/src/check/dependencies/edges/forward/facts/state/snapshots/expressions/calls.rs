use super::*;
use crate::check::dependencies::{bodies::completion::Shape, sequences::MAX_ITEMS};

impl Snapshots {
    pub(super) fn call_expression(
        &mut self,
        point: PointId,
        source: &(usize, Effect),
        mut control: Option<bool>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        let (
            owner,
            Effect::Call {
                args,
                shape: Shape::Scalar(kind),
                may_return: true,
                result: true,
                operation,
                control: direct,
                ..
            },
        ) = source
        else {
            return Err(invalid(span));
        };
        if args.len() > MAX_ITEMS || !flow.spend(args.len() + 8) {
            return Err(budget(span));
        }
        let row = self
            .rows
            .get(&Port::Normal(point))
            .ok_or_else(|| invalid(span))?;
        let op = self.rows.get(&Port::Operation(point));
        if *direct && control != Some(true)
            || row.owner != *owner
            || row.before != row.after
            || *operation != op.is_some()
            || op.is_some_and(|op| op.owner != *owner || op.after != row.before)
            || !operation && row.before != Value::Missing
        {
            return Err(invalid(span));
        }
        let state = match row.before {
            Value::Missing => None,
            Value::Known(id) => {
                let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                if state.owner() != *owner {
                    return Err(invalid(span));
                }
                Some(state)
            }
        };
        for &arg in args {
            if !flow.spend(5) {
                return Err(budget(span));
            }
            let row = self.rows.get(&Port::Normal(arg));
            let fact = self.expressions.get(&arg);
            if arg == point
                || row.is_some_and(|row| row.owner != *owner || row.before != row.after)
                || fact.is_some_and(|fact| fact.owner != *owner || row.is_none())
            {
                return Err(invalid(span));
            }
            let value = fact.and_then(|fact| fact.value.as_ref());
            if let Some(value) = value {
                if !flow.spend(value.domain.weight()) {
                    return Err(budget(span));
                }
                if !value.domain.valid() {
                    return Err(invalid(span));
                }
            }
            let available = match row.map(|row| row.after) {
                Some(Value::Known(id)) => {
                    if self
                        .states
                        .get(id)
                        .is_none_or(|state| state.owner() != *owner)
                    {
                        return Err(invalid(span));
                    }
                    true
                }
                _ => false,
            };
            let dependent = value.filter(|_| available).map(|value| value.control);
            control = match (control, dependent) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
        }
        let value = match state {
            None => None,
            Some(state) if state.reach == FALSE => Some(Scalar {
                domain: Domain::BOTTOM,
                local: None,
                version: None,
                origin: None,
                control: control.unwrap_or(true),
            }),
            Some(_) => match control {
                None => None,
                Some(control) => Domain::scalar(*kind, flow, span)?.map(|domain| Scalar {
                    domain,
                    local: None,
                    version: None,
                    origin: Some(Origin::Expression(*owner, point)),
                    control,
                }),
            },
        };
        self.record_expression(
            point,
            Fact {
                owner: *owner,
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
mod boundaries;
