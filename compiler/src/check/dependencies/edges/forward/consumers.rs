use super::{effects::Effect, entries::Reports, *};
use crate::{
    check::dependencies::{
        BinaryClass,
        bodies::{Layout, completion::Shape},
    },
    hir,
};

mod blocks;
pub(super) mod dispatch;
mod emissions;
pub(super) mod field_results;
mod fields;
mod grouped;
mod index;
mod primary;

pub(super) mod scalars;

#[cfg(test)]
mod outputs;

#[cfg(test)]
mod lists;

#[cfg(test)]
mod receivers;

pub(crate) type Index = BTreeMap<PointId, (usize, hir::BlockId)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Slot {
    pub(crate) block: hir::BlockId,
    pub(crate) index: usize,
}

pub(crate) type Uses = BTreeMap<Port, (usize, Slot)>;

impl Checker {
    pub(super) fn slot_uses(&mut self, reports: &Reports, span: Span) -> Result<Uses> {
        self.slot_uses_limited(reports, span, MAX_EDGES)
    }

    pub(self) fn slot_uses_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
    ) -> Result<Uses> {
        let budget = || Diagnostic::unsupported("proof slot-use budget exhausted", span);
        let limit = limit
            .checked_sub(reports.effects.len())
            .and_then(|room| room.checked_sub(reports.blocks.len()))
            .and_then(|room| room.checked_sub(reports.results.len()))
            .and_then(|room| room.checked_sub(reports.consumers.len()))
            .and_then(|room| room.checked_sub(reports.eligible.len()))
            .and_then(|room| room.checked_sub(reports.initializers.len()))
            .ok_or_else(budget)?;
        if !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut uses = Uses::new();
        for (&id, (owner, effect)) in &reports.effects {
            if matches!(effect, Effect::Emission(_)) {
                self.emission_slot_uses(reports, (id, *owner), effect, &mut uses, limit, span)?;
                continue;
            }
            if let Effect::Output(output) = effect {
                self.validate_output_report(reports, id, *owner, output, span)?;
                if !self.flow.spend(output.parts.len() * 2 + 1) {
                    return Err(budget());
                }
                for (&part, observed) in &output.parts {
                    if !observed.projection {
                        continue;
                    }
                    let input = observed.input.ok_or_else(|| {
                        Diagnostic::unsupported("proof output-effect identity mismatch", span)
                    })?;
                    let slot = if let Some(slot) =
                        self.primary_slot(reports, input.point, *owner, span)?
                    {
                        Some(slot)
                    } else if let Some(Shape::Scalar(ty)) = input.source {
                        if let Some(slot) =
                            self.dispatch_primary_slot(reports, input.point, *owner, ty, span)?
                        {
                            Some(slot)
                        } else {
                            self.receiver_primary_slot(reports, input.point, *owner, ty, span)?
                        }
                    } else {
                        None
                    };
                    if let Some(slot) = slot {
                        let port = Port::Projection {
                            point: id,
                            step: part,
                        };
                        self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
                    }
                }
                continue;
            }
            if let Effect::List(list) = effect {
                self.validate_list_report(reports, id, *owner, list, span)?;
                if !self.flow.spend(list.inputs.len() * 2 + 1) {
                    return Err(budget());
                }
                for (step, input) in list.inputs.iter().enumerate() {
                    if !input.projected {
                        continue;
                    }
                    let slot = if let Some(slot) =
                        self.primary_slot(reports, input.point, *owner, span)?
                    {
                        Some(slot)
                    } else if let Some(Shape::Scalar(ty)) = input.source {
                        if let Some(slot) =
                            self.dispatch_primary_slot(reports, input.point, *owner, ty, span)?
                        {
                            Some(slot)
                        } else {
                            self.receiver_primary_slot(reports, input.point, *owner, ty, span)?
                        }
                    } else {
                        None
                    };
                    if let Some(slot) = slot {
                        let port = Port::Projection { point: id, step };
                        self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
                    }
                }
                continue;
            }
            let mut slots = [None; 3];
            slots[0] = self
                .field_slot(reports, id, *owner, effect, span)?
                .map(|slot| (Port::Operation(id), slot));
            for (step, input) in self
                .primary_effect_inputs(reports, id, *owner, effect, span)?
                .into_iter()
                .enumerate()
            {
                if let Some(input) = input {
                    let slot =
                        if let Some(slot) = self.primary_slot(reports, input, *owner, span)? {
                            Some(slot)
                        } else if let Effect::Unary(op) = effect {
                            if let Some(slot) =
                                self.dispatch_primary_slot(reports, input, *owner, op.ty, span)?
                            {
                                Some(slot)
                            } else {
                                self.receiver_primary_slot(reports, input, *owner, op.ty, span)?
                            }
                        } else if let Effect::Binary(op) = effect
                            && let BinaryClass::Scalar(ty) = op.types.inputs[step]
                        {
                            if let Some(slot) =
                                self.dispatch_primary_slot(reports, input, *owner, ty, span)?
                            {
                                Some(slot)
                            } else {
                                self.receiver_primary_slot(reports, input, *owner, ty, span)?
                            }
                        } else if let Effect::Coercion(op) = effect
                            && let Some(Shape::Scalar(ty)) = op.source
                        {
                            if let Some(slot) =
                                self.dispatch_primary_slot(reports, input, *owner, ty, span)?
                            {
                                Some(slot)
                            } else {
                                self.receiver_primary_slot(reports, input, *owner, ty, span)?
                            }
                        } else if let Effect::Binary(op) = effect
                            && let BinaryClass::SharedScalar(ty) = op.types.inputs[step]
                        {
                            if let Some(slot) = self.dispatch_primary_shape(
                                reports,
                                input,
                                *owner,
                                Shape::SharedScalar(ty),
                                span,
                            )? {
                                Some(slot)
                            } else {
                                self.receiver_primary_shape(
                                    reports,
                                    input,
                                    *owner,
                                    Shape::SharedScalar(ty),
                                    span,
                                )?
                            }
                        } else if let Effect::Coercion(op) = effect
                            && let Some(source @ Shape::SharedScalar(_)) = op.source
                        {
                            if let Some(slot) =
                                self.dispatch_primary_shape(reports, input, *owner, source, span)?
                            {
                                Some(slot)
                            } else {
                                self.receiver_primary_shape(reports, input, *owner, source, span)?
                            }
                        } else {
                            None
                        };
                    slots[step + 1] = slot.map(|slot| (Port::Projection { point: id, step }, slot));
                }
            }
            for (port, slot) in slots.into_iter().flatten() {
                self.record_slot_use(&mut uses, port, (*owner, slot), limit, span)?;
            }
        }
        Ok(uses)
    }

    pub(super) fn record_slot_use(
        &mut self,
        uses: &mut Uses,
        port: Port,
        value: (usize, Slot),
        limit: usize,
        span: Span,
    ) -> Result<()> {
        if uses.len() >= limit
            || !self
                .flow
                .spend(uses.len().checked_ilog2().unwrap_or(0) as usize + 2)
        {
            return Err(Diagnostic::unsupported(
                "proof slot-use budget exhausted",
                span,
            ));
        }
        uses.insert(port, value);
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits;
