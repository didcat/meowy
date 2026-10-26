use super::*;
use crate::check::dependencies::bodies::{Layout, completion::Shape};

impl Checker {
    pub(super) fn scalar_field_emission(
        &mut self,
        graph: &graph::Graph<'_>,
        observations: &mut Observations<'_, '_>,
        source: (PointId, &(usize, Effect)),
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        let (statement, source) = source;
        let &(owner, Effect::Emission(ref observed)) = source else {
            return Err(invalid(span));
        };
        let [target] = observed.targets.as_slice() else {
            return Err(invalid(span));
        };
        if !self.flow.spend(3) {
            return Err(budget(span));
        }
        if observed.composed.is_some()
            || target.projection != Projection::Value
            || !observed.initialized[0]
            || !observations.scopes.contains_key(&target.block)
        {
            return Ok(());
        }
        let body = self
            .bodies
            .get(&target.block)
            .ok_or_else(|| invalid(span))?;
        if !matches!(body.completion.result, Shape::Record { .. }) {
            return Ok(());
        }
        let Layout::Slots(slots) = &body.layout else {
            return Err(invalid(span));
        };
        if !self.flow.spend(slots.len() + 2) {
            return Err(budget(span));
        }
        if body.owner != owner {
            return Err(invalid(span));
        }
        let Some(slot) = slots.iter().find(|slot| slot.field == target.field) else {
            return Ok(());
        };
        if slot.mutable || !matches!(slot.shape, Shape::Scalar(_)) {
            return Err(invalid(span));
        }
        let port = Port::Emission(target.id);
        self.scalar_scope_edges(
            graph,
            owner,
            Port::Normal(statement),
            Family::Emission,
            &[Edge::new(port, Port::Normal(statement), Route::Next)],
            span,
        )?;
        let control = super::super::super::super::controls::dependency(
            statement,
            owner,
            observed.control,
            &self.points,
            &self.branch_inputs,
            &mut self.flow,
        )
        .map_err(|error| error.diagnostic("scalar-field-emission", span))?;
        let Some(control) = control else {
            return Ok(());
        };
        self.scalar_scope_edges(
            graph,
            owner,
            port,
            Family::Emission,
            &[Edge::new(Port::Normal(observed.input), port, Route::Next)],
            span,
        )?;
        if !self.flow.spend(2) {
            return Err(budget(span));
        }
        *parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
        if observations
            .forwarded
            .insert(
                port,
                Forward {
                    owner,
                    input: Port::Normal(observed.input),
                    control,
                },
            )
            .is_some()
        {
            return Err(invalid(span));
        }
        Ok(())
    }
}
