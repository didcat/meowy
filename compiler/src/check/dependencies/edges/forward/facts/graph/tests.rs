use super::*;

pub(in super::super) fn context(source: &str) -> (Checker, Reports, ports::Ports, usize) {
    let (mut checker, program, reports) = super::super::versions::tests::context(source);
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    (checker, reports, ports, parts)
}

#[test]
pub(crate) fn scalar_graph_retains_unreached_predecessors_without_inventing_states() {
    let (mut checker, reports, ports, parts) = context("");
    let block = reports.entries[&0].0;
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    assert!(std::ptr::eq(graph.edges, reports.index.edges.as_slice()));
    let incoming = &graph.nodes[&Port::BlockNormal(block)].incoming;
    assert_eq!(incoming.len(), 2);
    assert_eq!(incoming.iter().filter(|input| input.reached).count(), 1);
    let unknown = incoming.iter().find(|input| !input.reached).unwrap();
    let edge = graph.edges[unknown.edge].1;
    assert_eq!(
        edge,
        Edge::new(Port::Leave(block), Port::BlockNormal(block), Route::Join)
    );
    assert_eq!(unknown.source, graph.external[&edge.from]);
    assert!(!graph.nodes.contains_key(&edge.from));
    assert!(graph.nodes[&Port::BlockResult(block)].outgoing.is_empty());
}

#[test]
pub(crate) fn scalar_graph_keeps_restart_boundaries_routes_and_frame_owners() {
    let (mut checker, reports, ports, parts) =
        context("flag:false;'loop{|flag|'loop.restart()};unused:(n<int32>){}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let mut backs = 0;
    for (&port, node) in &graph.nodes {
        for &position in &node.outgoing {
            let edge = graph.edges[position].1;
            assert_eq!(edge.from, port);
            let incoming = graph.nodes[&edge.to]
                .incoming
                .iter()
                .find(|input| input.edge == position)
                .unwrap();
            assert!(incoming.reached);
            assert_eq!(incoming.source, node.site);
            assert_eq!(graph.nodes[&edge.to].site.owner, node.site.owner);
            if edge.route == Route::Backedge {
                assert!(
                    matches!(port, Port::Restart { target, .. } if edge.to == Port::BlockEntry(target))
                );
                backs += 1;
            }
        }
    }
    assert_eq!(backs, 1);
    assert!(graph.nodes.values().any(|node| node.site.owner == 1));
}

#[test]
pub(crate) fn scalar_graph_rejects_missing_targets_foreign_edges_and_invalid_backedges() {
    for fault in 0..4 {
        let (mut checker, mut reports, mut ports, parts) = context("x:1;unused:(){}");
        let start = Port::BlockEntry(reports.entries[&0].0);
        let foreign = Port::BlockEntry(reports.entries[&1].0);
        match fault {
            0 => {
                ports.remove(
                    &reports
                        .index
                        .edges
                        .iter()
                        .find(|(_, edge)| edge.from == start)
                        .unwrap()
                        .1
                        .to,
                );
            }
            1 => reports
                .index
                .edges
                .push((Family::Region, Edge::new(start, foreign, Route::Next))),
            2 => reports
                .index
                .edges
                .push((Family::Restart, Edge::new(start, start, Route::Backedge))),
            3 => reports.index.edges.push((
                Family::Region,
                Edge::new(Port::Entry(usize::MAX), start, Route::Join),
            )),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}");
        assert_eq!(
            checker
                .scalar_graph(&reports, ports, Span::default(), parts)
                .unwrap_err()
                .code,
            "B001"
        );
        assert_eq!(format!("{reports:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_graph_bounds_work_and_shared_payload_without_partial_publication() {
    let (mut checker, reports, ports, parts) = context("flag:false;|flag|x:1");
    let start = checker.flow.work;
    let expected = checker
        .scalar_graph(&reports, ports.clone(), Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.1;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_graph(&reports, ports.clone(), Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((graph, left)) = result {
            assert_eq!(graph, expected.0);
            assert_eq!(left, 0);
        }
    }
}
