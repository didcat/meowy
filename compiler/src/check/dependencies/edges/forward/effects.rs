use super::{entries::Reports, *};
use crate::check::dependencies::{OperationKind, PathStep};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Storage {
        kind: OperationKind,
        local: crate::hir::LocalId,
        storage: crate::hir::LocalId,
        input: Option<PointId>,
        control: bool,
    },
    Path {
        local: crate::hir::LocalId,
        storage: crate::hir::LocalId,
        steps: Vec<PathStep>,
        input: PointId,
        control: bool,
    },
    Unknown,
}

pub(crate) type Effects = BTreeMap<PointId, (usize, Effect)>;

impl Checker {
    pub(super) fn operation_effects(&mut self, reports: &Reports, span: Span) -> Result<Effects> {
        self.operation_effects_limited(reports, span, MAX_EDGES, MAX_EDGES)
    }

    pub(self) fn operation_effects_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut steps: usize,
    ) -> Result<Effects> {
        let budget = || Diagnostic::unsupported("proof operation-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof operation-effect owner mismatch", span);
        if !self.flow.spend(reports.entries.len() + 1) {
            return Err(budget());
        }
        let mut effects = BTreeMap::new();
        for (&owner, (_, walk)) in &reports.entries {
            for &port in &walk.ports {
                if !self.flow.spend(1) {
                    return Err(budget());
                }
                let Port::Operation(id) = port else {
                    continue;
                };
                if !self.flow.spend(
                    reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                        + self.operations.len().checked_ilog2().unwrap_or(0) as usize
                        + self.paths.len().checked_ilog2().unwrap_or(0) as usize
                        + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 6,
                ) {
                    return Err(budget());
                }
                if reports.index.operations.get(&id) != Some(&owner) {
                    return Err(invalid());
                }
                if effects.contains_key(&id) {
                    continue;
                }
                if effects.len() >= limit {
                    return Err(budget());
                }
                let effect = if let Some(op) = self.operations.get(&id) {
                    if op.owner != owner {
                        return Err(invalid());
                    }
                    Effect::Storage {
                        kind: op.kind,
                        local: op.local,
                        storage: op.storage,
                        input: op.input,
                        control: op.control,
                    }
                } else if let Some(op) = self.paths.get(&id) {
                    if op.owner != owner {
                        return Err(invalid());
                    }
                    if op.steps.is_empty() {
                        return Err(Diagnostic::unsupported(
                            "proof path-effect identity mismatch",
                            span,
                        ));
                    }
                    if op.steps.len() > crate::list::MAX_WRITE_PATH
                        || op.steps.len() > steps
                        || !self.flow.spend(op.steps.len() + 1)
                    {
                        return Err(budget());
                    }
                    steps -= op.steps.len();
                    Effect::Path {
                        local: op.local,
                        storage: op.storage,
                        steps: op.steps.clone(),
                        input: op.input,
                        control: op.control,
                    }
                } else {
                    Effect::Unknown
                };
                effects.insert(id, (owner, effect));
            }
        }
        Ok(effects)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod paths;
