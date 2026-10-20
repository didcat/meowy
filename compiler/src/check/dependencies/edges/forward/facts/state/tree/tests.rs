use super::*;

pub(super) fn cells() -> Vec<(hir::LocalId, Cell)> {
    (0..64)
        .map(|local| {
            (
                local * 3,
                Cell {
                    data: Data::Uninitialized,
                    control: local % 2 == 0,
                },
            )
        })
        .collect()
}

#[test]
pub(crate) fn scalar_cell_trees_preserve_ordered_sparse_keys_and_shared_snapshots() {
    let mut flow = Flow::default();
    let (tree, parts) =
        Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    assert_eq!(tree.len(), 64);
    assert_eq!(tree.depth(), 7);
    assert_eq!(tree.weight(), 128);
    assert_eq!(parts, 191);
    assert_eq!(
        tree.keys().copied().collect::<Vec<_>>(),
        (0..64).map(|local| local * 3).collect::<Vec<_>>()
    );
    for (local, cell) in cells() {
        assert_eq!(tree.get(&local), Some(&cell));
    }
    assert_eq!(tree.get(&1), None);
    let copy = tree.clone();
    assert!(Arc::ptr_eq(
        tree.root.as_ref().unwrap(),
        copy.root.as_ref().unwrap()
    ));
    assert_eq!(tree, copy);
    assert_eq!(
        Tree::new(7),
        Tree::from_sorted(7, Vec::new(), &mut flow, Span::default(), 0)
            .unwrap()
            .0
    );
}

#[test]
pub(crate) fn scalar_cell_trees_reject_unsorted_foreign_and_over_budget_construction() {
    let mut flow = Flow::default();
    let start = flow.work;
    let (expected, used) =
        Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    let cost = flow.work - start;
    for (limit, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = Tree::from_sorted(7, cells(), &mut flow, Span::default(), limit);
        assert_eq!(result.is_ok(), pass);
        if let Ok((tree, parts)) = result {
            assert_eq!(parts, used);
            assert_eq!(tree, expected);
        }
    }
    for fault in 0..3 {
        flow.work = 0;
        flow.full = false;
        let mut cells = cells();
        match fault {
            0 => cells.reverse(),
            1 => cells[1].0 = cells[0].0,
            2 => {
                cells[0].1.data = Data::Initialized {
                    version: Some(Version {
                        owner: 8,
                        local: 0,
                        definition: versions::Definition::Parameter(0),
                    }),
                    domain: None,
                    origin: None,
                }
            }
            _ => unreachable!(),
        }
        assert!(Tree::from_sorted(7, cells, &mut flow, Span::default(), MAX_EDGES).is_err());
    }
}
