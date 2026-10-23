use super::super::tests::cells;
use super::*;

#[test]
pub(crate) fn scalar_cell_joins_reuse_shared_paths_and_preserve_partial_initialization() {
    let mut flow = Flow::default();
    let (tree, _) = Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    let cell = Cell {
        data: Data::Initialized {
            version: None,
            domain: Some(Domain::boolean(true)),
            origin: None,
        },
        control: true,
    };
    let (next, _) = tree
        .updated(6, &cell, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let (joined, used) = tree
        .joined::<false>(&next, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(joined.get(&6).unwrap().data, Data::Partial);
    assert!(joined.get(&6).unwrap().control);
    assert!(!joined.bottom());
    assert_eq!(used, tree.depth() + 1);
    let (Kind::Branch(_, old), Kind::Branch(_, new)) = (
        &tree.root.as_ref().unwrap().kind,
        &joined.root.as_ref().unwrap().kind,
    ) else {
        panic!()
    };
    assert!(Arc::ptr_eq(old, new));
    let (same, used) = next
        .joined::<false>(&next.clone(), &mut flow, Span::default(), 0)
        .unwrap();
    assert_eq!(used, 0);
    assert!(Arc::ptr_eq(
        same.root.as_ref().unwrap(),
        next.root.as_ref().unwrap()
    ));
    let (missing, _) = tree
        .joined::<false>(&Tree::new(7), &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        missing
            .into_iter()
            .all(|(_, cell)| cell.data == Data::Unavailable)
    );
}

#[test]
pub(crate) fn scalar_cell_joins_match_conservative_map_joins_across_different_keys_and_shapes() {
    let mut flow = Flow::default();
    for (a, b) in [
        (vec![0, 2, 4, 6], vec![1, 2, 4]),
        (vec![0], vec![1, 3]),
        (vec![], vec![2]),
        (vec![], vec![]),
    ] {
        let make = |keys: Vec<usize>, control: bool, flow: &mut Flow| {
            Tree::from_sorted(
                0,
                keys.into_iter()
                    .map(|local| {
                        (
                            local,
                            Cell {
                                data: Data::Initialized {
                                    version: None,
                                    domain: Some(Domain::boolean(control)),
                                    origin: None,
                                },
                                control,
                            },
                        )
                    })
                    .collect(),
                flow,
                Span::default(),
                MAX_EDGES,
            )
            .unwrap()
            .0
        };
        let left = make(a, false, &mut flow);
        let right = make(b, true, &mut flow);
        let (joined, _) = left
            .joined::<false>(&right, &mut flow, Span::default(), MAX_EDGES)
            .unwrap();
        let expected = left
            .keys()
            .chain(right.keys())
            .map(|&local| {
                (
                    local,
                    Cell::join::<false>(
                        left.get(&local),
                        right.get(&local),
                        &mut flow,
                        Span::default(),
                    )
                    .unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            joined
                .into_iter()
                .map(|(&id, cell)| (id, cell.clone()))
                .collect::<BTreeMap<_, _>>(),
            expected
        );
        assert_eq!(
            joined,
            right
                .joined::<false>(&left, &mut flow, Span::default(), MAX_EDGES)
                .unwrap()
                .0
        );
    }
}

#[test]
pub(crate) fn scalar_cell_joins_bound_new_paths_and_reject_foreign_owners_atomically() {
    let mut flow = Flow::default();
    let (tree, _) = Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    let (next, _) = tree
        .updated(
            6,
            &Cell {
                data: Data::Partial,
                control: false,
            },
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let before = format!("{tree:?}{next:?}");
    let start = flow.work;
    let (expected, used) = tree
        .joined::<false>(&next, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = flow.work - start;
    for (limit, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = tree.joined::<false>(&next, &mut flow, Span::default(), limit);
        assert_eq!(result.is_ok(), pass);
        if let Ok((actual, parts)) = result {
            assert_eq!(actual, expected);
            assert_eq!(parts, used);
        }
        assert_eq!(format!("{tree:?}{next:?}"), before);
    }
    flow.work = 0;
    flow.full = false;
    assert!(
        tree.joined::<false>(&Tree::new(8), &mut flow, Span::default(), MAX_EDGES)
            .is_err()
    );
}
