use super::*;

#[test]
pub(crate) fn edge_family_counts_keep_fixed_maps_and_retained_counters_distinct() {
    let mut checker = Checker::new();
    let edge = Edge::new(Port::Entry(0), Port::Normal(0), Route::Next);
    checker.branch_edges.insert(0, [edge; 6]);
    checker.region_edges.insert(0, [edge; 2]);
    checker.restart_edges.insert(0, edge);
    checker.scalar_leaf_edges = 4;
    checker.heap_leaf_edges = 2;
    checker.sequence_edges = 3;
    let counts = checker.edge_counts();
    for (index, (family, _)) in counts.iter().enumerate() {
        assert_eq!(*family as usize, index);
    }
    assert_eq!(counts[Family::Branch as usize].1, 6);
    assert_eq!(counts[Family::Region as usize].1, 2);
    assert_eq!(counts[Family::Restart as usize].1, 1);
    assert_eq!(counts[Family::ScalarLeaf as usize].1, 4);
    assert_eq!(counts[Family::HeapLeaf as usize].1, 2);
    assert_eq!(counts[Family::Sequence as usize].1, 3);
    assert_eq!(counts.iter().map(|(_, count)| count).sum::<usize>(), 18);
}

#[test]
pub(crate) fn edge_family_counts_preserve_exact_capacity_and_overflow_rejection() {
    let mut checker = Checker::new();
    checker.local_read_edges = MAX_EDGES - 2;
    checker.scalar_leaf_edges = 2;
    assert!(checker.edge_room(0));
    assert!(!checker.edge_room(1));
    checker.heap_leaf_edges = 1;
    assert!(!checker.edge_room(0));
    checker.local_read_edges = usize::MAX;
    assert!(!checker.edge_room(0));
    checker.scalar_leaf_edges = 0;
    checker.heap_leaf_edges = 0;
    assert!(!checker.edge_room(1));
    let checker = Checker::new();
    assert!(checker.edge_room(MAX_EDGES));
    assert!(!checker.edge_room(MAX_EDGES + 1));
}

#[test]
pub(crate) fn edge_family_counts_cover_checked_reads_lists_borrows_and_outputs() {
    let source = "d:@\"debug\";xs<int32[2]>:=[1];view:&xs;v:view[1];xs=xs.add(2);d.print(v)";
    crate::compile(source).unwrap();
    let checker = super::super::tests::check(source);
    let counts = checker.edge_counts();
    for family in [
        Family::Storage,
        Family::LocalRead,
        Family::ScalarLeaf,
        Family::Index,
        Family::Method,
        Family::PlaceBorrow,
        Family::Output,
        Family::Narrowing,
    ] {
        assert!(counts[family as usize].1 > 0, "{family:?}");
    }
    let count: usize = counts.iter().map(|(_, count)| count).sum();
    assert!(checker.edge_room(MAX_EDGES - count));
    assert!(!checker.edge_room(MAX_EDGES - count + 1));
}
