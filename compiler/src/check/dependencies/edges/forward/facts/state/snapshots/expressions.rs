use super::super::super::super::effects::Effect;
use super::*;

mod blocks;
mod calls;
mod conversions;
mod forwarded;
mod logical;
mod reads;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Scalar {
    pub(super) domain: Domain,
    pub(super) local: Option<hir::LocalId>,
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
                + usize::from(value.local.is_some())
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
    ) -> Result<Option<bool>> {
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let Port::Normal(point) = port else {
            return Ok(Some(false));
        };
        if let Some(logical) = observations.logical.get(&port) {
            if observations.forwarded.contains_key(&port)
                || observations.sources.contains_key(&port)
            {
                return Err(invalid(span));
            }
            return self
                .logical_expression(point, logical, flow, span)
                .map(|()| Some(false));
        }
        if let Some(forward) = observations.forwarded.get(&port) {
            if observations.sources.contains_key(&port) {
                return Err(invalid(span));
            }
            if matches!(forward.input, Port::BlockResult(_)) {
                return self
                    .block_expression(point, forward, observations, flow, span)
                    .map(|()| Some(false));
            }
            return self
                .forward_expression(point, forward, flow, span)
                .map(|()| Some(false));
        }
        let Some(source) = observations.sources.get(&port) else {
            return Ok(Some(false));
        };
        if !flow.spend(3) {
            return Err(budget(span));
        }
        let (owner, effect) = *source;
        if matches!(effect, Effect::Coercion(_)) {
            return self
                .converted_expression(point, source, flow, span)
                .map(|()| Some(false));
        }
        if matches!(effect, Effect::Call { .. }) {
            let control = observations
                .call_controls
                .get(&port)
                .ok_or_else(|| invalid(span))?;
            return self.call_expression(point, source, *control, flow, span);
        }
        let (normal, control) = match effect {
            Effect::Borrow(_) => return Ok(Some(false)),
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
                        local: None,
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
                                    local: None,
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
        self.record_expression(point, fact, flow, span)
            .map(|()| Some(false))
    }

    pub(super) fn record_expression(
        &mut self,
        point: PointId,
        fact: Fact,
        flow: &mut Flow,
        span: Span,
    ) -> Result<()> {
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

#[cfg(test)]
mod boundaries;

#[cfg(test)]
mod places;
