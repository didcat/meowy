use super::super::{
    MAX_EDGES,
    tests::{check, id},
};
use super::*;

#[test]
pub(crate) fn region_edges_link_matcher_conditions_and_erased_statement_contents() {
    let checker =
        check("p:@\"proof\";n:3;|false|q:p.can_copy<({-><uint8[n]>})>();|true|<T>:{-><uint8[n]>}");
    assert_eq!(checker.region_edges.len(), 4);
    let query = &checker.points[checker.queries[0].point];
    let content = query.parent.unwrap();
    let region = checker.points[content].parent.unwrap();
    assert_eq!(
        checker.region_edges[&region],
        [
            Edge::new(Port::Entry(region), Port::Entry(content), Route::Next),
            Edge::new(Port::Normal(content), Port::Normal(region), Route::Next),
        ]
    );
    for edges in checker.region_edges.values() {
        let parent = id(edges[0].from);
        let child = id(edges[0].to);
        assert_eq!(checker.points[child].parent, Some(parent));
        assert!(checker.points[child].complete);
        assert!(!edges.contains(&Edge::new(
            Port::Entry(parent),
            Port::Normal(parent),
            Route::Next
        )));
    }
}

#[test]
pub(crate) fn region_edges_keep_exit_and_function_boundaries_without_invented_completion() {
    let source = "'out{|true|'out.leave()};f:(){|true|1}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.region_edges.len(), 4);
    for edges in checker.region_edges.values() {
        let child = id(edges[0].to);
        assert_eq!(
            checker.points[child].owner,
            checker.points[id(edges[0].from)].owner
        );
        assert!(
            !checker
                .region_edges
                .values()
                .flatten()
                .any(|edge| { edge.from == Port::Entry(child) && edge.to == Port::Normal(child) })
        );
    }
    for (source, code) in [("|1|0", "E215"), ("|true|x<boolean>:1", "E207")] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, code);
        assert!(checker.region_edges.is_empty());
        assert!(checker.point.is_none());
    }
}

#[test]
pub(crate) fn region_edges_reject_invalid_content_without_replacing_prior_links() {
    for case in 0..5 {
        let mut checker = check("|true|1");
        let (&region, &edges) = checker.region_edges.first_key_value().unwrap();
        let child = id(edges[0].to);
        let span = checker.points[region].span;
        checker.region_edges(region, child, span).unwrap();
        let count = checker.region_edges.len();
        match case {
            0 => checker.points[child].parent = None,
            1 => checker.points[child].owner += 1,
            2 => checker.points[child].block = None,
            3 => checker.points[child].complete = false,
            4 => checker.points[child].kind = PointKind::Read,
            _ => unreachable!(),
        }
        let error = checker.region_edges(region, child, span).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(checker.region_edges.len(), count);
        assert_eq!(checker.region_edges[&region], edges);
    }
}

#[test]
pub(crate) fn region_edges_share_branch_capacity_and_preserve_atomic_registration() {
    for capacity in [false, true] {
        let mut checker = check("|true|1");
        let (&region, &edges) = checker.region_edges.first_key_value().unwrap();
        let (&branch, &routes) = checker.branch_edges.first_key_value().unwrap();
        let span = checker.points[region].span;
        if capacity {
            checker.branch_edges.clear();
            checker.region_edges.clear();
            for id in 1..=MAX_EDGES / routes.len() {
                checker
                    .branch_edges
                    .insert(checker.points.len() + id, routes);
            }
            checker.region_edges.insert(usize::MAX, edges);
            checker.region_edges.insert(usize::MAX - 1, edges);
            assert!(checker.edge_room(0));
            assert!(!checker.edge_room(1));
        } else {
            assert!(!checker.flow.spend(usize::MAX));
        }
        let counts = (checker.branch_edges.len(), checker.region_edges.len());
        let error = checker
            .region_edges(region, id(edges[0].to), span)
            .unwrap_err();
        assert!(error.message.contains("region edge budget"));
        let error = checker
            .branch_edges(
                branch,
                id(routes[0].to),
                id(routes[1].to),
                id(routes[2].to),
                span,
            )
            .unwrap_err();
        assert!(error.message.contains("branch edge budget"));
        assert_eq!(
            (checker.branch_edges.len(), checker.region_edges.len()),
            counts
        );
    }
}
