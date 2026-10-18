use super::*;
use crate::check::dependencies::{OperationKind, edges::forward::effects::Effect};
use crate::flow::TRUE;
use versions::Definition;

pub(in super::super) type Entries = BTreeMap<usize, (Port, State)>;

pub(super) struct Seed {
    pub(super) port: Port,
    pub(super) cells: Vec<(hir::LocalId, Cell)>,
    pub(super) parts: usize,
}

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
        let mut seeds = BTreeMap::new();
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
                Seed {
                    port,
                    cells: Vec::new(),
                    parts: 1,
                },
            );
            parts -= 1;
        }
        let mut binds = self
            .scalar_entry_binds(versions, events, span)?
            .into_iter()
            .peekable();
        for (&local, slot) in versions {
            while binds.peek().is_some_and(|(id, _)| *id < local) {
                let (_, version) = binds.next().ok_or_else(|| invalid(span))?;
                self.scalar_entry_cell(
                    &mut seeds,
                    version,
                    Cell {
                        data: Data::Uninitialized,
                        control: false,
                    },
                    span,
                    &mut parts,
                )?;
            }
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
                slot.version,
                Cell {
                    data,
                    control: false,
                },
                span,
                &mut parts,
            )?;
        }
        for (_, version) in binds {
            self.scalar_entry_cell(
                &mut seeds,
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
        for (owner, seed) in seeds {
            if !self
                .flow
                .spend(entries.len().checked_ilog2().unwrap_or(0) as usize + 1)
            {
                return Err(budget(span));
            }
            let value =
                State::from_sorted(owner, TRUE, seed.cells, &mut self.flow, span, seed.parts)?;
            let empty = State::new(owner, FALSE, &mut self.flow, span)?;
            let state = empty.join(&value, &mut self.flow, span, parts)?;
            parts = parts
                .checked_sub(state.weight())
                .ok_or_else(|| budget(span))?;
            entries.insert(owner, (seed.port, state));
        }
        Ok((entries, parts))
    }

    pub(self) fn scalar_entry_binds(
        &mut self,
        versions: &versions::Versions,
        events: &events::Events,
        span: Span,
    ) -> Result<BTreeMap<hir::LocalId, Version>> {
        let mut binds = BTreeMap::new();
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
            if !self.flow.spend(
                versions.len().checked_ilog2().unwrap_or(0) as usize
                    + binds.len().checked_ilog2().unwrap_or(0) as usize
                    + 3,
            ) {
                return Err(budget(span));
            }
            if versions.contains_key(&version.local)
                || binds.insert(version.local, version).is_some()
            {
                return Err(invalid(span));
            }
        }
        Ok(binds)
    }

    pub(self) fn scalar_entry_cell(
        &mut self,
        seeds: &mut BTreeMap<usize, Seed>,
        version: Version,
        cell: Cell,
        span: Span,
        parts: &mut usize,
    ) -> Result<()> {
        if !self
            .flow
            .spend(seeds.len().checked_ilog2().unwrap_or(0) as usize + 1)
        {
            return Err(budget(span));
        }
        let seed = seeds.get_mut(&version.owner).ok_or_else(|| invalid(span))?;
        let weight = 1 + cell.weight();
        *parts = parts.checked_sub(weight).ok_or_else(|| budget(span))?;
        seed.parts = seed.parts.checked_add(weight).ok_or_else(|| budget(span))?;
        seed.cells.push((version.local, cell));
        Ok(())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod boundaries;
