use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_exits_retain_nested_targets_aliases_and_separate_function_owners() {
    let (mut checker, _, _) =
        context("'out{stop:'out.leave;'inner{stop()}};f:()'out{'out.leave()}");
    let (sources, left) = checker
        .scalar_exit_sources(Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(sources.leaves.len(), 2);
    assert_eq!(sources.targets.len(), 2);
    assert_eq!(MAX_EDGES - left, 4);
    assert!(sources.unknown.is_empty() && sources.incomplete.is_empty());
    for (&point, leave) in &sources.leaves {
        assert_eq!(
            checker.scope_exits[&point].edge.to,
            Port::Leave(leave.target)
        );
        assert_eq!(checker.points[point].block, Some(leave.block));
        assert_eq!(checker.bodies[&leave.target].owner, leave.owner);
    }
    let owners = sources
        .leaves
        .values()
        .map(|leave| leave.owner)
        .collect::<HashSet<_>>();
    assert_eq!(owners, HashSet::from([0, 1]));
}

#[test]
pub(crate) fn scalar_exits_keep_missing_sources_and_nested_bodies_incomplete() {
    let (mut checker, _, _) = context("'out{'inner{'out.leave()}}");
    let (&point, exit) = checker.scope_exits.first_key_value().unwrap();
    let Port::Leave(target) = exit.edge.to else {
        panic!()
    };
    let block = checker.points[point].block.unwrap();
    let body = checker.bodies.get_mut(&block).unwrap();
    let index = body
        .sources
        .iter()
        .position(|source| *source == Some(point))
        .unwrap();
    body.sources[index] = None;
    let (sources, _) = checker
        .scalar_exit_sources(Span::default(), MAX_EDGES)
        .unwrap();
    assert!(sources.leaves.is_empty());
    assert_eq!(sources.unknown, HashSet::from([target]));
    checker.bodies.remove(&block);
    let (sources, _) = checker
        .scalar_exit_sources(Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(sources.incomplete, HashSet::from([0]));
}

#[test]
pub(crate) fn scalar_exits_reject_stale_owner_source_target_and_inventory_evidence() {
    for fault in 0..5 {
        let (mut checker, _, _) = context("'out{'out.leave()};f:(){}");
        let point = *checker.scope_exits.first_key_value().unwrap().0;
        let block = checker.points[point].block.unwrap();
        match fault {
            0 => checker.scope_exits.get_mut(&point).unwrap().owner += 1,
            1 => checker.points[point].complete = false,
            2 => checker.scope_exits.get_mut(&point).unwrap().edge.to = Port::Leave(usize::MAX),
            3 => {
                checker.scope_exits.remove(&point);
            }
            4 => {
                let body = checker.bodies.get_mut(&block).unwrap();
                body.facts.clear();
                body.sources.clear();
            }
            _ => unreachable!(),
        }
        let before = format!("{:?}{:?}", checker.bodies, checker.scope_exits);
        assert!(
            checker
                .scalar_exit_sources(Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(
            format!("{:?}{:?}", checker.bodies, checker.scope_exits),
            before
        );
    }
}
