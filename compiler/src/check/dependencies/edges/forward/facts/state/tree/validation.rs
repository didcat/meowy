use super::*;

impl Tree {
    pub(in super::super) fn validate(&self, flow: &mut Flow, span: Span) -> Result<()> {
        if !flow.spend(1)
            || self.len() >= MAX_EDGES
            || self.weight() >= MAX_EDGES
            || self.depth() > usize::BITS as usize
        {
            return Err(budget(span));
        }
        if self.checked {
            return Ok(());
        }
        let mut pending = self.root.as_deref().into_iter().collect::<Vec<_>>();
        while let Some(node) = pending.pop() {
            if !flow.spend(2) {
                return Err(budget(span));
            }
            match &node.kind {
                Kind::Leaf(local, cell) => {
                    cell.validate(self.owner, *local, flow, span)?;
                    if node.low != *local
                        || node.high != *local
                        || node.len != 1
                        || node.depth != 1
                        || node.weight != 1 + cell.weight()
                        || node.bottom != cell.bottom()
                    {
                        return Err(invalid(span));
                    }
                }
                Kind::Branch(left, right) => {
                    if node.low != left.low
                        || node.high != right.high
                        || left.high >= right.low
                        || left.len.checked_add(right.len) != Some(node.len)
                        || left.weight.checked_add(right.weight) != Some(node.weight)
                        || left.depth.max(right.depth).checked_add(1) != Some(node.depth)
                        || node.bottom != (left.bottom || right.bottom)
                    {
                        return Err(invalid(span));
                    }
                    pending.push(right);
                    pending.push(left);
                }
            }
        }
        Ok(())
    }
}
