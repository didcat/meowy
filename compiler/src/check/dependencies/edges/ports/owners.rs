use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn edge_owners_reject_cross_function_links_without_mutating_inventory() {
    let mut checker = check("x:1;f<int32>:(v<int32>){->v+1}");
    let root = checker
        .points
        .iter()
        .position(|point| point.complete && point.owner == 0)
        .unwrap();
    let nested = checker
        .points
        .iter()
        .position(|point| point.complete && point.owner != 0)
        .unwrap();
    let edge = Edge::new(Port::Normal(root), Port::Entry(nested), Route::Next);
    let edges = vec![(Family::Region, edge)];
    let counts = checker.edge_counts();
    let error = checker
        .validate_edge_ports(&edges, Span::default())
        .unwrap_err();
    assert!(error.message.contains("owner mismatch"));
    assert_eq!(edges, [(Family::Region, edge)]);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn edge_owners_preserve_nested_targets_duplicates_and_conditional_routes() {
    let source = "flag:false;'outer{'inner{|flag|'outer.leave();|!flag|'inner.restart()}};f<int32>:(v<int32>){->v};x:f(1)";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let edges = checker.edge_inventory(Span::default()).unwrap();
    checker
        .validate_edge_ports(&edges, Span::default())
        .unwrap();
    let duplicate = vec![edges[0], edges[0]];
    checker
        .validate_edge_ports(&duplicate, Span::default())
        .unwrap();
    assert_eq!(duplicate.len(), 2);
    for route in [
        Route::True,
        Route::False,
        Route::Backedge,
        Route::Returned,
        Route::Exit,
    ] {
        assert!(edges.iter().any(|(_, edge)| edge.route == route));
    }
    let (&emit, &(source, _)) = checker.emission_sources.first_key_value().unwrap();
    checker.emissions.get_mut(&source).unwrap().owner += 100;
    assert!(
        checker
            .port_owner(Port::Emission(emit), Span::default())
            .is_err()
    );
}

#[test]
pub(crate) fn edge_owner_validation_rejects_excess_work_without_partial_results() {
    let mut checker = check("x:1");
    let edges = checker.edge_inventory(Span::default()).unwrap();
    let counts = checker.edge_counts();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - edges.len() - 1;
    assert!(
        checker
            .validate_edge_ports(&edges, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
    let large = vec![edges[0]; MAX_EDGES + 1];
    assert!(
        Checker::new()
            .validate_edge_ports(&large, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
