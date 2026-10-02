use super::*;
use crate::check::dependencies::emissions::MAX_TARGETS;
use std::collections::btree_map::Entry;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Candidates {
    pub(super) mutable: bool,
    pub(super) values: Vec<Candidate>,
}

pub(super) type Index<'a> = BTreeMap<(hir::BlockId, Option<&'a str>), Candidates>;

impl Checker {
    pub(super) fn result_source_index<'a>(
        &mut self,
        reports: &'a Reports,
        parts: &mut usize,
        span: Span,
    ) -> Result<Index<'a>> {
        let budget = || Diagnostic::unsupported("proof result-source budget exhausted", span);
        let invalid = || Diagnostic::unsupported("proof result-source identity mismatch", span);
        if !self.flow.spend(reports.effects.len() + 1) {
            return Err(budget());
        }
        let mut index = Index::new();
        let mut room = *parts;
        for (&statement, (owner, effect)) in &reports.effects {
            let super::super::effects::Effect::Emission(observed) = effect else {
                continue;
            };
            let len = observed.targets.len();
            if len > MAX_TARGETS || !self.flow.spend(len + 1) {
                return Err(budget());
            }
            let bytes = observed.targets.iter().try_fold(0usize, |bytes, target| {
                bytes
                    .checked_add(target.field.as_ref().map_or(0, String::len))
                    .ok_or_else(budget)
            })?;
            if bytes > MAX_EDGES || !self.flow.spend(bytes + len * 3 + 8) {
                return Err(budget());
            }
            if self.emission_effect_stage(reports, *owner, Port::Normal(statement), span)?
                != Some((statement, None))
            {
                return Err(invalid());
            }
            let op = self.emissions.get(&statement).ok_or_else(invalid)?;
            if observed.input != op.input
                || observed.composed != op.composed
                || observed.targets != op.targets
                || observed.control != op.control
                || observed.initialized.len() != len
            {
                return Err(invalid());
            }
            for (target, value) in observed.targets.iter().enumerate() {
                if !observed.initialized[target] {
                    continue;
                }
                if !self.flow.spend(
                    reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                        + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                        + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                        + 4,
                ) {
                    return Err(budget());
                }
                let Some((block_owner, block)) = reports.blocks.get(&value.block) else {
                    continue;
                };
                if *block_owner != *owner {
                    return Err(invalid());
                }
                if !block.result {
                    continue;
                }
                if !matches!(
                    self.bodies.get(&value.block).ok_or_else(invalid)?.layout,
                    Layout::Slots(_)
                ) {
                    continue;
                }
                let mutable = match value.alias {
                    Some(id) => self.proofs.aliases.get(&id).ok_or_else(invalid)?.mutable,
                    None => false,
                };
                let key = (value.block, value.field.as_deref());
                let work = (key.1.map_or(0, str::len) + 1)
                    * (index.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3);
                if !self.flow.spend(work) {
                    return Err(budget());
                }
                let row = match index.entry(key) {
                    Entry::Occupied(row) => {
                        room = room.checked_sub(1).ok_or_else(budget)?;
                        row.into_mut()
                    }
                    Entry::Vacant(row) => {
                        room = room.checked_sub(2).ok_or_else(budget)?;
                        row.insert(Candidates::default())
                    }
                };
                row.mutable |= mutable;
                row.values.push(Candidate {
                    emission: value.id,
                    statement,
                    target,
                });
            }
        }
        *parts = room;
        Ok(index)
    }
}
