mod binaries;
mod calls;
mod coercions;
mod derefs;
mod fields;
mod heaps;
mod indices;
mod methods;
mod narrowing;
mod outputs;
mod reads;
mod scalars;
mod unary;

use super::{entries::Reports, *};
use crate::check::dependencies::{OperationKind, Origins, PathStep, references::MAX_ROOTS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Storage {
        kind: OperationKind,
        local: crate::hir::LocalId,
        storage: crate::hir::LocalId,
        input: Option<PointId>,
        control: bool,
    },
    Read {
        local: crate::hir::LocalId,
        storage: crate::hir::LocalId,
        normal: bool,
        control: bool,
    },
    Deref {
        input: PointId,
        mode: crate::hir::ReferenceMode,
        normal: bool,
        control: bool,
    },
    Field {
        input: PointId,
        index: usize,
        load: bool,
        normal: bool,
        control: bool,
    },
    Path {
        local: crate::hir::LocalId,
        storage: crate::hir::LocalId,
        steps: Vec<PathStep>,
        input: PointId,
        control: bool,
    },
    Indirect {
        target: PointId,
        input: PointId,
        origins: Origins,
        control: bool,
    },
    Call {
        site: crate::hir::CallId,
        function: crate::hir::FunctionId,
        args: Vec<PointId>,
        may_return: bool,
        control: bool,
    },
    Output(outputs::Observed),
    Index(indices::Observed),
    Method(methods::Observed),
    Unary(unary::Observed),
    Binary(binaries::Observed),
    Scalar(scalars::Observed),
    Heap(heaps::Observed),
    Narrowing(narrowing::Observed),
    Coercion(coercions::Observed),
    Unknown,
}

pub(crate) type Effects = BTreeMap<PointId, (usize, Effect)>;

impl Checker {
    pub(super) fn operation_effects(&mut self, reports: &Reports, span: Span) -> Result<Effects> {
        self.operation_effects_limited(reports, span, MAX_EDGES, MAX_EDGES, MAX_EDGES)
    }

    pub(self) fn operation_effects_limited(
        &mut self,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut parts: usize,
        mut roots: usize,
    ) -> Result<Effects> {
        let budget = || Diagnostic::unsupported("proof operation-effect budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof operation-effect owner mismatch", span);
        if !self.flow.spend(reports.entries.len() + 1) {
            return Err(budget());
        }
        let calls = self.call_effect_index(span)?;
        let mut effects = BTreeMap::new();
        for (&owner, (_, walk)) in &reports.entries {
            for &port in &walk.ports {
                if !self.flow.spend(1) {
                    return Err(budget());
                }
                if let Some(stage) = self.coercion_effect_stage(reports, owner, port, span)? {
                    self.record_coercion_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.narrowing_effect_stage(reports, owner, port, span)? {
                    self.record_narrowing_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.heap_effect_stage(reports, owner, port, span)? {
                    self.record_heap_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.scalar_effect_stage(reports, owner, port, span)? {
                    self.record_scalar_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.binary_effect_stage(reports, owner, port, span)? {
                    self.record_binary_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.unary_effect_stage(reports, owner, port, span)? {
                    self.record_unary_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.index_effect_stage(reports, owner, port, span)? {
                    self.record_index_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.method_effect_stage(reports, owner, port, span)? {
                    self.record_method_effect(stage, &mut effects, limit, span)?;
                    continue;
                }
                if let Some(stage) = self.output_effect_stage(owner, port, span)? {
                    if matches!(stage.kind, outputs::Kind::Finish) {
                        if !self.flow.spend(
                            reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                                + 1,
                        ) {
                            return Err(budget());
                        }
                        if reports.index.operations.get(&stage.point) != Some(&owner) {
                            return Err(invalid());
                        }
                    }
                    self.record_output_effect(stage, &mut effects, limit, &mut parts, span)?;
                    continue;
                }
                let Port::Operation(id) = port else {
                    continue;
                };
                if !self.flow.spend(
                    reports.index.operations.len().checked_ilog2().unwrap_or(0) as usize
                        + self.operations.len().checked_ilog2().unwrap_or(0) as usize
                        + self.local_reads.len().checked_ilog2().unwrap_or(0) as usize
                        + self.derefs.len().checked_ilog2().unwrap_or(0) as usize
                        + self.fields.len().checked_ilog2().unwrap_or(0) as usize
                        + self.paths.len().checked_ilog2().unwrap_or(0) as usize
                        + self.stores.len().checked_ilog2().unwrap_or(0) as usize
                        + calls.len().checked_ilog2().unwrap_or(0) as usize
                        + self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                        + effects.len().checked_ilog2().unwrap_or(0) as usize * 2
                        + 9,
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
                } else if self.local_reads.contains_key(&id) {
                    self.read_effect(reports, id, owner, span)?
                } else if self.derefs.contains_key(&id) {
                    self.deref_effect(reports, id, owner, span)?
                } else if self.fields.contains_key(&id) {
                    self.field_effect(reports, id, owner, span)?
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
                        || op.steps.len() > parts
                        || !self.flow.spend(op.steps.len() + 1)
                    {
                        return Err(budget());
                    }
                    parts -= op.steps.len();
                    Effect::Path {
                        local: op.local,
                        storage: op.storage,
                        steps: op.steps.clone(),
                        input: op.input,
                        control: op.control,
                    }
                } else if let Some(op) = self.stores.get(&id) {
                    if op.owner != owner {
                        return Err(invalid());
                    }
                    if op.origins.roots.len() > MAX_ROOTS
                        || op.origins.roots.len() > roots
                        || !self.flow.spend(op.origins.roots.len() + 1)
                    {
                        return Err(budget());
                    }
                    roots -= op.origins.roots.len();
                    Effect::Indirect {
                        target: op.target,
                        input: op.input,
                        origins: op.origins.clone(),
                        control: op.control,
                    }
                } else if let Some(site) = calls.get(&id) {
                    if self.invocations[site].owner != owner {
                        return Err(invalid());
                    }
                    self.call_effect(reports, *site, &mut parts, span)?
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

#[cfg(test)]
mod stores;
