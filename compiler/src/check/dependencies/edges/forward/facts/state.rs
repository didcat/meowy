use super::*;
use crate::flow::{FALSE, Flow, Guard};
use domain::Domain;
use versions::{Origin, Version};

mod cells;
mod entries;
mod joins;
#[cfg(test)]
mod slots;
mod snapshots;
mod tree;
pub(super) use cells::{Cell, Data};
pub(super) use entries::Entries;

#[derive(Debug)]
pub(super) struct State {
    pub(self) owner: usize,
    pub(self) reach: Guard,
    pub(self) cells: tree::Tree,
    pub(self) slots: tree::Tree,
    pub(self) parts: usize,
    pub(self) used: usize,
}

impl State {
    pub(super) fn owner(&self) -> usize {
        self.owner
    }

    #[cfg(test)]
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
            cells: tree::Tree::new(owner),
            slots: tree::Tree::new(owner),
            parts: 1,
            used: 1,
        })
    }

    pub(super) fn get(&self, local: hir::LocalId) -> Option<&Cell> {
        self.cells.get(&local)
    }

    pub(super) fn weight(&self) -> usize {
        self.parts
    }

    pub(super) fn allocation(&self) -> usize {
        self.used
    }

    pub(super) fn put(
        &mut self,
        local: hir::LocalId,
        cell: &Cell,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<()> {
        self.validate(flow, span)?;
        let room = limit
            .min(MAX_EDGES)
            .checked_sub(self.used)
            .ok_or_else(|| budget(span))?;
        if self.reach == FALSE || cell.bottom() {
            cell.validate(self.owner, local, flow, span)?;
            if cell.bottom() {
                self.reach = FALSE;
                self.cells = tree::Tree::new(self.owner);
                self.slots = tree::Tree::new(self.owner);
                self.parts = 1;
            }
            return Ok(());
        }
        let (cells, used) = self.cells.updated(local, cell, flow, span, room)?;
        let parts = 1 + cells.weight() + self.slots.weight();
        if parts > MAX_EDGES {
            return Err(budget(span));
        }
        self.parts = parts;
        self.cells = cells;
        self.used += used;
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
        if limit == 0 || !flow.spend(1) {
            return Err(budget(span));
        }
        Ok(Self {
            owner: self.owner,
            reach,
            cells: if reach == FALSE {
                tree::Tree::new(self.owner)
            } else {
                self.cells.clone()
            },
            slots: if reach == FALSE {
                tree::Tree::new(self.owner)
            } else {
                self.slots.clone()
            },
            parts: if reach == FALSE { 1 } else { self.parts },
            used: 1,
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
        if !flow.spend(1) {
            return Err(budget(span));
        }
        if reach >> 1 >= flow.nodes.len() {
            return Err(invalid(span));
        }
        let room = limit
            .min(MAX_EDGES)
            .checked_sub(1)
            .ok_or_else(|| budget(span))?;
        let (cells, used) = tree::Tree::from_sorted(owner, cells, flow, span, room)?;
        let bottom = reach == FALSE || cells.bottom();
        Ok(Self {
            owner,
            reach: if bottom { FALSE } else { reach },
            parts: if bottom { 1 } else { 1 + cells.weight() },
            cells: if bottom {
                tree::Tree::new(owner)
            } else {
                cells
            },
            slots: tree::Tree::new(owner),
            used: used + 1,
        })
    }

    pub(self) fn validate(&self, flow: &mut Flow, span: Span) -> Result<()> {
        if !flow.spend(2) || self.used == 0 || self.used > MAX_EDGES || self.parts > MAX_EDGES {
            return Err(budget(span));
        }
        self.cells.validate(flow, span)?;
        if !self.slots.is_empty() {
            self.slots.validate(flow, span)?;
        }
        if self.reach >> 1 >= flow.nodes.len()
            || self.cells.owner() != self.owner
            || self.slots.owner() != self.owner
            || self.parts != 1 + self.cells.weight() + self.slots.weight()
            || self.cells.bottom()
            || self.slots.bottom()
            || self.reach == FALSE && (!self.cells.is_empty() || !self.slots.is_empty())
        {
            return Err(invalid(span));
        }
        Ok(())
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.reach == other.reach
            && self.parts == other.parts
            && self.cells == other.cells
            && self.slots == other.slots
    }
}

impl Eq for State {}

pub(super) fn budget(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-state budget exhausted", span)
}

pub(super) fn invalid(span: Span) -> Diagnostic {
    Diagnostic::unsupported("proof scalar-state identity mismatch", span)
}

#[cfg(test)]
mod tests;
