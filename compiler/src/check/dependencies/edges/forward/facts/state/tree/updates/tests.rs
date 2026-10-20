use super::super::tests::cells;
use super::*;

pub(super) fn cell() -> Cell {
    Cell {
        data: Data::Initialized {
            version: Some(Version {
                owner: 7,
                local: 6,
                definition: versions::Definition::Write(10),
            }),
            domain: Some(Domain::boolean(true)),
            origin: Some(Origin::Expression(7, 9)),
        },
        control: true,
    }
}

#[test]
pub(crate) fn scalar_cell_updates_share_unchanged_paths_and_keep_versions_independent() {
    let mut flow = Flow::default();
    let (tree, _) = Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    let (next, used) = tree
        .updated(6, &cell(), &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(next.owner(), 7);
    assert_eq!(used, tree.depth() + cell().weight());
    assert_eq!(tree.get(&6).unwrap().data, Data::Uninitialized);
    assert_eq!(next.get(&6), Some(&cell()));
    let (Kind::Branch(_, old), Kind::Branch(_, new)) = (
        &tree.root.as_ref().unwrap().kind,
        &next.root.as_ref().unwrap().kind,
    ) else {
        panic!()
    };
    assert!(Arc::ptr_eq(old, new));
    let mut write = cell();
    let Data::Initialized { version, .. } = &mut write.data else {
        panic!()
    };
    version.as_mut().unwrap().definition = versions::Definition::Write(11);
    let (last, cost) = next
        .updated(6, &write, &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(cost, used);
    assert_eq!(last.get(&6), Some(&write));
    assert_eq!(next.get(&6), Some(&cell()));
    let (same, cost) = next
        .updated(6, &cell(), &mut flow, Span::default(), 0)
        .unwrap();
    assert_eq!(cost, 0);
    assert!(Arc::ptr_eq(
        next.root.as_ref().unwrap(),
        same.root.as_ref().unwrap()
    ));
    let (added, _) = tree
        .updated(
            1,
            &Cell {
                data: Data::Unavailable,
                control: false,
            },
            &mut flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(added.len(), tree.len() + 1);
    assert_eq!(added.get(&1).unwrap().data, Data::Unavailable);
    assert!(tree.get(&1).is_none());
}

#[test]
pub(crate) fn scalar_cell_updates_bound_new_allocations_without_changing_older_roots() {
    let mut flow = Flow::default();
    let (tree, _) = Tree::from_sorted(7, cells(), &mut flow, Span::default(), MAX_EDGES).unwrap();
    let before = format!("{tree:?}");
    let start = flow.work;
    let (expected, used) = tree
        .updated(6, &cell(), &mut flow, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = flow.work - start;
    for (limit, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        flow.full = false;
        let result = tree.updated(6, &cell(), &mut flow, Span::default(), limit);
        assert_eq!(result.is_ok(), pass);
        if let Ok((next, parts)) = result {
            assert_eq!(next, expected);
            assert_eq!(parts, used);
        }
        assert_eq!(format!("{tree:?}"), before);
    }
}
