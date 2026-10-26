use super::*;
use crate::check::dependencies::emissions::Projection;

mod fields;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct Emission {
    pub(in super::super) owner: usize,
    pub(in super::super) statement: PointId,
    pub(in super::super) input: PointId,
    pub(in super::super) target: hir::BlockId,
    pub(in super::super) reached: bool,
    pub(in super::super) normal: bool,
    pub(in super::super) control: Option<bool>,
}

impl Checker {
    pub(in super::super) fn scalar_emission_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        mut observations: Observations<'a, 'd>,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        for (&statement, source) in &reports.effects {
            let (owner, effect) = source;
            let Effect::Emission(observed) = effect else {
                continue;
            };
            self.validate_emission_report(reports, statement, *owner, observed, span)?;
            if !self.flow.spend(observed.targets.len() * 2 + 3) {
                return Err(budget(span));
            }
            if observed.result != graph.nodes.contains_key(&Port::Normal(statement))
                || observed.targets.iter().zip(&observed.initialized).any(
                    |(target, initialized)| {
                        *initialized != graph.nodes.contains_key(&Port::Emission(target.id))
                    },
                )
            {
                return Err(invalid(span));
            }
            let [target] = observed.targets.as_slice() else {
                continue;
            };
            if target.field.is_some() {
                self.scalar_field_emission(
                    graph,
                    &mut observations,
                    (statement, source),
                    span,
                    &mut parts,
                )?;
                continue;
            }
            if observed.composed.is_some()
                || target.projection != Projection::Value
                || target.field.is_some()
                || target.alias.is_some()
                || target.storage.is_some()
            {
                continue;
            }
            let port = Port::Emission(target.id);
            for edge in [
                Edge::new(Port::Normal(observed.input), port, Route::Next),
                Edge::new(port, Port::Normal(statement), Route::Next),
            ] {
                self.scalar_scope_edges(graph, *owner, edge.to, Family::Emission, &[edge], span)?;
            }
            if !observed.initialized[0] {
                continue;
            }
            let control = super::super::super::controls::dependency(
                statement,
                *owner,
                observed.control,
                &self.points,
                &self.branch_inputs,
                &mut self.flow,
            )
            .map_err(|error| error.diagnostic("scalar-emission", span))?;
            parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
            if observations
                .emissions
                .insert(
                    target.id,
                    Emission {
                        owner: *owner,
                        statement,
                        input: observed.input,
                        target: target.block,
                        reached: graph.nodes.contains_key(&Port::Normal(observed.input)),
                        normal: observed.result,
                        control,
                    },
                )
                .is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok((observations, parts))
    }
}

#[cfg(test)]
mod tests;
