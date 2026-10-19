use super::super::super::super::effects::Effect;
use super::*;

mod reads;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Scalar {
    pub(super) domain: Domain,
    pub(super) version: Option<Version>,
    pub(super) origin: Option<Origin>,
    pub(super) control: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub(super) owner: usize,
    pub(super) value: Option<Scalar>,
}

impl Fact {
    pub(super) fn weight(&self) -> usize {
        1 + self.value.as_ref().map_or(0, |value| {
            value.domain.weight()
                + usize::from(value.version.is_some())
                + usize::from(value.origin.is_some())
        })
    }
}

impl Snapshots {
    pub(super) fn expression(
        &mut self,
        port: Port,
        observations: &observations::Observations<'_, '_>,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        let Port::Normal(point) = port else {
            return Ok(());
        };
        let Some(source) = observations.sources.get(&port) else {
            return Ok(());
        };
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let (owner, effect) = *source;
        let (normal, control) = match effect {
            Effect::Scalar(observed) => (observed.result, observed.control),
            Effect::Read {
                normal, control, ..
            } => (*normal, *control),
            _ => return Err(invalid(span)),
        };
        let row = self.rows.get(&port).ok_or_else(|| invalid(span))?;
        if row.owner != *owner || row.before != row.after || !normal {
            return Err(invalid(span));
        }
        let value = match row.before {
            Value::Missing => None,
            Value::Known(id) => {
                let state = self.states.get(id).ok_or_else(|| invalid(span))?;
                if state.owner() != *owner {
                    return Err(invalid(span));
                }
                if state.reach == FALSE {
                    Some(Scalar {
                        domain: Domain::BOTTOM,
                        version: None,
                        origin: None,
                        control,
                    })
                } else {
                    match effect {
                        Effect::Scalar(observed) => observed
                            .value
                            .map(|value| {
                                Ok(Scalar {
                                    domain: Domain::literal(value, flow, span)?,
                                    version: None,
                                    origin: Some(Origin::Expression(*owner, point)),
                                    control,
                                })
                            })
                            .transpose()?,
                        Effect::Read { local, storage, .. } if local == storage => {
                            self.read(state, *local, control, observations, flow, span)?
                        }
                        _ => return Err(invalid(span)),
                    }
                }
            }
        };
        let fact = Fact {
            owner: *owner,
            value,
        };
        let weight = fact.weight();
        if self.expressions.len() >= MAX_EDGES || !flow.spend(weight + 2) {
            return Err(budget(span));
        }
        if self.expressions.contains_key(&point) {
            return Err(invalid(span));
        }
        let parts = self.parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        self.expressions.insert(point, fact);
        self.parts = parts;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
