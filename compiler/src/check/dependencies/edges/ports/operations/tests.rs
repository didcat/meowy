use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn operation_ports_keep_call_points_distinct_from_call_ids_and_list_endpoints() {
    let source = "f<int32>:(v<int32>){->v+1};x:f(1);xs:[x,2]";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let calls: Vec<_> = checker
        .invocations
        .values()
        .map(|call| (call.site, call.point, call.owner))
        .collect();
    let owners = checker.operation_port_owners(Span::default()).unwrap();
    assert!(calls.iter().any(|(site, point, _)| site != point));
    for (_, point, owner) in calls {
        assert_eq!(owners[&point], owner);
    }
    for (source, edges) in &checker.endpoints {
        if let crate::check::SequenceSource::Expr(id) = source
            && edges.iter().any(|edge| edge.to == Port::Operation(*id))
        {
            assert_eq!(owners[id], checker.points[*id].owner);
        }
    }
    let edges = checker.edge_inventory(Span::default()).unwrap();
    checker
        .validate_edge_ports(&edges, Span::default())
        .unwrap();
}

#[test]
pub(crate) fn operation_ports_reject_undeclared_stages_and_wrong_producer_owners() {
    let mut checker = check("x:(1)");
    let id = *checker.region_edges.first_key_value().unwrap().0;
    let edge = Edge::new(Port::Entry(id), Port::Operation(id), Route::Next);
    let counts = checker.edge_counts();
    assert!(
        checker
            .validate_edge_ports(&[(Family::Region, edge)], Span::default())
            .unwrap_err()
            .message
            .contains("operation-port identity")
    );
    assert_eq!(checker.edge_counts(), counts);
    let (&leaf, _) = checker.scalar_leaves.first_key_value().unwrap();
    checker.scalar_leaves.get_mut(&leaf).unwrap().owner += 100;
    assert!(
        checker
            .operation_port_owners(Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
}

#[test]
pub(crate) fn operation_ports_charge_scan_work_without_mutating_producers() {
    let mut checker = check("n:1;f<int32>:(v<int32>){->v};x:f(n)");
    let counts = checker.edge_counts();
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .operation_port_owners(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.edge_counts(), counts);
    let mut checker = Checker::new();
    checker
        .endpoints
        .insert(crate::check::SequenceSource::Block(0), Vec::new());
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .operation_port_owners(Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.endpoints.len(), 1);
}
