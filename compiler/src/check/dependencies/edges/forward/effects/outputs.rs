use super::*;
use crate::check::dependencies::FormatInput;

mod edges;
mod validation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Prefix,
    Projection {
        part: usize,
        input: FormatInput,
    },
    Part {
        part: usize,
        input: Option<FormatInput>,
    },
    Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stage {
    pub(super) point: PointId,
    pub(super) owner: usize,
    pub(super) panic: bool,
    pub(super) control: bool,
    pub(super) total: usize,
    pub(super) stopped: Option<usize>,
    pub(super) kind: Kind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Observed {
    pub(crate) panic: bool,
    pub(crate) control: bool,
    pub(crate) total: usize,
    pub(crate) stopped: Option<usize>,
    pub(crate) prefix: bool,
    pub(crate) terminal: bool,
    pub(crate) parts: BTreeMap<usize, Part>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Part {
    pub(crate) input: Option<FormatInput>,
    pub(crate) projection: bool,
    pub(crate) output: bool,
}

impl Checker {
    pub(super) fn record_output_effect(
        &mut self,
        stage: Stage,
        effects: &mut Effects,
        limit: usize,
        parts: &mut usize,
        span: Span,
    ) -> Result<()> {
        let invalid = || Diagnostic::unsupported("proof output-effect identity mismatch", span);
        let budget = || Diagnostic::unsupported("proof output-effect budget exhausted", span);
        if !self
            .flow
            .spend(effects.len().checked_ilog2().unwrap_or(0) as usize * 2 + 9)
        {
            return Err(budget());
        }
        let part = match stage.kind {
            Kind::Projection { part, input } => Some((part, Some(input))),
            Kind::Part { part, input } => Some((part, input)),
            _ => None,
        };
        if let Some((part, Some(input))) = part
            && !input.valid_source(part, stage.stopped)
        {
            return Err(invalid());
        }
        let mut fresh = part.is_some();
        if let Some((owner, effect)) = effects.get(&stage.point) {
            let Effect::Output(output) = effect else {
                return Err(invalid());
            };
            if *owner != stage.owner
                || output.panic != stage.panic
                || output.control != stage.control
                || output.total != stage.total
                || output.stopped != stage.stopped
            {
                return Err(invalid());
            }
            if let Some((id, input)) = part {
                if !self
                    .flow
                    .spend(output.parts.len().checked_ilog2().unwrap_or(0) as usize * 2 + 2)
                {
                    return Err(budget());
                }
                if let Some(prior) = output.parts.get(&id) {
                    if prior.input != input {
                        return Err(invalid());
                    }
                    fresh = false;
                }
            }
        } else if effects.len() >= limit {
            return Err(budget());
        }
        if fresh && *parts == 0 {
            return Err(budget());
        }
        let (_, Effect::Output(output)) = effects.entry(stage.point).or_insert_with(|| {
            (
                stage.owner,
                Effect::Output(Observed {
                    panic: stage.panic,
                    control: stage.control,
                    total: stage.total,
                    stopped: stage.stopped,
                    prefix: false,
                    terminal: false,
                    parts: BTreeMap::new(),
                }),
            )
        }) else {
            unreachable!()
        };
        match stage.kind {
            Kind::Prefix => output.prefix = true,
            Kind::Finish => output.terminal = true,
            Kind::Projection { part, input } => {
                output
                    .parts
                    .entry(part)
                    .or_insert(Part {
                        input: Some(input),
                        projection: false,
                        output: false,
                    })
                    .projection = true
            }
            Kind::Part { part, input } => {
                output
                    .parts
                    .entry(part)
                    .or_insert(Part {
                        input,
                        projection: false,
                        output: false,
                    })
                    .output = true
            }
        }
        if fresh {
            *parts -= 1;
        }
        Ok(())
    }

    pub(super) fn output_effect_stage(
        &mut self,
        owner: usize,
        port: Port,
        span: Span,
    ) -> Result<Option<Stage>> {
        let invalid = || Diagnostic::unsupported("proof output-effect identity mismatch", span);
        let budget = || Diagnostic::unsupported("proof output-effect budget exhausted", span);
        let (id, reserved) = match port {
            Port::Prefix(id) | Port::Output { point: id, .. } => (id, true),
            Port::Operation(id) | Port::Projection { point: id, .. } => (id, false),
            _ => return Ok(None),
        };
        if !self
            .flow
            .spend(self.outputs.len().checked_ilog2().unwrap_or(0) as usize + 13)
        {
            return Err(budget());
        }
        let Some(op) = self.outputs.get(&id) else {
            return if reserved { Err(invalid()) } else { Ok(None) };
        };
        if op.parts.len() > crate::check::dependencies::sequences::MAX_ITEMS {
            return Err(budget());
        }
        let point = self.points.get(id).ok_or_else(invalid)?;
        if op.owner != owner
            || point.owner != owner
            || point.kind != PointKind::Expr
            || !point.complete
            || point.span != op.span
            || op
                .stopped
                .is_some_and(|part| op.parts.get(part).is_none_or(Option::is_none))
        {
            return Err(invalid());
        }
        let kind = match port {
            Port::Prefix(_) if op.panic => Kind::Prefix,
            Port::Operation(_) if op.stopped.is_none() => Kind::Finish,
            Port::Projection { step, .. } if op.stopped.is_none_or(|stop| step <= stop) => {
                let input = op
                    .parts
                    .get(step)
                    .and_then(|input| *input)
                    .filter(|input| input.primary)
                    .ok_or_else(invalid)?;
                Kind::Projection { part: step, input }
            }
            Port::Output { part, .. } if op.stopped.is_none_or(|stop| part < stop) => Kind::Part {
                part,
                input: *op.parts.get(part).ok_or_else(invalid)?,
            },
            _ => return Err(invalid()),
        };
        let input = match kind {
            Kind::Projection { part, input }
            | Kind::Part {
                part,
                input: Some(input),
            } => Some((part, input)),
            _ => None,
        };
        if let Some((part, input)) = input
            && (!input.valid_source(part, op.stopped)
                || !self.points.get(input.point).is_some_and(|child| {
                    child.complete
                        && child.owner == owner
                        && child.parent == Some(id)
                        && child.block == point.block
                        && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
                }))
        {
            return Err(invalid());
        }
        Ok(Some(Stage {
            point: id,
            owner,
            panic: op.panic,
            control: op.control,
            total: op.parts.len(),
            stopped: op.stopped,
            kind,
        }))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reports;

#[cfg(test)]
mod sources;
