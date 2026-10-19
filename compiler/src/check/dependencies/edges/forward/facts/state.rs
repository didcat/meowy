use super::*;
use crate::flow::{FALSE, Flow, Guard};
use domain::Domain;
use versions::{Origin, Version};

mod cells;
mod entries;
mod joins;
mod snapshots;
pub(super) use cells::{Cell, Data};
pub(super) use entries::Entries;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct State {
    pub(self) owner: usize,
    pub(self) reach: Guard,
    pub(self) cells: BTreeMap<hir::LocalId, Cell>,
    pub(self) parts: usize,
}

impl State {
    pub(super) fn owner(&self) -> usize {
        self.owner
    }

    pub(super) fn new(owner: usize, reach: Guard, flow: &mut Flow, span: Span) -> Result<Self> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        if reach >> 1 >= flow.nodes.len() {
            return Err(invalid(span));
        }
        Ok(Self {
            owner,
            reach,
            cells: BTreeMap::new(),
            parts: 1,
        })
    }

    pub(super) fn get(&self, local: hir::LocalId) -> Option<&Cell> {
        self.cells.get(&local)
    }

    pub(super) fn weight(&self) -> usize {
        self.parts
    }

    pub(super) fn put(
        &mut self,
        local: hir::LocalId,
        cell: &Cell,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<()> {
        let limit = limit.min(MAX_EDGES);
        if !flow.spend(self.cells.len().checked_ilog2().unwrap_or(0) as usize * 2 + 3)
            || limit == 0
            || self.parts > MAX_EDGES
        {
            return Err(budget(span));
        }
        if self.reach >> 1 >= flow.nodes.len() {
            return Err(invalid(span));
        }
        cell.validate(self.owner, local, flow, span)?;
        if self.reach == FALSE {
            return Ok(());
        }
        if cell.bottom() {
            self.reach = FALSE;
            self.cells.clear();
            self.parts = 1;
            return Ok(());
        }
        let prior = self.cells.get(&local).map_or(0, |cell| 1 + cell.weight());
        let parts = self
            .parts
            .checked_sub(prior)
            .ok_or_else(|| invalid(span))?
            .checked_add(1 + cell.weight())
            .filter(|parts| *parts <= limit)
            .ok_or_else(|| budget(span))?;
        if !flow.spend(cell.weight()) {
            return Err(budget(span));
        }
        self.cells.insert(local, cell.clone());
        self.parts = parts;
        Ok(())
    }

    pub(super) fn snapshot(&self, flow: &mut Flow, span: Span, limit: usize) -> Result<Self> {
        self.validate(flow, span)?;
        self.copy(self.reach, flow, span, limit)
    }

    pub(self) fn copy(
        &self,
        reach: Guard,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Self> {
        let parts = if reach == FALSE { 1 } else { self.parts };
        if parts > limit.min(MAX_EDGES) || !flow.spend(parts) {
            return Err(budget(span));
        }
        Ok(Self {
            owner: self.owner,
            reach,
            cells: if reach == FALSE {
                BTreeMap::new()
            } else {
                self.cells.clone()
            },
            parts,
        })
    }

    pub(super) fn from_sorted(
        owner: usize,
        reach: Guard,
        cells: Vec<(hir::LocalId, Cell)>,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Self> {
        if cells.len() >= MAX_EDGES || !flow.spend(cells.len() + 1) {
            return Err(budget(span));
        }
        if reach >> 1 >= flow.nodes.len() {
            return Err(invalid(span));
        }
        let mut prior = None;
        let mut parts = 1usize;
        let mut bottom = reach == FALSE;
        for (local, cell) in &cells {
            if prior.is_some_and(|prior| prior >= *local) {
                return Err(invalid(span));
            }
            prior = Some(*local);
            cell.validate(owner, *local, flow, span)?;
            bottom |= cell.bottom();
            parts = parts
                .checked_add(1 + cell.weight())
                .filter(|parts| *parts <= limit.min(MAX_EDGES))
                .ok_or_else(|| budget(span))?;
        }
        if parts > limit.min(MAX_EDGES) || !flow.spend(cells.len() + 1) {
            return Err(budget(span));
        }
        Ok(Self {
            owner,
            reach: if bottom { FALSE } else { reach },
            cells: if bottom {
                BTreeMap::new()
            } else {
                cells.into_iter().collect()
            },
            parts: if bottom { 1 } else { parts },
        })
    }

    pub(self) fn validate(&self, flow: &mut Flow, span: Span) -> Result<()> {
        if self.cells.len() >= MAX_EDGES || !flow.spend(self.cells.len() + 1) {
            return Err(budget(span));
        }
        if self.reach >> 1 >= flow.nodes.len() || (self.reach == FALSE && !self.cells.is_empty()) {
            return Err(invalid(span));
        }
        let mut parts = 1usize;
        for (&local, cell) in &self.cells {
            cell.validate(self.owner, local, flow, span)?;
            if cell.bottom() {
                return Err(invalid(span));
            }
            parts = parts
                .checked_add(1 + cell.weight())
                .filter(|parts| *parts <= MAX_EDGES)
                .ok_or_else(|| budget(span))?;
        }
        if parts != self.parts {
            return Err(invalid(span));
        }
        Ok(())
    }
}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-state budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-state identity mismatch", span)
}

#[cfg(test)]
mod tests;
