use super::*;

impl State {
    pub(super) fn initialize_slot(
        &self,
        block: hir::BlockId,
        slot: &observations::Slot,
        mut cell: Cell,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Option<Self>> {
        if !flow.spend(self.slots.depth() + 4) {
            return Err(budget(span));
        }
        if slot.owner != self.owner || slot.declared == Domain::BOTTOM {
            return Err(invalid(span));
        }
        let Data::Initialized {
            version: None,
            domain: Some(domain),
            ..
        } = &cell.data
        else {
            return Err(invalid(span));
        };
        let Some(old) = self.slot(block) else {
            return Ok(None);
        };
        if old.data != Data::Uninitialized || !domain.fits(&slot.declared, flow, span)? {
            return Ok(None);
        }
        cell.control |= old.control || slot.control.unwrap_or(true);
        let mut next = self.snapshot(flow, span, limit)?;
        next.put_slot(block, &cell, flow, span, limit)?;
        Ok(Some(next))
    }

    pub(super) fn enter_slot(
        &self,
        block: hir::BlockId,
        slot: &observations::Slot,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<Option<Self>> {
        if slot.declared.weight() > MAX_EDGES
            || !flow.spend(self.slots.depth() + slot.declared.weight() + 4)
        {
            return Err(budget(span));
        }
        if slot.owner != self.owner || !slot.declared.valid() || slot.declared == Domain::BOTTOM {
            return Err(invalid(span));
        }
        if self.slot(block).is_some() {
            return Ok(None);
        }
        let mut next = self.snapshot(flow, span, limit)?;
        next.put_slot(
            block,
            &Cell {
                data: Data::Uninitialized,
                control: slot.control.unwrap_or(true),
            },
            flow,
            span,
            limit,
        )?;
        Ok(Some(next))
    }

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

#[cfg(test)]
mod boundaries;
