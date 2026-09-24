use super::{
    tests::{check, id},
    *,
};

#[test]
pub(crate) fn logic_edges_route_erased_rhs_uses_on_the_correct_boolean_successor() {
    for (op, left, route) in [("&&", "false", Route::True), ("||", "true", Route::False)] {
        let source =
            format!("p:@\"proof\";n:3;x:{left}{op}{{q:p.can_copy<({{-><uint8[n]>}})>();->true}}");
        let checker = check(&source);
        let query = &checker.queries[0];
        let expr = checker.points[query.point].parent.unwrap();
        let arm = checker.points[expr].parent.unwrap();
        let branch = checker.points[arm].parent.unwrap();
        let edges = &checker.branch_edges[&branch];
        assert!(
            edges
                .iter()
                .any(|edge| edge.route == route && edge.to == Port::Entry(arm))
        );
        assert!(edges.contains(&Edge::new(
            Port::Normal(arm),
            Port::Normal(branch),
            Route::Join
        )));
        assert!(!edges.contains(&Edge::new(Port::Entry(arm), Port::Normal(arm), Route::Next)));
        let skipped = edges
            .iter()
            .find(|edge| matches!(edge.route, Route::True | Route::False) && edge.route != route)
            .unwrap();
        let skipped = id(skipped.to);
        assert!(edges.contains(&Edge::new(
            Port::Entry(skipped),
            Port::Normal(skipped),
            Route::Next
        )));
        let read = checker.body_inputs.values().flatten().next().unwrap();
        assert_eq!(checker.points[read.point].parent, Some(query.point));
        assert_eq!(
            read.root,
            checker.query_budgets[query.root].as_ref().unwrap().root
        );
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn logic_edges_keep_nested_decisions_and_function_owners_separate() {
    let source = "a:=false;b:=true;|a&&(b||false)|{f<boolean>:(){->true||false};x:1}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.branch_edges.len(), 4);
    for (branch, edges) in &checker.branch_edges {
        let owner = checker.points[*branch].owner;
        for edge in edges {
            assert_eq!(checker.points[id(edge.from)].owner, owner);
            assert_eq!(checker.points[id(edge.to)].owner, owner);
        }
        let join = edges
            .iter()
            .filter(|edge| edge.route == Route::Join)
            .collect::<Vec<_>>();
        assert_eq!(join.len(), 2);
        assert!(join.iter().all(|edge| edge.to == Port::Normal(*branch)));
    }
}

#[test]
pub(crate) fn logic_edges_preserve_failures_without_claiming_completed_regions() {
    for source in ["false&&1", "true||1"] {
        let mut checker = Checker::new();
        let block = crate::parser::parse(source).unwrap();
        assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E222");
        assert!(checker.branch_edges.is_empty());
        assert!(checker.point.is_none());
    }
    let mut checker = Checker::new();
    let block = crate::parser::parse("x<int32>:false&&true").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E207");
    let branch = *checker.branch_edges.keys().next().unwrap();
    assert!(!checker.points[branch].complete);
    assert!(checker.bodies.is_empty());
    assert!(checker.point.is_none());
    assert!(check("x:1+2;y:1==2").branch_edges.is_empty());
}
