use super::*;
use std::collections::btree_map::Entry;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Candidates {
    pub(super) mutable: bool,
    pub(super) values: Vec<Candidate>,
}

#[cfg(test)]
mod tests;

pub(super) type Index<'a> = BTreeMap<(hir::BlockId, Option<&'a str>), Candidates>;

impl Checker {
    #[cfg(test)]
    pub(super) fn result_source_index<'a>(
        &mut self,
        reports: &'a Reports,
        parts: &mut usize,
        span: Span,
    ) -> Result<Index<'a>> {
        let mut room = *parts;
        let dispatches = self.dispatch_result_index(reports, &mut room, MAX_EDGES, span)?;
        let index =
            self.result_source_index_with_dispatch(reports, &dispatches, &mut room, span)?;
        *parts = room;
        Ok(index)
    }

    pub(super) fn result_source_index_with_dispatch<'a>(
        &mut self,
        reports: &'a Reports,
        dispatches: &dispatch::Index,
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
            self.validate_emission_report(reports, statement, *owner, observed, span)?;
            for (target, value) in observed.targets.iter().enumerate() {
                if !observed.initialized[target] {
                    continue;
                }
                if !self.flow.spend(
                    reports.blocks.len().checked_ilog2().unwrap_or(0) as usize
                        + dispatches.len().checked_ilog2().unwrap_or(0) as usize
                        + self.bodies.len().checked_ilog2().unwrap_or(0) as usize
                        + self.proofs.aliases.len().checked_ilog2().unwrap_or(0) as usize
                        + 4,
                ) {
                    return Err(budget());
                }
                let block_owner =
                    if let Some((block_owner, block)) = reports.blocks.get(&value.block) {
                        if *block_owner != *owner {
                            return Err(invalid());
                        }
                        if !block.result {
                            continue;
                        }
                        *block_owner
                    } else if let Some(&(block_owner, _)) = dispatches.get(&value.block) {
                        block_owner
                    } else {
                        continue;
                    };
                if block_owner != *owner {
                    return Err(invalid());
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
