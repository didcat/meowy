use super::*;
use crate::check::dependencies::{OperationKind, edges::forward::effects::Effect};
use crate::flow::TRUE;
use std::collections::BTreeSet;
use versions::Definition;

pub(in super::super) type Entries = BTreeMap<usize, (Port, State)>;

impl Checker {
    pub(in super::super) fn scalar_entry_states(
        &mut self,
        reports: &Reports,
        versions: &versions::Versions,
        events: &events::Events,
        span: Span,
        parts: usize,
    ) -> Result<(Entries, usize)> {
        let mut parts = parts.min(MAX_EDGES);
        if reports.entries.len() > parts
            || versions.len() > MAX_EDGES
            || events.len() > MAX_EDGES
            || !self
                .flow
                .spend(reports.entries.len() + versions.len() + events.len() + 1)
        {
            return Err(budget(span));
        }
        let mut seeds = Entries::new();
        let mut seen = BTreeSet::new();
        for (&owner, (block, _)) in &reports.entries {
            if !self
                .flow
                .spend(seeds.len().checked_ilog2().unwrap_or(0) as usize + 2)
            {
                return Err(budget(span));
            }
            let port = Port::BlockEntry(*block);
            if self.port_owner(port, span)? != owner {
                return Err(invalid(span));
            }
            seeds.insert(
                owner,
                (port, State::new(owner, TRUE, &mut self.flow, span)?),
            );
            parts -= 1;
        }
        for (&local, slot) in versions {
            if local != slot.version.local {
                return Err(invalid(span));
            }
            let data = match slot.version.definition {
                Definition::Parameter(position) => {
                    let function = slot
                        .version
                        .owner
                        .checked_sub(1)
                        .ok_or_else(|| invalid(span))?;
                    if !self.flow.spend(
                        reports.signatures.params.len().checked_ilog2().unwrap_or(0) as usize
                            + 2
                            + slot.declared.as_ref().map_or(0, Domain::weight),
                    ) {
                        return Err(budget(span));
                    }
                    if reports.signatures.params.get(&local) != Some(&(function, position)) {
                        return Err(invalid(span));
                    }
                    Data::Initialized {
                        version: Some(slot.version),
                        domain: slot.declared.clone(),
                        origin: slot
                            .declared
                            .as_ref()
                            .map(|_| Origin::Parameter(function, position)),
                    }
                }
                Definition::Bind(_) => Data::Uninitialized,
                Definition::Write(_) => return Err(invalid(span)),
            };
            self.scalar_entry_cell(
                &mut seeds,
                &mut seen,
                slot.version,
                Cell {
                    data,
                    control: false,
                },
                span,
                &mut parts,
            )?;
        }
        for (&point, event) in events {
            let Some(version) = event.version else {
                continue;
            };
            let (
                owner,
                Effect::Storage {
                    kind,
                    local,
                    storage,
                    ..
                },
            ) = event.source
            else {
                return Err(invalid(span));
            };
            if version.owner != *owner || version.local != *storage || local != storage {
                return Err(invalid(span));
            }
            match (*kind, version.definition) {
                (OperationKind::Write, Definition::Write(id)) if id == point => continue,
                (OperationKind::Bind, Definition::Bind(id)) if id == point => {}
                _ => return Err(invalid(span)),
            }
            self.scalar_entry_cell(
                &mut seeds,
                &mut seen,
                version,
                Cell {
                    data: Data::Uninitialized,
                    control: false,
                },
                span,
                &mut parts,
            )?;
        }
        let mut entries = Entries::new();
        for (owner, (port, seed)) in seeds {
            if !self
                .flow
                .spend(entries.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget(span));
            }
            let empty = State::new(owner, FALSE, &mut self.flow, span)?;
            let state = empty.join(&seed, &mut self.flow, span, parts)?;
            parts = parts
                .checked_sub(state.weight())
                .ok_or_else(|| budget(span))?;
            entries.insert(owner, (port, state));
        }
        Ok((entries, parts))
    }

    pub(self) fn scalar_entry_cell(
        &mut self,
        seeds: &mut Entries,
        seen: &mut BTreeSet<hir::LocalId>,
        version: Version,
        cell: Cell,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        if !self.flow.spend(
            seeds.len().checked_ilog2().unwrap_or(0) as usize
                + seen.len().checked_ilog2().unwrap_or(0) as usize * 2
                + 3,
        ) {
            return Err(budget(span));
        }
        if !seen.insert(version.local) {
            return Err(invalid(span));
        }
        let (_, state) = seeds.get_mut(&version.owner).ok_or_else(|| invalid(span))?;
        let prior = state.weight();
        let available = parts.checked_add(prior).ok_or_else(|| budget(span))?;
        state.put(version.local, &cell, &mut self.flow, span, available)?;
        *parts = parts
            .checked_sub(state.weight().saturating_sub(prior))
            .ok_or_else(|| budget(span))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
