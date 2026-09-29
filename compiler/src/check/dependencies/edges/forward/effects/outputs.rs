use super::*;
use crate::check::dependencies::FormatInput;

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

impl Checker {
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
            .spend(self.outputs.len().checked_ilog2().unwrap_or(0) as usize + 10)
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
            Kind::Projection { input, .. }
            | Kind::Part {
                input: Some(input), ..
            } => Some(input),
            _ => None,
        };
        if let Some(input) = input
            && !self.points.get(input.point).is_some_and(|child| {
                child.complete
                    && child.owner == owner
                    && child.parent == Some(id)
                    && child.block == point.block
                    && matches!(child.kind, PointKind::Expr | PointKind::And | PointKind::Or)
            })
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
