use super::*;
use crate::check::dependencies::operations::SharedRecord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Shared {
    pub(crate) init: Initializer,
    pub(crate) record: SharedRecord,
}

pub(crate) type Index = BTreeMap<hir::LocalId, Shared>;

impl Checker {
    pub(in super::super) fn shared_initializers(
        &mut self,
        program: &hir::Program,
        reports: &Reports,
        span: Span,
        limit: usize,
        mut parts: usize,
    ) -> Result<(Index, usize)> {
        let budget = || Diagnostic::unsupported("proof shared-initializer budget exhausted", span);
        let invalid =
            || Diagnostic::unsupported("proof shared-initializer identity mismatch", span);
        if reports.effects.len() > MAX_EDGES || !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let params = &self.program_signatures(program, reports, span)?.params;
        let mut index = Index::new();
        for (&id, (owner, effect)) in &reports.effects {
            if !matches!(effect, Effect::Storage { .. }) {
                continue;
            }
            let Some(binding) = self.binding_effect(reports, id, *owner, span)? else {
                continue;
            };
            let local = binding.local;
            let ty = program.locals.get(local).ok_or_else(invalid)?;
            let record = Self::shared_record_type(ty, &mut self.flow, span)?;
            if record != self.operations[&id].shared {
                return Err(invalid());
            }
            let Some(record) = record else {
                continue;
            };
            let work = [
                self.proofs.mutable.len(),
                self.proofs.fields.len(),
                self.proofs.aliases.len(),
                self.proofs.receivers.len(),
                self.proofs.temporaries.len(),
                params.len(),
                index.len(),
                index.len(),
            ]
            .into_iter()
            .fold(12, |work, len| {
                work + len.checked_ilog2().unwrap_or(0) as usize
            });
            if !self.flow.spend(work) {
                return Err(budget());
            }
            if self.proofs.variable(local)
                || local != binding.storage
                || self.proofs.aliases.contains_key(&local)
                || self.proofs.receivers.contains(&local)
                || self.proofs.temporaries.contains_key(&local)
                || params.contains_key(&local)
            {
                continue;
            }
            if index.contains_key(&local) {
                return Err(invalid());
            }
            if index.len() >= limit.min(MAX_EDGES) {
                return Err(budget());
            }
            parts = parts.checked_sub(1).ok_or_else(budget)?;
            index.insert(
                local,
                Shared {
                    init: Initializer {
                        statement: id,
                        owner: *owner,
                        input: binding.input,
                    },
                    record,
                },
            );
        }
        Ok((index, parts))
    }
}

#[cfg(test)]
mod tests;
