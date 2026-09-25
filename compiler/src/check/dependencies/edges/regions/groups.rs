use super::super::tests::{check, id};
use super::*;

#[test]
pub(crate) fn group_links_connect_nested_calls_branches_and_contextual_values() {
    let source = "f<int32>:(){->1};x:((f()));y<boolean>:((false&&true));z<uint8>:((7))";
    crate::compile(source).unwrap();
    let checker = check(source);
    let groups = checker
        .region_edges
        .iter()
        .filter(|(id, _)| checker.points[**id].kind == PointKind::Expr)
        .collect::<Vec<_>>();
    assert_eq!(groups.len(), 6);
    for (&group, edges) in groups {
        let child = id(edges[0].to);
        assert_eq!(
            *edges,
            [
                Edge::new(Port::Entry(group), Port::Entry(child), Route::Next),
                Edge::new(Port::Normal(child), Port::Normal(group), Route::Next),
            ]
        );
        assert_eq!(checker.points[child].parent, Some(group));
        assert!(checker.points[child].complete);
    }
    let call = checker.invocations.values().next().unwrap().point;
    let inner = checker.points[call].parent.unwrap();
    let outer = checker.points[inner].parent.unwrap();
    assert_eq!(checker.region_edges[&inner][0].to, Port::Entry(call));
    assert_eq!(checker.region_edges[&outer][0].to, Port::Entry(inner));
    assert!(
        checker
            .operations
            .values()
            .any(|op| op.input == Some(outer))
    );
    assert_eq!(checker.invocations.len(), 1);
}

#[test]
pub(crate) fn group_links_preserve_nonreturning_children_and_function_ownership() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){((stop()))};((f()))"#;
    crate::compile(source).unwrap();
    let checker = check(source);
    for call in checker.invocations.values() {
        assert!(!call.may_return);
        assert!(
            !call
                .edges
                .iter()
                .any(|edge| edge.to == Port::Normal(call.point))
        );
        let group = checker.points[call.point].parent.unwrap();
        assert_eq!(checker.region_edges[&group][0].to, Port::Entry(call.point));
        assert_eq!(checker.points[group].owner, call.owner);
    }
    assert!(checker.invocations.values().any(|call| call.owner != 0));
    assert!(!checker.region_edges.values().flatten().any(|edge| {
        matches!((edge.from, edge.to), (Port::Entry(a), Port::Normal(b)) if a == b)
    }));
    let checker = check("'out{(({ 'out.leave() }))}");
    assert_eq!(checker.scope_exits.len(), 1);
    assert!(
        checker
            .region_edges
            .keys()
            .any(|id| checker.points[*id].kind == PointKind::Expr)
    );
}

#[test]
pub(crate) fn group_links_preserve_original_errors_and_restore_active_points() {
    for (source, code) in [
        ("x<boolean>:((1))", "E207"),
        ("x<uint8>:((256))", "E216"),
        ("((missing))", "E201"),
    ] {
        let mut checker = Checker::new();
        let error = checker
            .block(&crate::parser::parse(source).unwrap(), None, None)
            .unwrap_err();
        assert_eq!(error.code, code);
        assert!(checker.region_edges.is_empty());
        assert!(checker.point.is_none());
    }
    let source = "x:=1;p:&x;x=((2));n:*p";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
    crate::compile("x<uint8>:((7));p:&x;q:((p));y<uint8>:*q").unwrap();
}

#[test]
pub(crate) fn group_links_validate_children_and_share_the_edge_budget_atomically() {
    for case in 0..6 {
        let mut checker = check("x:((1))");
        let (&group, &edges) = checker
            .region_edges
            .iter()
            .find(|(id, _)| checker.points[**id].kind == PointKind::Expr)
            .unwrap();
        let child = id(edges[0].to);
        let span = checker.points[group].span;
        checker.region_edges(group, child, span).unwrap();
        match case {
            0 => checker.points[child].parent = None,
            1 => checker.points[child].owner += 1,
            2 => checker.points[child].block = None,
            3 => checker.points[child].complete = false,
            4 => checker.points[child].kind = PointKind::Stmt,
            5 => {
                checker.region_edges.remove(&group);
                checker.output_edges = super::super::MAX_EDGES;
            }
            _ => unreachable!(),
        }
        let count = checker.region_edges.len();
        let error = checker.region_edges(group, child, span).unwrap_err();
        assert!(
            error
                .message
                .contains(if case == 5 { "budget" } else { "identity" })
        );
        assert_eq!(checker.region_edges.len(), count);
        if case != 5 {
            assert_eq!(checker.region_edges[&group], edges);
        }
    }
}
