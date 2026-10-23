use super::*;

impl State {
    pub(super) fn slot(&self, block: hir::BlockId) -> Option<&Cell> {
        self.slots.get(&block)
    }

    pub(super) fn put_slot(
        &mut self,
        block: hir::BlockId,
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
        if matches!(
            cell.data,
            Data::Initialized {
                version: Some(_),
                ..
            }
        ) {
            return Err(invalid(span));
        }
        if self.reach == FALSE || cell.bottom() {
            cell.validate(self.owner, block, flow, span)?;
            if cell.bottom() {
                self.reach = FALSE;
                self.cells = tree::Tree::new(self.owner);
                self.slots = tree::Tree::new(self.owner);
                self.parts = 1;
            }
            return Ok(());
        }
        let (slots, used) = self.slots.updated(block, cell, flow, span, room)?;
        let parts = 1 + self.cells.weight() + slots.weight();
        if parts > MAX_EDGES {
            return Err(budget(span));
        }
        self.slots = slots;
        self.parts = parts;
        self.used += used;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
