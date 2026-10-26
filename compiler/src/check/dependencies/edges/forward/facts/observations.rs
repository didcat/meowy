use super::super::effects::Effect;
use super::*;

mod borrows;
mod call_results;
mod calls;
mod completion;
mod emissions;
mod logical;
mod reads;
mod scopes;
mod slots;
mod wrappers;
pub(super) use emissions::Emission;
pub(super) use logical::Logical;
pub(super) use slots::Slot;
pub(super) use wrappers::Forward;
pub(super) use wrappers::Primary;

#[derive(Debug)]
pub(super) struct Observations<'a, 'd> {
    pub(super) definitions: &'d definitions::Definitions,
    pub(super) sources: HashMap<Port, &'a (usize, Effect)>,
    pub(super) declared: HashMap<hir::LocalId, domain::Domain>,
    pub(super) forwarded: HashMap<Port, Forward>,
    pub(super) primaries: HashMap<Port, Primary>,
    pub(super) guards: guards::Inputs,
    pub(super) logical: HashMap<Port, Logical>,
    pub(super) calls: Option<&'d super::super::arguments::Mutations>,
    pub(super) call_controls: HashMap<Port, Option<bool>>,
    pub(super) scopes: HashMap<hir::BlockId, scopes::Scope>,
    pub(super) emissions: HashMap<hir::EmitId, Emission>,
    pub(super) completion: completion::Completion,
    pub(super) slots: HashMap<hir::BlockId, Slot>,
}

impl<'a, 'd> Observations<'a, 'd> {
    pub(super) fn new(definitions: &'d definitions::Definitions) -> Self {
        Self {
            definitions,
            sources: HashMap::new(),
            declared: HashMap::new(),
            forwarded: HashMap::new(),
            primaries: HashMap::new(),
            guards: guards::Inputs::new(),
            logical: HashMap::new(),
            calls: None,
            call_controls: HashMap::new(),
            scopes: HashMap::new(),
            emissions: HashMap::new(),
            completion: completion::Completion::default(),
            slots: HashMap::new(),
        }
    }
}

impl Checker {
    pub(super) fn scalar_observations<'a, 'd>(
        &mut self,
        reports: &'a Reports,
        graph: &graph::Graph<'_>,
        definitions: &'d definitions::Definitions,
        span: Span,
        parts: usize,
    ) -> Result<(Observations<'a, 'd>, usize)> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget(span));
        }
        let mut parts = parts.min(MAX_EDGES);
        let mut observations = Observations::new(definitions);
        for (&point, source) in &reports.effects {
            let (owner, Effect::Scalar(observed)) = source else {
                continue;
            };
            self.validate_scalar_report(reports, point, *owner, observed, span)?;
            for (port, reached) in [
                (Port::Operation(point), observed.operation),
                (Port::Normal(point), observed.result),
            ] {
                if !self.flow.spend(2) {
                    return Err(budget(span));
                }
                let node = graph.nodes.get(&port);
                if reached != node.is_some() {
                    return Err(invalid(span));
                }
                if let Some(node) = node {
                    if node.site != self.scalar_port_site(port, *owner, span)? {
                        return Err(invalid(span));
                    }
                    parts = parts.checked_sub(1).ok_or_else(|| budget(span))?;
                    observations.sources.insert(port, source);
                }
            }
        }
        let (mut observations, parts) =
            self.scalar_wrapper_observations(reports, graph, observations, span, parts)?;
        let (guards, parts) = self.scalar_guard_inputs(graph, span, parts)?;
        observations.guards = guards;
        let (logical, parts) =
            self.scalar_logical_observations(&observations.guards, graph, span, parts)?;
        observations.logical = logical;
        Ok((observations, parts))
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-observation budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-observation identity mismatch", span)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod histories;
