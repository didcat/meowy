use super::*;

impl Tree {
    pub(in super::super) fn updated(
        &self,
        local: hir::LocalId,
        cell: &Cell,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<(Self, usize)> {
        self.validate(flow, span)?;
        cell.validate(self.owner, local, flow, span)?;
        let limit = limit.min(MAX_EDGES);
        let mut room = limit;
        if let Some(root) = &self.root
            && let Some(root) = Node::updated(root, local, cell, flow, span, &mut room)?
        {
            if root.weight >= MAX_EDGES {
                return Err(budget(span));
            }
            return Ok((Self::from_root(self.owner, Some(root)), limit - room));
        }
        let weight = self
            .weight()
            .checked_add(cell.weight() + 1 + self.len())
            .filter(|weight| *weight <= limit)
            .ok_or_else(|| budget(span))?;
        if !flow.spend(self.weight() + self.len() * 4 + cell.weight() + 2) {
            return Err(budget(span));
        }
        let mut cells = self
            .into_iter()
            .map(|(&id, cell)| (id, cell.clone()))
            .collect::<Vec<_>>();
        let at = cells
            .binary_search_by_key(&local, |(id, _)| *id)
            .expect_err("absent cell");
        cells.insert(at, (local, cell.clone()));
        Self::from_sorted(self.owner, cells, flow, span, weight)
    }
}

impl Node {
    pub(super) fn updated(
        node: &Arc<Self>,
        local: hir::LocalId,
        cell: &Cell,
        flow: &mut Flow,
        span: Span,
        room: &mut usize,
    ) -> Result<Option<Arc<Self>>> {
        if !flow.spend(2) {
            return Err(budget(span));
        }
        match &node.kind {
            Kind::Leaf(id, old) => {
                if *id != local {
                    return Ok(None);
                }
                if old.same(cell, flow, span)? {
                    return Ok(Some(node.clone()));
                }
                *room = room
                    .checked_sub(1 + cell.weight())
                    .ok_or_else(|| budget(span))?;
                if !flow.spend(cell.weight()) {
                    return Err(budget(span));
                }
                Ok(Some(Arc::new(Self::leaf(local, cell.clone()))))
            }
            Kind::Branch(left, right) => {
                let old = if local < right.low { left } else { right };
                let Some(next) = Self::updated(old, local, cell, flow, span, room)? else {
                    return Ok(None);
                };
                if Arc::ptr_eq(old, &next) {
                    return Ok(Some(node.clone()));
                }
                *room = room.checked_sub(1).ok_or_else(|| budget(span))?;
                let next = if local < right.low {
                    Self::branch(next, right.clone())
                } else {
                    Self::branch(left.clone(), next)
                };
                Ok(Some(Arc::new(next)))
            }
        }
    }
}

#[cfg(test)]
mod tests;
