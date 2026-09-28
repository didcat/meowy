use super::*;
use crate::check::{
    SequenceSource,
    dependencies::{CoercionKind, MethodKind, PathStep, ProjectionStep},
};

impl Checker {
    pub(crate) fn stage_port_valid(
        &mut self,
        port: Port,
        owner: usize,
        span: Span,
    ) -> Result<bool> {
        let tables: &[usize] = match port {
            Port::Snapshot(_) => &[self.indices.len(), self.methods.len()],
            Port::Prefix(_) | Port::Output { .. } => &[self.outputs.len()],
            Port::Projection { .. } => &[
                self.outputs.len(),
                self.binaries.len(),
                self.unaries.len(),
                self.coercions.len(),
                self.fields.len(),
                self.indices.len(),
                self.methods.len(),
                self.projections.len(),
                self.list_inputs.len(),
                self.sequences.len(),
            ],
            Port::Conversion { .. } => &[
                self.projections.len(),
                self.list_inputs.len(),
                self.sequences.len(),
            ],
            Port::Address { .. } => &[
                self.place_borrows.len(),
                self.paths.len(),
                self.exclusives.len(),
                self.elements.len(),
                self.stores.len(),
            ],
            Port::Reserve { .. } => &[self.paths.len(), self.exclusives.len()],
            Port::Operation(id) => {
                return Ok(matches!(
                    self.points[id].kind,
                    PointKind::Expr | PointKind::Stmt
                ));
            }
            _ => return Ok(true),
        };
        let work: usize = tables
            .iter()
            .map(|size| size.checked_ilog2().unwrap_or(0) as usize + 1)
            .sum();
        if !self.flow.spend(work) {
            return Err(Diagnostic::unsupported(
                "proof graph-port budget exhausted",
                span,
            ));
        }
        Ok(match port {
            Port::Snapshot(id) => {
                self.indices
                    .get(&id)
                    .is_some_and(|op| op.owner == owner && op.access.is_some())
                    || self.methods.get(&id).is_some_and(|op| {
                        op.owner == owner && matches!(op.kind, MethodKind::Add { .. })
                    })
            }
            Port::Prefix(id) => self
                .outputs
                .get(&id)
                .is_some_and(|op| op.owner == owner && op.panic),
            Port::Output { point, part } => self
                .outputs
                .get(&point)
                .is_some_and(|op| op.owner == owner && part < op.parts.len()),
            Port::Projection { point, step } => {
                self.outputs.get(&point).is_some_and(|op| {
                    op.owner == owner
                        && op
                            .parts
                            .get(step)
                            .and_then(|input| *input)
                            .is_some_and(|input| input.primary)
                }) || self
                    .binaries
                    .get(&point)
                    .is_some_and(|op| op.owner == owner && op.plan.primary.get(step) == Some(&true))
                    || (step == 0
                        && (self
                            .unaries
                            .get(&point)
                            .is_some_and(|op| op.owner == owner && op.primary)
                            || self
                                .coercions
                                .get(&point)
                                .is_some_and(|op| op.owner == owner && op.primary)
                            || self
                                .fields
                                .get(&point)
                                .is_some_and(|op| op.owner == owner && op.load)
                            || self
                                .indices
                                .get(&point)
                                .is_some_and(|op| op.owner == owner && op.load)
                            || self
                                .methods
                                .get(&point)
                                .is_some_and(|op| op.owner == owner && op.load)))
                    || self
                        .projections
                        .get(&point)
                        .is_some_and(|op| op.owner == owner && step < op.steps.len())
                    || (self
                        .sequences
                        .get(&SequenceSource::Expr(point))
                        .is_some_and(|seq| seq.owner == owner)
                        && self
                            .list_inputs
                            .get(&point)
                            .and_then(|inputs| inputs.get(step))
                            .is_some_and(|input| input.primary))
            }
            Port::Conversion { point, part } => {
                self.projections.get(&point).is_some_and(|op| {
                    op.owner == owner
                        && matches!(
                            op.steps.get(part),
                            Some(ProjectionStep::Field { narrow: true, .. })
                        )
                }) || (self
                    .sequences
                    .get(&SequenceSource::Expr(point))
                    .is_some_and(|seq| seq.owner == owner)
                    && self
                        .list_inputs
                        .get(&point)
                        .and_then(|inputs| inputs.get(part))
                        .is_some_and(|input| input.kind == CoercionKind::Convert))
            }
            Port::Address { point, step } => {
                self.place_borrows
                    .get(&point)
                    .is_some_and(|op| op.owner == owner && step <= op.place.fields.len())
                    || self
                        .paths
                        .get(&point)
                        .is_some_and(|op| op.owner == owner && step <= op.steps.len())
                    || self
                        .exclusives
                        .get(&point)
                        .is_some_and(|op| op.owner == owner && step <= op.steps.len())
                    || (step == 0
                        && (self
                            .elements
                            .get(&point)
                            .is_some_and(|op| op.owner == owner && op.access.is_some())
                            || self.stores.get(&point).is_some_and(|op| op.owner == owner)))
            }
            Port::Reserve { point, step } => {
                self.paths.get(&point).is_some_and(|op| {
                    op.owner == owner && matches!(op.steps.get(step), Some(PathStep::Index { .. }))
                }) || self.exclusives.get(&point).is_some_and(|op| {
                    op.owner == owner && matches!(op.steps.get(step), Some(PathStep::Index { .. }))
                })
            }
            _ => unreachable!(),
        })
    }
}

#[cfg(test)]
mod tests;
