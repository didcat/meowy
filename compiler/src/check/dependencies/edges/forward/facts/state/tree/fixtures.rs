use super::*;

impl Tree {
    pub(in super::super) fn get_mut(&mut self, key: &hir::LocalId) -> Option<&mut Cell> {
        self.checked = false;
        let mut node = Arc::make_mut(self.root.as_mut()?);
        loop {
            match &mut node.kind {
                Kind::Leaf(local, cell) => return (key == local).then_some(cell),
                Kind::Branch(left, right) => {
                    node = Arc::make_mut(if *key < right.low { left } else { right })
                }
            }
        }
    }

    pub(in super::super) fn clear(&mut self) {
        self.root = None;
        self.checked = true;
    }

    pub(in super::super) fn remove(&mut self, key: &hir::LocalId) -> Option<Cell> {
        let mut cells = self
            .into_iter()
            .map(|(&id, cell)| (id, cell.clone()))
            .collect::<Vec<_>>();
        let at = cells.binary_search_by_key(key, |(local, _)| *local).ok()?;
        let (_, cell) = cells.remove(at);
        self.root = Node::build(&mut cells.into_iter(), self.len() - 1);
        self.checked = false;
        Some(cell)
    }
}

#[test]
pub(crate) fn scalar_cell_fixtures_preserve_shared_roots_and_revalidate_corrupted_cells() {
    let mut flow = Flow::default();
    let (mut tree, _) = Tree::from_sorted(
        7,
        super::tests::cells(),
        &mut flow,
        Span::default(),
        MAX_EDGES,
    )
    .unwrap();
    let old = tree.clone();
    tree.get_mut(&6).unwrap().control = false;
    tree.validate(&mut flow, Span::default()).unwrap();
    assert!(old.get(&6).unwrap().control);
    tree.get_mut(&6).unwrap().data = Data::Initialized {
        version: Some(Version {
            owner: 8,
            local: 6,
            definition: versions::Definition::Parameter(0),
        }),
        domain: None,
        origin: None,
    };
    assert!(tree.validate(&mut flow, Span::default()).is_err());
    assert!(tree.remove(&6).is_some());
    assert!(tree.get(&6).is_none());
    tree.validate(&mut flow, Span::default()).unwrap();
    tree.clear();
    assert!(tree.is_empty());
}
