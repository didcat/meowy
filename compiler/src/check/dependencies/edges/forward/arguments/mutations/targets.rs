use super::*;
use crate::borrow::{Facts, Origin};

pub(in super::super) struct Targets<'a> {
    pub(in super::super) unknown: bool,
    pub(in super::super) origins: Vec<(&'a Origin, PointId)>,
}

pub(in super::super) struct Call<'a> {
    pub(in super::super) owner: usize,
    pub(in super::super) snapshot: Option<&'a crate::borrow::calls::Inputs>,
    pub(in super::super) inputs: Vec<(Input<'a>, Targets<'a>)>,
}

pub(in super::super) type Calls<'a> = BTreeMap<hir::CallId, Call<'a>>;

impl Checker {
    pub(in super::super) fn call_mutation_targets<'a>(
        &mut self,
        program: &Program,
        reports: &Reports,
        facts: &'a Facts,
        span: Span,
        parts: usize,
    ) -> Result<(Calls<'a>, usize)> {
        let budget = || Diagnostic::unsupported("proof call-mutation budget exhausted", span);
        let (view, parts) = self.call_snapshots(program, reports, facts, span, parts)?;
        let blocks = self.program_blocks(program, reports, span, parts)?;
        let parts = parts.checked_sub(blocks.len()).ok_or_else(budget)?;
        let (storage, mut parts) = self.mutation_storage(program, reports, &blocks, span, parts)?;
        let mut calls = Calls::new();
        for &site in view.calls.keys() {
            let (owner, function, count) = {
                let (call, inputs) = self.call_mutation_inputs(&view, site, span)?;
                (call.owner, call.function, inputs.len())
            };
            parts = parts.checked_sub(count + 1).ok_or_else(budget)?;
            if !self.flow.spend(
                calls.len().checked_ilog2().unwrap_or(0) as usize
                    + view.facts.call_inputs.len().checked_ilog2().unwrap_or(0) as usize
                    + view
                        .reports
                        .signatures
                        .functions
                        .len()
                        .checked_ilog2()
                        .unwrap_or(0) as usize
                    + count.saturating_mul(
                        self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                            + view.access.len().checked_ilog2().unwrap_or(0) as usize
                            + 3,
                    )
                    + 2,
            ) {
                return Err(budget());
            }
            let mut call = Call {
                owner,
                snapshot: facts.call_inputs.get(&site),
                inputs: Vec::with_capacity(count),
            };
            let params = &reports.signatures.functions[&function].params;
            for (position, &param) in params.iter().enumerate() {
                let point = self.invocations[&site].args[position];
                let access = view.access[&param];
                let state = call.snapshot.and_then(|snapshot| {
                    snapshot
                        .args
                        .get(position)
                        .map(|(_, state)| (state, position < snapshot.normal))
                });
                let mut targets = Targets {
                    unknown: true,
                    origins: Vec::new(),
                };
                if let Some((state, true)) = state {
                    match access {
                        Access::Value | Access::Shared(_) => targets.unknown = false,
                        Access::Unknown => (),
                        Access::Exclusive(kind) => {
                            targets.unknown = state.origins.is_empty()
                                || !state.bounds.is_empty()
                                || !state.active.is_empty();
                            let mut covered = crate::flow::FALSE;
                            for origin in &state.origins {
                                if let Some(id) = self.mutation_origin(
                                    (program, reports, &blocks, &storage),
                                    call.owner,
                                    kind,
                                    origin,
                                    span,
                                )? {
                                    parts = parts.checked_sub(1).ok_or_else(budget)?;
                                    targets.origins.push((origin, id));
                                    covered = self.flow.or(covered, origin.guard);
                                } else {
                                    targets.unknown = true;
                                }
                            }
                            let present = self.flow.and(state.present, state.proof);
                            targets.unknown |= !self.flow.implies(present, covered);
                            if self.flow.exceeded() {
                                return Err(budget());
                            }
                        }
                    }
                }
                call.inputs.push(((param, point, access, state), targets));
            }
            calls.insert(site, call);
        }
        Ok((calls, parts))
    }
}

#[cfg(test)]
mod tests;
