use super::super::tests::checked;
use super::*;

pub(super) fn graph(count: usize, edges: &[(usize, usize)]) -> CallGraph {
    let nodes = (0..count)
        .map(|owner| {
            (
                owner,
                Node {
                    body: owner,
                    targets: BTreeMap::new(),
                    missing: Vec::new(),
                    backedges: Vec::new(),
                },
            )
        })
        .collect();
    let mut graph = CallGraph {
        nodes,
        sites: BTreeMap::new(),
    };
    for (site, &(caller, callee)) in edges.iter().enumerate() {
        graph
            .nodes
            .get_mut(&caller)
            .unwrap()
            .targets
            .entry(callee)
            .or_default()
            .push(site);
        graph.sites.insert(
            site,
            Call {
                point: site,
                caller,
                callee,
            },
        );
    }
    graph
}

#[test]
pub(crate) fn call_components_group_mutual_self_and_unused_functions_by_actual_owner() {
    let source = "f<()->int32>;g<()->int32>;g<int32>:(){->f()};f<int32>:(){alias:g;->alias()};again<int32>:(){->again()};unused<int32>:(){->1};x:1";
    crate::compile(source).unwrap();
    let (_, reports) = checked(source);
    let members: Vec<_> = reports
        .groups
        .groups
        .iter()
        .map(|group| group.members.clone())
        .collect();
    assert_eq!(members, [vec![0], vec![1, 2], vec![3], vec![4]]);
    assert_eq!(
        reports
            .groups
            .groups
            .iter()
            .map(|group| group.recursive)
            .collect::<Vec<_>>(),
        [false, true, true, false]
    );
    for (id, group) in reports.groups.groups.iter().enumerate() {
        for owner in &group.members {
            assert_eq!(reports.groups.owners[owner], id);
        }
    }
    let (_, empty) = checked("");
    assert_eq!(empty.groups.groups[0].members, [0]);
    assert!(!empty.groups.groups[0].recursive);
    let (_, looped) = checked("stop<never>:(){'loop{'loop.restart()}}");
    assert!(!looped.calls.nodes[&1].backedges.is_empty());
    assert!(!looped.groups.groups[1].recursive);
}

#[test]
pub(crate) fn call_components_keep_cross_edges_outside_cycles_and_parallel_sites() {
    let graph = graph(5, &[(0, 1), (0, 2), (1, 2), (1, 2), (2, 3), (3, 2)]);
    let result = graph.components(&mut Flow::new(), Span::default()).unwrap();
    assert_eq!(
        result
            .groups
            .iter()
            .map(|group| group.members.clone())
            .collect::<Vec<_>>(),
        [vec![0], vec![1], vec![2, 3], vec![4]]
    );
    assert_eq!(graph.nodes[&1].targets[&2].len(), 2);
}

#[test]
pub(crate) fn call_components_reject_inconsistent_adjacency_and_site_membership() {
    for fault in 0..5 {
        let mut graph = graph(3, &[(0, 1), (1, 2)]);
        match fault {
            0 => {
                graph.nodes.remove(&2);
            }
            1 => graph
                .nodes
                .get_mut(&0)
                .unwrap()
                .targets
                .get_mut(&1)
                .unwrap()
                .clear(),
            2 => graph
                .nodes
                .get_mut(&0)
                .unwrap()
                .targets
                .get_mut(&1)
                .unwrap()
                .push(0),
            3 => graph.sites.get_mut(&0).unwrap().caller = 1,
            4 => {
                graph.nodes.get_mut(&1).unwrap().targets.clear();
            }
            _ => unreachable!(),
        }
        let error = graph
            .components(&mut Flow::new(), Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
    }
}

#[test]
pub(crate) fn call_components_bound_deep_graphs_and_exact_work_without_recursion() {
    let edges: Vec<_> = (0..2047).map(|id| (id, id + 1)).collect();
    let graph = graph(2048, &edges);
    let mut flow = Flow::new();
    let expected = graph.components(&mut flow, Span::default()).unwrap();
    let work = flow.work;
    assert_eq!(expected.groups.len(), 2048);
    assert!(expected.groups.iter().all(|group| !group.recursive));
    for (nodes, sites) in [(2047, 2047), (2048, 2046)] {
        assert!(
            graph
                .components_limited(&mut Flow::new(), Span::default(), nodes, sites)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(
        graph
            .components_limited(&mut Flow::new(), Span::default(), 2048, 2047)
            .unwrap(),
        expected
    );
    for spare in [0, 1] {
        let mut flow = Flow::new();
        flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = graph.components(&mut flow, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(result) = result {
            assert_eq!(result, expected);
        }
    }
    assert_eq!(graph.sites.len(), 2047);
}

#[test]
pub(crate) fn call_components_match_reachability_for_every_three_owner_graph() {
    for mask in 0..512 {
        let edges: Vec<_> = (0..9)
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| (bit / 3, bit % 3))
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
        let result = graph(3, &edges)
            .components(&mut Flow::new(), Span::default())
            .unwrap();
        for (from, row) in reach.iter().enumerate() {
            assert_eq!(
                result.groups[result.owners[&from]].recursive, row[from],
                "mask {mask}"
            );
            for (to, reverse) in reach.iter().enumerate() {
                assert_eq!(
                    result.owners[&from] == result.owners[&to],
                    from == to || row[to] && reverse[from],
                    "mask {mask}"
                );
            }
        }
        let reversed: Vec<_> = edges.into_iter().rev().collect();
        assert_eq!(
            graph(3, &reversed)
                .components(&mut Flow::new(), Span::default())
                .unwrap(),
            result
        );
    }
}
