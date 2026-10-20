use super::super::Mutations;
use super::*;
use crate::borrow::Facts;
use crate::hir::ReferenceMode;
use transfers::Authority;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mutation {
    pub(in super::super::super) site: hir::CallId,
    pub(in super::super::super) owner: usize,
    pub(in super::super::super) input: Port,
    pub(in super::super::super) roots: Vec<hir::LocalId>,
    pub(in super::super::super) unknown: bool,
    pub(in super::super::super) control: bool,
}

impl Checker {
    pub(in super::super::super) fn scalar_call_mutations(
        &mut self,
        program: &Program,
        reports: &Reports,
        facts: &Facts,
        span: Span,
        parts: usize,
    ) -> Result<(Mutations, usize)> {
        let budget = || Diagnostic::unsupported("proof scalar-call-input budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof scalar-call-input identity mismatch", span);
        let (transfers, mut parts) =
            self.call_mutation_transfers(program, reports, facts, span, parts)?;
        if !self.flow.spend(transfers.len() + 1) {
            return Err(budget());
        }
        let mut mutations = Mutations::new();
        for (point, transfer) in transfers {
            if !transfer.observed.is_some_and(|(operation, _)| operation) {
                continue;
            }
            if !self.flow.spend(
                self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                    + transfer.roots.len()
                    + transfer.call.inputs.len()
                    + 4,
            ) {
                return Err(budget());
            }
            let input = self
                .invocations
                .get(&transfer.site)
                .ok_or_else(invalid)?
                .args
                .last()
                .map_or(Port::Entry(point), |input| Port::Normal(*input));
            let mut unknown = transfer.entry == Authority::Unknown || transfer.unknown.is_none();
            for ((_, input, access, state), targets) in &transfer.call.inputs {
                unknown |= state.is_none_or(|(_, normal)| !normal) || targets.unknown;
                match access {
                    Access::Value | Access::Shared(_) => (),
                    Access::Unknown => unknown = true,
                    Access::Exclusive(_) => {
                        if !self.flow.spend(
                            self.place_borrows.len().checked_ilog2().unwrap_or(0) as usize
                                + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                                + 5,
                        ) {
                            return Err(budget());
                        }
                        let source = match targets.origins.as_slice() {
                            [(_, source)] => Some(*source),
                            _ => None,
                        };
                        let direct = source
                            .and_then(|source| self.place_borrows.get(&source))
                            .is_some_and(|borrow| {
                                borrow.owner == transfer.call.owner
                                    && borrow.mode == ReferenceMode::Exclusive
                                    && borrow.place.fields.is_empty()
                                    && borrow.place.root == borrow.storage
                                    && !self.proofs.aliases.contains_key(&borrow.place.root)
                                    && targets.roots.len() == 1
                                    && targets
                                        .roots
                                        .first_key_value()
                                        .is_some_and(|(root, _)| *root == borrow.storage)
                            });
                        unknown |= !direct
                            || !self.scalar_mutation_source(
                                reports,
                                transfer.call.owner,
                                *input,
                                source,
                                span,
                            )?;
                    }
                }
            }
            let weight = transfer.roots.len() + 1;
            parts = parts.checked_sub(weight).ok_or_else(budget)?;
            if !self
                .flow
                .spend(weight + mutations.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget());
            }
            mutations.insert(
                point,
                Mutation {
                    site: transfer.site,
                    owner: transfer.call.owner,
                    input,
                    roots: transfer.roots.keys().copied().collect(),
                    unknown,
                    control: transfer.entry != Authority::Independent,
                },
            );
        }
        Ok((mutations, parts))
    }

    pub(self) fn scalar_mutation_source(
        &mut self,
        reports: &Reports,
        owner: usize,
        mut point: PointId,
        source: Option<PointId>,
        span: Span,
    ) -> Result<bool> {
        use crate::check::dependencies::edges::forward::effects::Effect;
        let mut seen = std::collections::HashSet::new();
        loop {
            if seen.len() >= crate::check::dependencies::grouped::MAX_GROUPS
                || !self.flow.spend(
                    self.group_inputs.len().checked_ilog2().unwrap_or(0) as usize
                        + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                        + 3,
                )
            {
                return Err(Diagnostic::unsupported(
                    "proof scalar-call-input budget exhausted",
                    span,
                ));
            }
            if !seen.insert(point) {
                return Err(Diagnostic::unsupported(
                    "proof scalar-call-input identity mismatch",
                    span,
                ));
            }
            if Some(point) == source {
                return Ok(true);
            }
            let next = if let Some(&group) = self.group_inputs.get(&point) {
                Some(self.qualified_group_input(point, owner, group, span)?)
            } else {
                match reports.effects.get(&point).map(|(_, effect)| effect) {
                    Some(Effect::Coercion(_)) => {
                        self.forward_coercion_input(reports, point, owner, span)?
                    }
                    Some(Effect::Narrowing(_)) => {
                        self.unchanged_narrowing_input(reports, point, owner, span)?
                    }
                    Some(Effect::Typed(_)) => {
                        self.unchanged_ascription_input(reports, point, owner, span)?
                    }
                    _ => None,
                }
            };
            let Some(next) = next else {
                return Ok(false);
            };
            point = next;
        }
    }
}

#[cfg(test)]
mod tests;
