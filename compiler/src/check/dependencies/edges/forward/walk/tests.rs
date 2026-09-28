use super::*;

pub(super) fn index(edges: &[Edge]) -> ForwardIndex {
    ForwardIndex::build(
        edges.iter().map(|edge| (Family::Region, *edge)).collect(),
        &mut crate::flow::Flow::new(),
        Span::default(),
    )
    .unwrap()
}

#[test]
pub(crate) fn structural_walk_preserves_duplicate_edges_and_stops_forward_cycles() {
    let a = Port::Entry(0);
    let b = Port::Normal(0);
    let graph = index(&[
        Edge::new(a, b, Route::Checked),
        Edge::new(a, b, Route::Checked),
        Edge::new(b, a, Route::Returned),
        Edge::new(b, b, Route::Next),
    ]);
    let walk = graph
        .walk(a, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [a, b]);
    assert_eq!(walk.forward, [0, 1, 2, 3]);
    assert!(walk.backedges.is_empty());
    assert!(walk.missing.is_empty());
    assert_eq!(graph.edges[walk.forward[0]].1.route, Route::Checked);
    assert_eq!(graph.edges[walk.forward[2]].1.route, Route::Returned);
}

#[test]
pub(crate) fn structural_walk_reports_backedges_without_visiting_their_targets() {
    let a = Port::Entry(0);
    let b = Port::BlockEntry(1);
    let c = Port::Normal(2);
    let graph = index(&[
        Edge::new(a, b, Route::Backedge),
        Edge::new(a, c, Route::Next),
        Edge::new(b, Port::Normal(1), Route::Next),
        Edge::new(c, c, Route::Backedge),
    ]);
    let walk = graph
        .walk(a, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [a, c]);
    assert_eq!(walk.forward, [1]);
    assert_eq!(walk.backedges, [0, 3]);
    assert!(walk.missing.is_empty());
}

#[test]
pub(crate) fn structural_walk_reports_missing_sources_including_an_unknown_seed() {
    let a = Port::Entry(0);
    let b = Port::Normal(0);
    let graph = index(&[Edge::new(a, b, Route::Next)]);
    let walk = graph
        .walk(a, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [a, b]);
    assert_eq!(walk.missing, [b]);
    let unknown = Port::Entry(usize::MAX);
    let walk = graph
        .walk(unknown, &mut crate::flow::Flow::new(), Span::default())
        .unwrap();
    assert_eq!(walk.ports, [unknown]);
    assert_eq!(walk.missing, [unknown]);
    assert!(walk.forward.is_empty());
    assert!(walk.backedges.is_empty());
}
