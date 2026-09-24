use super::*;

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    checker.block(&block, None, None).unwrap();
    checker
}

pub(crate) fn id(port: Port) -> PointId {
    match port {
        Port::Entry(id) | Port::Normal(id) | Port::Operation(id) => id,
        Port::Leave(_)
        | Port::Restart { .. }
        | Port::BlockEntry(_)
        | Port::BlockNormal(_)
        | Port::BlockResult(_) => panic!("point port expected"),
    }
}

#[test]
pub(crate) fn matcher_edges_link_erased_uses_to_independent_decisions_and_normal_joins() {
    let checker =
        check("p:@\"proof\";n:3;|false|q:p.can_copy<({-><uint8[n]>})>();|true|<T>:{-><uint8[n]>}");
    assert_eq!(checker.branch_edges.len(), 2);
    let query = &checker.points[checker.queries[0].point];
    let body = query.parent.unwrap();
    let taken = checker.points[body].parent.unwrap();
    let branch = checker.points[taken].parent.unwrap();
    let edges = &checker.branch_edges[&branch];
    assert!(
        edges
            .iter()
            .any(|edge| edge.route == Route::True && edge.to == Port::Entry(taken))
    );
    assert!(edges.contains(&Edge::new(
        Port::Normal(taken),
        Port::Normal(branch),
        Route::Join
    )));
    let skipped = id(edges
        .iter()
        .find(|edge| edge.route == Route::False)
        .unwrap()
        .to);
    assert!(edges.contains(&Edge::new(
        Port::Entry(skipped),
        Port::Normal(skipped),
        Route::Next
    )));
    let other = *checker
        .branch_edges
        .keys()
        .find(|id| **id != branch)
        .unwrap();
    assert!(!edges.iter().any(|edge| edge.to == Port::Entry(other)));
    assert_eq!(checker.points[skipped].kind, PointKind::Else);
}

#[test]
pub(crate) fn matcher_edges_do_not_claim_taken_path_completion_after_leaves() {
    let source = "'out{|true|'out.leave()};after:1";
    crate::compile(source).unwrap();
    let checker = check(source);
    let edges = checker.branch_edges.values().next().unwrap();
    let taken = id(edges
        .iter()
        .find(|edge| edge.route == Route::True)
        .unwrap()
        .to);
    assert!(
        !edges
            .iter()
            .any(|edge| edge.from == Port::Entry(taken) && edge.to == Port::Normal(taken))
    );
    let checker = check("|false|{f:(){|true|0}};|true|1");
    assert_eq!(checker.branch_edges.len(), 3);
    for (branch, edges) in &checker.branch_edges {
        for edge in edges {
            assert_eq!(
                checker.points[id(edge.from)].owner,
                checker.points[*branch].owner
            );
            assert_eq!(
                checker.points[id(edge.to)].owner,
                checker.points[*branch].owner
            );
        }
    }
}

#[test]
pub(crate) fn matcher_edges_preserve_errors_and_reject_invalid_region_identity() {
    for (source, code) in [("|1|0", "E215"), ("|true|x<boolean>:1", "E207")] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(checker.branch_edges.is_empty());
        assert!(checker.point.is_none());
    }
    for case in 0..4 {
        let mut checker = check("|true|1");
        let (&branch, &edges) = checker.branch_edges.first_key_value().unwrap();
        let condition = id(edges[0].to);
        let taken = id(edges[1].to);
        let skipped = id(edges[2].to);
        let span = checker.points[branch].span;
        checker
            .branch_edges(branch, condition, taken, skipped, span)
            .unwrap();
        assert_eq!(checker.branch_edges.len(), 1);
        match case {
            0 => checker.points[taken].complete = false,
            1 => checker.points[taken].parent = None,
            2 => checker.points[taken].owner += 1,
            3 => checker.points[taken].block = None,
            _ => unreachable!(),
        }
        let error = checker
            .branch_edges(branch, condition, taken, skipped, span)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(checker.branch_edges[&branch], edges);
    }
}

#[test]
pub(crate) fn matcher_edges_bound_atomic_registration_without_spending_logical_steps() {
    for capacity in [false, true] {
        let mut checker = check("|true|1");
        let (&branch, &edges) = checker.branch_edges.first_key_value().unwrap();
        checker.branch_edges.clear();
        if capacity {
            for id in 1..=MAX_EDGES / edges.len() {
                checker
                    .branch_edges
                    .insert(checker.points.len() + id, edges);
            }
        } else {
            assert!(!checker.flow.spend(usize::MAX));
        }
        let count = checker.branch_edges.len();
        let error = checker
            .branch_edges(
                branch,
                id(edges[0].to),
                id(edges[1].to),
                id(edges[2].to),
                checker.points[branch].span,
            )
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("edge budget"));
        assert_eq!(checker.branch_edges.len(), count);
        assert!(!checker.branch_edges.contains_key(&branch));
        assert!(checker.type_work.is_none());
    }
}
