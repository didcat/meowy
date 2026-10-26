use super::*;
use crate::check::dependencies::{BinaryClass, bodies::completion::Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super::super) struct Primary {
    pub(in super::super::super) owner: usize,
    pub(in super::super::super) input: PointId,
    pub(in super::super::super) block: hir::BlockId,
    pub(in super::super::super) control: bool,
    pub(in super::super::super) stored: bool,
}

impl Checker {
    pub(super) fn scalar_primary_observation(
        &mut self,
        reports: &Reports,
        graph: &graph::Graph<'_>,
        source: (PointId, &(usize, Effect)),
        observations: &mut Observations<'_, '_>,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let (point, (owner, effect)) = source;
        let (family, kinds, projected, operation, result, control) = match effect {
            Effect::Coercion(op) => {
                self.forward_coercion_input(reports, point, *owner, span)?;
                let kind = match op.source {
                    Some(Shape::Scalar(kind)) => Some(kind),
                    _ => None,
                };
                (
                    Family::Coercion,
                    [kind, None],
                    [op.projected, false],
                    op.operation,
                    op.result,
                    op.control,
                )
            }
            Effect::Unary(op) => (
                Family::Unary,
                [Some(op.ty), None],
                [op.projected, false],
                op.operation,
                op.result,
                op.control,
            ),
            Effect::Binary(op) => {
                let kinds = op.types.inputs.map(|kind| match kind {
                    BinaryClass::Scalar(kind) => Some(kind),
                    _ => None,
                });
                (
                    Family::Binary,
                    kinds,
                    op.projected,
                    op.operation,
                    op.result,
                    op.control,
                )
            }
            _ => return Err(invalid(span)),
        };
        let inputs = self.primary_effect_inputs(reports, point, *owner, effect, span)?;
        for (stage, seen) in [
            (Port::Projection { point, step: 0 }, projected[0]),
            (Port::Projection { point, step: 1 }, projected[1]),
            (Port::Operation(point), operation),
            (Port::Normal(point), result),
        ] {
            if !self.flow.spend(1) {
                return Err(budget(span));
            }
            if seen != graph.nodes.contains_key(&stage) {
                return Err(invalid(span));
            }
        }
        for (step, (input, kind)) in inputs.into_iter().zip(kinds).enumerate() {
            let (Some(input), Some(kind)) = (input, kind) else {
                continue;
            };
            let port = Port::Projection { point, step };
            let Some(slot) = self.primary_slot(reports, input, *owner, span)? else {
                continue;
            };
            if !self
                .flow
                .spend(reports.slot_uses.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(budget(span));
            }
            if reports.slot_uses.get(&port) != Some(&(*owner, slot)) || slot.index != 0 {
                return Err(invalid(span));
            }
            let Some(scalar) = self.scalar_scope_type(slot.block, span)? else {
                continue;
            };
            if scalar.nullable || scalar.kind != kind {
                return Err(invalid(span));
            }
            let parent = self.bodies[&slot.block]
                .parent
                .ok_or_else(|| invalid(span))?;
            let mut current = input;
            let mut hops = 0;
            let mut stored = false;
            let mut wrapped = false;
            while current != parent {
                if hops >= MAX_GROUPS
                    || !self
                        .flow
                        .spend(self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize + 1)
                {
                    return Err(budget(span));
                }
                if let Some(group) = self.group_inputs.get(&current).copied() {
                    current = self.qualified_group_input(current, *owner, group, span)?;
                } else {
                    if !self
                        .flow
                        .spend(reports.effects.len().checked_ilog2().unwrap_or(0) as usize + 3)
                    {
                        return Err(budget(span));
                    }
                    if let Some((_, Effect::Read { local, .. })) = reports.effects.get(&current) {
                        let Some(source) =
                            self.read_initializer_input(reports, current, *owner, span)?
                        else {
                            break;
                        };
                        let Some(location) = observations.definitions.declarations.get(local)
                        else {
                            break;
                        };
                        let versions::Definition::Bind(bind) = location.version.definition else {
                            break;
                        };
                        let Some(record) = observations.definitions.records.get(&bind) else {
                            break;
                        };
                        if record.block != slot.block
                            || record.storage.input != source
                            || record.storage.version != location.version
                            || location.version.owner != *owner
                            || location.version.local != *local
                        {
                            return Err(invalid(span));
                        }
                        stored = true;
                        break;
                    }
                    wrapped = true;
                    if let Some(next) =
                        self.forward_coercion_input(reports, current, *owner, span)?
                    {
                        current = next;
                    } else if let Some(next) =
                        self.unchanged_ascription_input(reports, current, *owner, span)?
                    {
                        current = next;
                    } else if let Some(next) =
                        self.unchanged_narrowing_input(reports, current, *owner, span)?
                    {
                        current = next;
                    } else {
                        break;
                    }
                }
                hops += 1;
            }
            if current != parent && !stored {
                continue;
            }
            let stored = stored || wrapped;
            self.scalar_scope_edges(
                graph,
                *owner,
                port,
                family,
                &[Edge::new(Port::Normal(input), port, Route::Next)],
                span,
            )?;
            if !self.flow.spend(4) {
                return Err(budget(span));
            }
            if observations.sources.contains_key(&port)
                || observations.forwarded.contains_key(&port)
                || observations.logical.contains_key(&port)
            {
                return Err(invalid(span));
            }
            *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if observations
                .primaries
                .insert(
                    port,
                    Primary {
                        owner: *owner,
                        input,
                        block: slot.block,
                        control,
                        stored,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
