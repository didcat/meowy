use super::*;

impl Tree {
    pub(in super::super) fn joined<const SLOTS: bool>(
        &self,
        other: &Self,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<(Self, usize)> {
        if !flow.spend(1) {
            return Err(budget(span));
        }
        if self.owner != other.owner {
            return Err(invalid(span));
        }
        self.validate(flow, span)?;
        other.validate(flow, span)?;
        let limit = limit.min(MAX_EDGES);
        let mut room = limit;
        let root = Node::joined::<SLOTS>(
            self.root.as_ref(),
            other.root.as_ref(),
            self.owner,
            flow,
            span,
            &mut room,
        )?;
        if root
            .as_ref()
            .is_some_and(|root| root.weight >= MAX_EDGES || root.depth > usize::BITS as usize)
        {
            return Err(budget(span));
        }
        Ok((Self::from_root(self.owner, root), limit - room))
    }
}

impl Node {
    pub(super) fn joined<const SLOTS: bool>(
        left: Option<&Arc<Self>>,
        right: Option<&Arc<Self>>,
        owner: usize,
        flow: &mut Flow,
        span: Span,
        room: &mut usize,
    ) -> Result<Option<Arc<Self>>> {
        if !flow.spend(2) {
            return Err(budget(span));
        }
        if let (Some(left), Some(right)) = (left, right) {
            if Arc::ptr_eq(left, right) {
                return Ok(Some(left.clone()));
            }
            match (&left.kind, &right.kind) {
                (Kind::Leaf(a, x), Kind::Leaf(b, y)) if a == b => {
                    let cell = Cell::join::<SLOTS>(Some(x), Some(y), flow, span)?;
                    if cell.same(x, flow, span)? {
                        return Ok(Some(left.clone()));
                    }
                    if cell.same(y, flow, span)? {
                        return Ok(Some(right.clone()));
                    }
                    *room = room
                        .checked_sub(1 + cell.weight())
                        .ok_or_else(|| budget(span))?;
                    if !flow.spend(1) {
                        return Err(budget(span));
                    }
                    return Ok(Some(Arc::new(Self::leaf(*a, cell))));
                }
                (Kind::Branch(a, b), Kind::Branch(x, y)) if b.low == y.low => {
                    let first = Self::joined::<SLOTS>(Some(a), Some(x), owner, flow, span, room)?
                        .ok_or_else(|| invalid(span))?;
                    let last = Self::joined::<SLOTS>(Some(b), Some(y), owner, flow, span, room)?
                        .ok_or_else(|| invalid(span))?;
                    if Arc::ptr_eq(a, &first) && Arc::ptr_eq(b, &last) {
                        return Ok(Some(left.clone()));
                    }
                    if Arc::ptr_eq(x, &first) && Arc::ptr_eq(y, &last) {
                        return Ok(Some(right.clone()));
                    }
                    *room = room.checked_sub(1).ok_or_else(|| budget(span))?;
                    return Ok(Some(Arc::new(Self::branch(first, last))));
                }
                _ => {}
            }
        }
        Self::merged::<SLOTS>(left, right, owner, flow, span, room)
    }

    pub(self) fn merged<const SLOTS: bool>(
        left: Option<&Arc<Self>>,
        right: Option<&Arc<Self>>,
        owner: usize,
        flow: &mut Flow,
        span: Span,
        room: &mut usize,
    ) -> Result<Option<Arc<Self>>> {
        let left = Tree::from_root(owner, left.cloned());
        let right = Tree::from_root(owner, right.cloned());
        if !flow.spend((left.len() + right.len()) * 2 + 1) {
            return Err(budget(span));
        }
        let mut a = left.into_iter().peekable();
        let mut b = right.into_iter().peekable();
        let mut cells = Vec::new();
        let mut weight = 0usize;
        while a.peek().is_some() || b.peek().is_some() {
            let local = match (a.peek(), b.peek()) {
                (Some((a, _)), Some((b, _))) => **a.min(b),
                (Some((local, _)), None) | (None, Some((local, _))) => **local,
                _ => unreachable!(),
            };
            let x = a.next_if(|(id, _)| **id == local).map(|(_, cell)| cell);
            let y = b.next_if(|(id, _)| **id == local).map(|(_, cell)| cell);
            let cell = Cell::join::<SLOTS>(x, y, flow, span)?;
            weight = weight
                .checked_add(1 + cell.weight())
                .filter(|weight| *weight <= *room)
                .ok_or_else(|| budget(span))?;
            cells.push((local, cell));
        }
        let (tree, used) = Tree::from_sorted(owner, cells, flow, span, *room)?;
        *room = room.checked_sub(used).ok_or_else(|| budget(span))?;
        Ok(tree.root)
    }
}

#[cfg(test)]
mod tests;
