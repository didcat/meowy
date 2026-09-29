use super::{super::tests::checked, *};

pub(super) const SOURCE: &str = "f<()->int32>;g<()->int32>;g<int32>:(){->f()};f<int32>:(){alias:g;->alias()};leaf<int32>:(){->1};left<int32>:(){->leaf()};right<int32>:(){alias:leaf;->alias()};a:left();b:right();c:leaf()";

#[test]
pub(crate) fn call_condensation_preserves_internal_and_cross_group_sites_once() {
    let (_, reports) = checked(SOURCE);
    let mut seen = BTreeSet::new();
    for (id, node) in reports.condensed.nodes.iter().enumerate() {
        for site in &node.internal {
            assert!(seen.insert(*site));
            let call = &reports.calls.sites[site];
            assert_eq!(reports.groups.owners[&call.caller], id);
            assert_eq!(reports.groups.owners[&call.callee], id);
        }
        for (target, sites) in &node.targets {
            assert_ne!(*target, id);
            for site in sites {
                assert!(seen.insert(*site));
                let call = &reports.calls.sites[site];
                assert_eq!(reports.groups.owners[&call.caller], id);
                assert_eq!(reports.groups.owners[&call.callee], *target);
            }
        }
    }
    assert_eq!(seen.len(), reports.calls.sites.len());
    assert_eq!(reports.condensed.nodes[1].internal.len(), 2);
    assert_eq!(reports.condensed.nodes[0].targets.len(), 3);
    let (_, empty) = checked("");
    assert_eq!(empty.condensed.nodes.len(), 1);
    assert!(empty.condensed.nodes[0].targets.is_empty());
    assert!(!empty.calls.nodes[&0].missing.is_empty());
    let (_, looped) = checked("f<never>:(){'loop{'loop.restart()}}");
    assert!(!looped.calls.nodes[&1].backedges.is_empty());
    assert!(looped.condensed.nodes[1].internal.is_empty());
}

#[test]
pub(crate) fn call_condensation_rejects_inconsistent_partition_and_call_membership() {
    for fault in 0..8 {
        let (_, mut reports) = checked(SOURCE);
        match fault {
            0 => {
                reports.groups.owners.remove(&0);
            }
            1 => reports.groups.groups[1].members.push(1),
            2 => reports.groups.groups[1].members.clear(),
            3 => {
                reports.groups.owners.insert(1, usize::MAX);
            }
            4 => reports.groups.groups[1].recursive = false,
            5 => reports.groups.groups.swap(1, 2),
            6 => reports.calls.sites.first_entry().unwrap().get_mut().caller = usize::MAX,
            7 => {
                reports.calls.nodes.get_mut(&0).unwrap().targets.clear();
            }
            _ => unreachable!(),
        }
        let error = reports
            .calls
            .condense(&reports.groups, &mut Flow::new(), Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001", "fault {fault}");
        assert!(error.message.contains("identity mismatch"), "fault {fault}");
    }
}

#[test]
pub(crate) fn call_condensation_bounds_partition_sites_and_exact_work() {
    let (_, reports) = checked(SOURCE);
    let graph = &reports.calls;
    let groups = &reports.groups;
    let mut flow = Flow::new();
    let expected = graph.condense(groups, &mut flow, Span::default()).unwrap();
    let work = flow.work;
    for (nodes, sites) in [
        (graph.nodes.len() - 1, graph.sites.len()),
        (graph.nodes.len(), graph.sites.len() - 1),
    ] {
        assert!(
            graph
                .condense_limited(groups, &mut Flow::new(), Span::default(), nodes, sites)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(
        graph
            .condense_limited(
                groups,
                &mut Flow::new(),
                Span::default(),
                graph.nodes.len(),
                graph.sites.len()
            )
            .unwrap(),
        expected
    );
    for spare in [0, 1] {
        let mut flow = Flow::new();
        flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = graph.condense(groups, &mut flow, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(result) = result {
            assert_eq!(result, expected);
        }
        assert_eq!(reports.condensed, expected);
    }
}
