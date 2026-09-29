use super::*;
use crate::check::dependencies::edges::forward::calls::tests::checked;

pub(super) fn graph(count: usize, edges: &[(usize, usize)]) -> Condensed {
    let mut graph = Condensed {
        nodes: (0..count).map(|_| Cluster::default()).collect(),
    };
    for (site, &(caller, callee)) in edges.iter().enumerate() {
        graph.nodes[caller]
            .targets
            .entry(callee)
            .or_default()
            .push(site);
    }
    graph
}

#[test]
pub(crate) fn call_order_places_callees_before_callers_with_stable_ties() {
    let mut graph = graph(5, &[(0, 1), (0, 2), (1, 3), (2, 3), (1, 3)]);
    graph.nodes[1].internal.push(50);
    graph.nodes[4].internal.push(60);
    assert_eq!(
        graph
            .analysis_order(&mut Flow::new(), Span::default())
            .unwrap(),
        [3, 1, 2, 0, 4]
    );
    let (_, reports) = checked(super::super::tests::SOURCE);
    let positions: BTreeMap<_, _> = reports
        .order
        .iter()
        .enumerate()
        .map(|(position, id)| (*id, position))
        .collect();
    assert_eq!(positions.len(), reports.groups.groups.len());
    for (id, node) in reports.condensed.nodes.iter().enumerate() {
        for target in node.targets.keys() {
            assert!(positions[target] < positions[&id]);
        }
    }
    let (_, empty) = checked("");
    assert_eq!(empty.order, [0]);
}

#[test]
pub(crate) fn call_order_rejects_cycles_invalid_targets_and_repeated_sites() {
    for fault in 0..5 {
        let mut graph = graph(3, &[(0, 1), (1, 2)]);
        match fault {
            0 => {
                graph.nodes[2].targets.insert(0, vec![2]);
            }
            1 => {
                graph.nodes[0].targets.insert(0, vec![2]);
            }
            2 => {
                graph.nodes[0].targets.insert(usize::MAX, vec![2]);
            }
            3 => {
                graph.nodes[0].targets.get_mut(&1).unwrap().clear();
            }
            4 => graph.nodes[0].internal.push(0),
            _ => unreachable!(),
        }
        assert_eq!(
            graph
                .analysis_order(&mut Flow::new(), Span::default())
                .unwrap_err()
                .code,
            "B001"
        );
    }
}

#[test]
pub(crate) fn call_order_matches_every_three_group_acyclicity_and_edge_constraint() {
    let pairs = [(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)];
    for mask in 0..64 {
        let edges: Vec<_> = pairs
            .iter()
            .enumerate()
            .filter_map(|(bit, pair)| (mask & (1 << bit) != 0).then_some(*pair))
            .collect();
        let mut reach = [[false; 3]; 3];
        for &(from, to) in &edges {
            reach[from][to] = true;
        }
        for via in 0..3 {
            for from in 0..3 {
                for to in 0..3 {
                    reach[from][to] |= reach[from][via] && reach[via][to];
                }
            }
        }
        let result = graph(3, &edges).analysis_order(&mut Flow::new(), Span::default());
        assert_eq!(
            result.is_ok(),
            (0..3).all(|id| !reach[id][id]),
            "mask {mask}"
        );
        if let Ok(order) = result {
            let positions: BTreeMap<_, _> = order
                .iter()
                .enumerate()
                .map(|(position, id)| (*id, position))
                .collect();
            assert_eq!(positions.len(), 3);
            for (from, to) in &edges {
                assert!(positions[to] < positions[from], "mask {mask}");
            }
            let reversed: Vec<_> = edges.into_iter().rev().collect();
            assert_eq!(
                graph(3, &reversed)
                    .analysis_order(&mut Flow::new(), Span::default())
                    .unwrap(),
                order
            );
        }
    }
}

#[test]
pub(crate) fn call_order_bounds_deep_graphs_sites_and_exact_work() {
    let edges: Vec<_> = (0..2047).map(|id| (id, id + 1)).collect();
    let graph = graph(2048, &edges);
    let mut flow = Flow::new();
    let expected = graph.analysis_order(&mut flow, Span::default()).unwrap();
    assert_eq!(expected, (0..2048).rev().collect::<Vec<_>>());
    let work = flow.work;
    for (nodes, sites) in [(2047, 2047), (2048, 2046)] {
        assert!(
            graph
                .order_limited(&mut Flow::new(), Span::default(), nodes, sites)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(
        graph
            .order_limited(&mut Flow::new(), Span::default(), 2048, 2047)
            .unwrap(),
        expected
    );
    for spare in [0, 1] {
        flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = graph.analysis_order(&mut flow, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(order) = result {
            assert_eq!(order, expected);
        }
    }
}
