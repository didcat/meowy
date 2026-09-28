use super::*;
use crate::check::dependencies::edges::tests::check;

#[test]
pub(crate) fn forward_index_preserves_original_positions_duplicates_and_backedges() {
    let from = Port::Entry(7);
    let edge = Edge::new(from, Port::Normal(8), Route::Checked);
    let back = Edge::new(from, Port::BlockEntry(2), Route::Backedge);
    let edges = vec![
        (Family::Binary, edge),
        (Family::Restart, back),
        (Family::Binary, edge),
    ];
    let mut flow = crate::flow::Flow::new();
    let index = ForwardIndex::build(edges.clone(), &mut flow, Span::default()).unwrap();
    assert_eq!(index.edges, edges);
    assert_eq!(index.outgoing[&from].forward, [0, 2]);
    assert_eq!(index.outgoing[&from].backedges, [1]);
    assert!(!index.outgoing.contains_key(&Port::Normal(8)));
    assert!(!index.outgoing.contains_key(&Port::Entry(usize::MAX)));
}

#[test]
pub(crate) fn forward_index_keeps_exact_port_variants_and_conditional_routes() {
    let ports = [
        Port::Entry(0),
        Port::Normal(0),
        Port::Output { point: 0, part: 0 },
        Port::Output { point: 0, part: 1 },
        Port::Projection { point: 0, step: 0 },
        Port::Conversion { point: 0, part: 0 },
    ];
    let edges: Vec<_> = ports
        .iter()
        .enumerate()
        .map(|(id, port)| {
            (
                Family::Output,
                Edge::new(
                    *port,
                    Port::Normal(id + 1),
                    if id % 2 == 0 {
                        Route::True
                    } else {
                        Route::False
                    },
                ),
            )
        })
        .collect();
    let index = ForwardIndex::build(edges, &mut crate::flow::Flow::new(), Span::default()).unwrap();
    assert_eq!(index.outgoing.len(), ports.len());
    for (id, port) in ports.iter().enumerate() {
        assert_eq!(index.outgoing[port].forward, [id]);
        assert!(index.outgoing[port].backedges.is_empty());
    }
}

#[test]
pub(crate) fn forward_index_uses_validated_inventory_and_preserves_nested_restart_targets() {
    let source = "flag:false;'outer{'inner{|flag|'outer.restart();'inner.leave()}};f<int32>:(x<int32>){->x+1};x:f(2)";
    crate::compile(source).unwrap();
    let mut checker = check(source);
    let counts = checker.edge_counts();
    let index = checker.forward_index(Span::default()).unwrap();
    assert_eq!(checker.edge_counts(), counts);
    for (site, input) in &checker.restart_inputs {
        let port = Port::Restart {
            site: *site,
            target: input.target,
        };
        let links = &index.outgoing[&port];
        assert!(links.forward.is_empty());
        assert_eq!(links.backedges.len(), 1);
        let (_, edge) = index.edges[links.backedges[0]];
        assert_eq!(edge.to, Port::BlockEntry(input.target));
    }
    let id = *checker.scalar_leaves.first_key_value().unwrap().0;
    checker.points[id].complete = false;
    assert!(
        checker
            .forward_index(Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn forward_index_rejects_over_limit_input_and_mid_build_work_exhaustion() {
    let edge = (
        Family::Region,
        Edge::new(Port::Entry(0), Port::Normal(0), Route::Next),
    );
    let mut flow = crate::flow::Flow::new();
    assert!(
        ForwardIndex::build(vec![edge; MAX_EDGES + 1], &mut flow, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    let mut flow = crate::flow::Flow::new();
    flow.work = crate::flow::MAX_PROOF_WORK - 6;
    assert!(
        ForwardIndex::build(vec![edge; 2], &mut flow, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(flow.exceeded());
}
