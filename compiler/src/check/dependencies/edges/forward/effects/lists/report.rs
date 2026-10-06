use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Input {
    pub(crate) point: PointId,
    pub(crate) plan: Option<(bool, CoercionKind)>,
    pub(crate) source: Option<Shape>,
    pub(crate) projected: bool,
    pub(crate) converted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) capacity: usize,
    pub(crate) contextual: bool,
    pub(crate) normal: bool,
    pub(crate) control: bool,
    pub(crate) inputs: Vec<Input>,
    pub(crate) constructed: bool,
    pub(crate) result: bool,
}

impl Checker {
    pub(in super::super) fn record_list_effect(
        &mut self,
        owner: usize,
        port: Port,
        effects: &mut Effects,
        limit: usize,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let budget = || Diagnostic::unsupported("proof list-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof list-effect identity mismatch", span);
        let id = match port {
            Port::Projection { point, .. }
            | Port::Conversion { point, .. }
            | Port::Operation(point)
            | Port::Normal(point) => point,
            _ => return Err(invalid()),
        };
        if !self.flow.spend(
            self.lists.len().checked_ilog2().unwrap_or(0) as usize
                + self.sequences.len().checked_ilog2().unwrap_or(0) as usize
                + self.list_inputs.len().checked_ilog2().unwrap_or(0) as usize
                + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 8,
        ) {
            return Err(budget());
        }
        let op = self.lists.get(&id).ok_or_else(invalid)?;
        if op.count > crate::list::MAX_CAPACITY || !self.flow.spend(op.count * 10 + 6) {
            return Err(budget());
        }
        let sequence = self
            .sequences
            .get(&SequenceSource::Expr(id))
            .ok_or_else(invalid)?;
        let inputs = self.list_inputs.get(&id);
        if op.owner != owner
            || sequence.owner != owner
            || sequence.items.len() != op.count
            || op.count > op.capacity
            || op.capacity > crate::list::MAX_CAPACITY
            || op.contextual != inputs.is_some()
            || inputs.is_some_and(|inputs| inputs.len() != op.count)
        {
            return Err(invalid());
        }
        let plan = |part: usize| inputs.map(|inputs| (inputs[part].primary, inputs[part].kind));
        let source = |part: usize| inputs.and_then(|inputs| inputs[part].source);
        let stop = inputs.and_then(|inputs| {
            inputs
                .iter()
                .position(|input| input.kind == CoercionKind::Stopped)
        });
        let valid = match port {
            Port::Projection { step, .. } => {
                step < op.count
                    && stop.is_none_or(|stop| step <= stop)
                    && plan(step).is_some_and(|(primary, _)| primary)
            }
            Port::Conversion { part, .. } => {
                part < op.count
                    && stop.is_none_or(|stop| part < stop)
                    && plan(part).is_some_and(|(_, kind)| kind == CoercionKind::Convert)
            }
            _ => op.normal && stop.is_none(),
        };
        if !valid
            || sequence.items.iter().enumerate().any(|(part, point)| {
                point.is_none()
                    || inputs.is_some_and(|inputs| {
                        let input = inputs[part];
                        Some(input.point) != *point
                            || !input.kind.valid_source(input.primary, input.source)
                    })
            })
        {
            return Err(invalid());
        }
        let fresh = if let Some((prior_owner, effect)) = effects.get(&id) {
            let Effect::List(prior) = effect else {
                return Err(invalid());
            };
            if *prior_owner != owner
                || prior.capacity != op.capacity
                || prior.contextual != op.contextual
                || prior.normal != op.normal
                || prior.control != op.control
                || prior.inputs.len() != op.count
                || prior.inputs.iter().enumerate().any(|(part, input)| {
                    Some(input.point) != sequence.items[part]
                        || input.plan != plan(part)
                        || input.source != source(part)
                })
            {
                return Err(invalid());
            }
            false
        } else {
            if effects.len() >= limit || op.count * 4 > *parts {
                return Err(budget());
            }
            true
        };
        let (_, Effect::List(observed)) = effects.entry(id).or_insert_with(|| {
            (
                owner,
                Effect::List(Observed {
                    capacity: op.capacity,
                    contextual: op.contextual,
                    normal: op.normal,
                    control: op.control,
                    inputs: sequence
                        .items
                        .iter()
                        .enumerate()
                        .map(|(part, point)| Input {
                            point: point.unwrap(),
                            plan: plan(part),
                            source: source(part),
                            projected: false,
                            converted: false,
                        })
                        .collect(),
                    constructed: false,
                    result: false,
                }),
            )
        }) else {
            unreachable!()
        };
        match port {
            Port::Projection { step, .. } => observed.inputs[step].projected = true,
            Port::Conversion { part, .. } => observed.inputs[part].converted = true,
            Port::Operation(_) => observed.constructed = true,
            Port::Normal(_) => observed.result = true,
            _ => unreachable!(),
        }
        if fresh {
            *parts -= op.count * 4;
        }
        Ok(())
    }
}
