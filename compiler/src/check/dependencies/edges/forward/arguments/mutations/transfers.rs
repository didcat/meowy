use super::*;
use crate::borrow::Facts;
use crate::check::dependencies::edges::forward::effects::Effect;
use crate::flow::{FALSE, Guard};

pub(in super::super) struct Transfer<'a> {
    pub(in super::super) site: hir::CallId,
    pub(in super::super) call: targets::Call<'a>,
    pub(in super::super) observed: Option<(bool, bool)>,
    pub(in super::super) roots: storage::Roots,
    pub(in super::super) unknown: Option<Guard>,
}

pub(in super::super) type Transfers<'a> = BTreeMap<PointId, Transfer<'a>>;

impl Checker {
    pub(in super::super) fn call_mutation_transfers<'a>(
        &mut self,
        program: &Program,
        reports: &Reports,
        facts: &'a Facts,
        span: Span,
        parts: usize,
    ) -> Result<(Transfers<'a>, usize)> {
        let budget =
            || Diagnostic::unsupported("proof call-mutation-transfer budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof call-mutation-transfer identity mismatch", span);
        let (calls, mut parts) =
            self.call_mutation_targets(program, reports, facts, span, parts)?;
        self.validate_mutation_reports(reports, span)?;
        let mut transfers = Transfers::new();
        for (site, call) in calls {
            parts = parts.checked_sub(1).ok_or_else(budget)?;
            if !self.flow.spend(
                self.invocations.len().checked_ilog2().unwrap_or(0) as usize
                    + reports.effects.len().checked_ilog2().unwrap_or(0) as usize
                    + transfers.len().checked_ilog2().unwrap_or(0) as usize * 2
                    + 4,
            ) {
                return Err(budget());
            }
            let mut transfer = Transfer {
                site,
                call,
                observed: None,
                roots: storage::Roots::new(),
                unknown: None,
            };
            let point = self.invocations[&transfer.site].point;
            transfer.observed = match reports.effects.get(&point) {
                Some((
                    owner,
                    Effect::Call {
                        operation, result, ..
                    },
                )) if *owner == transfer.call.owner => Some((*operation, *result)),
                Some(_) => return Err(invalid()),
                None => None,
            };
            if transfer.observed.is_some_and(|(operation, _)| operation)
                && let Some(snapshot) = transfer.call.snapshot
                && snapshot.normal == snapshot.header.count
            {
                let entered = snapshot.header.entered;
                transfer.unknown = Some(FALSE);
                if !self.flow.spend(transfer.call.inputs.len() + 1) {
                    return Err(budget());
                }
                for ((_, _, _, state), targets) in &transfer.call.inputs {
                    let Some((state, true)) = state else {
                        return Err(invalid());
                    };
                    let present = self.flow.and(state.present, state.proof);
                    let active = self.flow.and(entered, present);
                    if targets.unknown || !self.flow.implies(entered, present) {
                        transfer.unknown = Some(entered);
                    }
                    if !self.flow.spend(targets.roots.len() + 1) {
                        return Err(budget());
                    }
                    for (&root, &guard) in &targets.roots {
                        let guard = self.flow.and(active, guard);
                        self.merge_mutation_root(
                            &mut transfer.roots,
                            root,
                            guard,
                            span,
                            &mut parts,
                        )?;
                    }
                    if self.flow.exceeded() {
                        return Err(budget());
                    }
                }
            }
            if transfers.insert(point, transfer).is_some() {
                return Err(invalid());
            }
        }
        Ok((transfers, parts))
    }

    pub(self) fn validate_mutation_reports(&mut self, reports: &Reports, span: Span) -> Result<()> {
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof call-mutation-transfer budget exhausted",
                span,
            ));
        }
        for (&point, (owner, effect)) in &reports.effects {
            if matches!(effect, Effect::Call { .. }) {
                self.validate_call_report(reports, point, *owner, effect, span)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
