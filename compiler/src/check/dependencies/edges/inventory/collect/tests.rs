use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn edge_inventory_retains_duplicates_and_exact_routes_without_new_links() {
    let mut checker = Checker::new();
    let edge = Edge::new(Port::Entry(1), Port::Normal(2), Route::Returned);
    let key = crate::check::SequenceSource::Expr(0);
    checker.endpoints.insert(key, vec![edge, edge]);
    checker.endpoint_edges = 2;
    let before = checker.edge_counts();
    let inventory = checker.edge_inventory(Span::default()).unwrap();
    assert_eq!(
        inventory,
        [(Family::Endpoint, edge), (Family::Endpoint, edge)]
    );
    assert_eq!(checker.edge_counts(), before);
    assert_eq!(checker.endpoints[&key], [edge, edge]);
}

#[test]
pub(crate) fn edge_inventory_preserves_checked_conditional_restart_and_call_edges() {
    let source = "d:@\"debug\";n:=0;'loop{|n==1|'loop.leave();n=n+1;|n<2|'loop.restart()};f<int32>:(n<int32>){->n+1};x:f(2);d.print(x)";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let counts = checker.edge_counts();
    let inventory = checker.edge_inventory(Span::default()).unwrap();
    assert_eq!(
        inventory.len(),
        counts.iter().map(|(_, count)| count).sum::<usize>()
    );
    for route in [
        Route::True,
        Route::False,
        Route::Backedge,
        Route::Returned,
        Route::Checked,
        Route::Exit,
    ] {
        assert!(
            inventory.iter().any(|(_, edge)| edge.route == route),
            "{route:?}"
        );
    }
    for edge in checker.restart_edges.values() {
        assert!(inventory.contains(&(Family::Restart, *edge)));
    }
}

#[test]
pub(crate) fn edge_inventory_rejects_counter_mismatches_and_bounds_without_mutating_ledgers() {
    for count in [0, 2, MAX_EDGES + 1, usize::MAX] {
        let mut checker = Checker::new();
        let edge = Edge::new(Port::Entry(0), Port::Normal(0), Route::Next);
        let key = crate::check::SequenceSource::Expr(0);
        checker.endpoints.insert(key, vec![edge]);
        checker.endpoint_edges = count;
        let error = checker.edge_inventory(Span::default()).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains(if count <= MAX_EDGES {
            "count mismatch"
        } else {
            "budget"
        }));
        assert_eq!(checker.endpoint_edges, count);
        assert_eq!(checker.endpoints[&key], [edge]);
    }
    let mut checker = check("x:1");
    let counts = checker.edge_counts();
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .edge_inventory(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
    assert!(checker.flow.exceeded());
}
