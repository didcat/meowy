use super::*;

pub(self) fn context(source: &str) -> (Checker, Reports, state::Entries, ports::Ports, usize) {
    let (mut checker, program, reports) = super::super::versions::tests::context(source);
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    (checker, reports, entries, ports, parts)
}

#[test]
pub(crate) fn scalar_schedule_waits_for_long_diamond_predecessors_before_joins() {
    let (mut checker, reports, entries, ports, parts) =
        context("choose:(flag<boolean>){|flag|{a:1;b:2};tail:3};unused:(n<int32>){n}");
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let (orders, _) = checker
        .scalar_schedule(&graph, &entries, Span::default(), parts)
        .unwrap();
    let mut total = 0;
    let mut differs = false;
    for (&owner, order) in &orders {
        let ranks = order
            .iter()
            .enumerate()
            .map(|(rank, port)| (*port, rank))
            .collect::<HashMap<_, _>>();
        let walk = &reports.entries[&owner].1;
        let prior = walk
            .ports
            .iter()
            .enumerate()
            .map(|(rank, port)| (*port, rank))
            .collect::<HashMap<_, _>>();
        assert_eq!(order.first(), Some(&entries[&owner].0));
        for &port in order {
            assert_eq!(graph.nodes[&port].site.owner, owner);
            for input in &graph.nodes[&port].incoming {
                let edge = graph.edges[input.edge].1;
                if input.reached && edge.route != Route::Backedge {
                    assert!(ranks[&edge.from] < ranks[&port]);
                    differs |= prior[&edge.from] > prior[&port];
                }
            }
        }
        total += order.len();
    }
    assert!(differs);
    assert_eq!(total, graph.nodes.len());
}

#[test]
pub(crate) fn scalar_schedule_uses_source_ties_independent_of_walk_and_hash_order() {
    let (mut checker, mut reports, entries, ports, parts) =
        context("flag:false;'loop{|flag|{a:1;b:2};|flag|'loop.restart()};tail:3");
    reports.entries.get_mut(&0).unwrap().1.ports.reverse();
    let mut expected = None;
    for _ in 0..6 {
        let (graph, parts) = checker
            .scalar_graph(&reports, ports.clone(), Span::default(), parts)
            .unwrap();
        let start = checker.flow.work;
        let (orders, _) = checker
            .scalar_schedule(&graph, &entries, Span::default(), parts)
            .unwrap();
        let cost = checker.flow.work - start;
        let mut seen = BTreeSet::new();
        for &port in &orders[&0] {
            let ready = graph
                .nodes
                .iter()
                .filter(|(port, node)| {
                    !seen.contains(*port)
                        && node.incoming.iter().all(|input| {
                            let edge = graph.edges[input.edge].1;
                            !input.reached
                                || edge.route == Route::Backedge
                                || seen.contains(&edge.from)
                        })
                })
                .map(|(&port, node)| key(port, node.site))
                .min()
                .unwrap();
            assert_eq!(key(port, graph.nodes[&port].site), ready);
            seen.insert(port);
        }
        let actual = (orders, cost);
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
    }
}

#[test]
pub(crate) fn scalar_schedule_rejects_cycles_orphans_and_missing_entries() {
    for fault in 0..3 {
        let (mut checker, mut reports, mut entries, ports, parts) = context("x:1");
        let entry = entries[&0].0;
        if fault == 0 {
            reports.index.edges.push((
                Family::Region,
                Edge::new(Port::BlockResult(reports.entries[&0].0), entry, Route::Next),
            ));
        }
        let (mut graph, parts) = checker
            .scalar_graph(&reports, ports, Span::default(), parts)
            .unwrap();
        if fault == 1 {
            graph.nodes.insert(
                Port::Entry(usize::MAX),
                graph::Node {
                    site: graph.nodes[&entry].site,
                    incoming: Vec::new(),
                    outgoing: Vec::new(),
                },
            );
        } else if fault == 2 {
            entries.clear();
        }
        let before = format!("{graph:?}{entries:?}");
        assert_eq!(
            checker
                .scalar_schedule(&graph, &entries, Span::default(), parts)
                .unwrap_err()
                .code,
            "B001"
        );
        assert_eq!(format!("{graph:?}{entries:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_schedule_reserves_scratch_and_output_and_charges_exact_work() {
    let (mut checker, reports, entries, ports, parts) = context("flag:false;|flag|{x:1};tail:2");
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let expected = checker
        .scalar_schedule(&graph, &entries, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.1;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_schedule(&graph, &entries, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((orders, left)) = result {
            assert_eq!(orders, expected.0);
            assert_eq!(left, 0);
        }
    }
}
