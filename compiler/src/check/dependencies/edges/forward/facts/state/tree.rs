use super::*;
use std::sync::Arc;

#[cfg(test)]
mod fixtures;
mod joins;
mod updates;
mod validation;

#[derive(Clone, Debug)]
pub(super) struct Tree {
    pub(self) owner: usize,
    pub(self) root: Option<Arc<Node>>,
    pub(self) checked: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Node {
    pub(self) low: hir::LocalId,
    pub(self) high: hir::LocalId,
    pub(self) len: usize,
    pub(self) weight: usize,
    pub(self) depth: usize,
    pub(self) bottom: bool,
    pub(self) kind: Kind,
}

#[derive(Clone, Debug)]
pub(super) enum Kind {
    Leaf(hir::LocalId, Cell),
    Branch(Arc<Node>, Arc<Node>),
}

pub(super) struct Iter<'a> {
    pub(self) pending: Vec<&'a Node>,
}

impl Tree {
    pub(super) fn owner(&self) -> usize {
        self.owner
    }

    pub(super) fn new(owner: usize) -> Self {
        Self::from_root(owner, None)
    }

    pub(self) fn from_root(owner: usize, root: Option<Arc<Node>>) -> Self {
        Self {
            owner,
            root,
            checked: true,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    pub(super) fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |node| node.len)
    }

    pub(super) fn weight(&self) -> usize {
        self.root.as_ref().map_or(0, |node| node.weight)
    }

    pub(super) fn depth(&self) -> usize {
        self.root.as_ref().map_or(0, |node| node.depth)
    }

    pub(super) fn bottom(&self) -> bool {
        self.root.as_ref().is_some_and(|node| node.bottom)
    }

    pub(super) fn get(&self, key: &hir::LocalId) -> Option<&Cell> {
        let mut node = self.root.as_deref()?;
        loop {
            match &node.kind {
                Kind::Leaf(local, cell) => return (key == local).then_some(cell),
                Kind::Branch(left, right) => node = if *key < right.low { left } else { right },
            }
        }
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = &hir::LocalId> {
        self.into_iter().map(|(key, _)| key)
    }

    pub(super) fn from_sorted(
        owner: usize,
        cells: Vec<(hir::LocalId, Cell)>,
        flow: &mut Flow,
        span: Span,
        limit: usize,
    ) -> Result<(Self, usize)> {
        if cells.len() >= MAX_EDGES || !flow.spend(cells.len() * 2 + 1) {
            return Err(budget(span));
        }
        let mut weight = cells.len().saturating_sub(1);
        let mut prior = None;
        for (local, cell) in &cells {
            if prior.is_some_and(|prior| prior >= *local) {
                return Err(invalid(span));
            }
            cell.validate(owner, *local, flow, span)?;
            weight = weight
                .checked_add(1 + cell.weight())
                .filter(|weight| *weight <= limit.min(MAX_EDGES))
                .ok_or_else(|| budget(span))?;
            prior = Some(*local);
        }
        let len = cells.len();
        let root = Node::build(&mut cells.into_iter(), len);
        Ok((Self::from_root(owner, root), weight))
    }
}

impl Node {
    pub(self) fn build(
        cells: &mut std::vec::IntoIter<(hir::LocalId, Cell)>,
        len: usize,
    ) -> Option<Arc<Self>> {
        if len == 0 {
            return None;
        }
        if len == 1 {
            let (local, cell) = cells.next().expect("validated cell count");
            return Some(Arc::new(Self::leaf(local, cell)));
        }
        let left = Self::build(cells, len / 2).expect("nonempty left half");
        let right = Self::build(cells, len - len / 2).expect("nonempty right half");
        Some(Arc::new(Self::branch(left, right)))
    }

    pub(self) fn leaf(local: hir::LocalId, cell: Cell) -> Self {
        Self {
            low: local,
            high: local,
            len: 1,
            weight: 1 + cell.weight(),
            depth: 1,
            bottom: cell.bottom(),
            kind: Kind::Leaf(local, cell),
        }
    }

    pub(self) fn branch(left: Arc<Self>, right: Arc<Self>) -> Self {
        Self {
            low: left.low,
            high: right.high,
            len: left.len + right.len,
            weight: left.weight + right.weight,
            depth: 1 + left.depth.max(right.depth),
            bottom: left.bottom || right.bottom,
            kind: Kind::Branch(left, right),
        }
    }
}

impl<'a> IntoIterator for &'a Tree {
    type Item = (&'a hir::LocalId, &'a Cell);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            pending: self.root.as_deref().into_iter().collect(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a hir::LocalId, &'a Cell);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.pending.pop() {
            match &node.kind {
                Kind::Leaf(local, cell) => return Some((local, cell)),
                Kind::Branch(left, right) => {
                    self.pending.push(right);
                    self.pending.push(left);
                }
            }
        }
        None
    }
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner && self.into_iter().eq(other)
    }
}

impl Eq for Tree {}

#[cfg(test)]
mod tests;
