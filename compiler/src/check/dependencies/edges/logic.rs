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
        let stmt = checker.points[query.point].parent.unwrap();
        let expr = checker.points[stmt].parent.unwrap();
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

#[test]
pub(crate) fn logic_contents_link_skipped_rhs_expression_roots_and_erased_queries() {
    for (left, op) in [("false", "&&"), ("true", "||")] {
        let source =
            format!("p:@\"proof\";n:3;x:{left}{op}{{q:p.can_copy<({{-><uint8[n]>}})>();->true}}");
        let checker = check(&source);
        assert_eq!(checker.region_edges.len(), 2);
        let query = &checker.queries[0];
        let stmt = checker.points[query.point].parent.unwrap();
        let root = checker.points[stmt].parent.unwrap();
        let region = checker.points[root].parent.unwrap();
        let edges = checker.region_edges[&region];
        assert_eq!(
            edges[0],
            Edge::new(Port::Entry(region), Port::Entry(root), Route::Next)
        );
        assert_eq!(
            edges[1],
            Edge::new(Port::Normal(root), Port::Normal(region), Route::Next)
        );
        assert!(checker.points[root].complete);
        assert!(!edges.contains(&Edge::new(
            Port::Entry(region),
            Port::Normal(region),
            Route::Next
        )));
        let read = checker.body_inputs.values().flatten().next().unwrap();
        assert_eq!(
            read.root,
            checker.query_budgets[query.root].as_ref().unwrap().root
        );
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, "B001");
    }
}

#[test]
pub(crate) fn logic_contents_keep_group_roots_and_nested_function_owners() {
    let source = "x:((true&&false))||{f<boolean>:(){->true&&false};->true}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.region_edges.len(), 8);
    let outer = checker
        .points
        .iter()
        .position(|point| point.kind == PointKind::Or)
        .unwrap();
    let condition = id(checker.branch_edges[&outer][0].to);
    let root = id(checker.region_edges[&condition][0].to);
    assert_eq!(checker.points[root].kind, PointKind::Expr);
    assert_eq!(checker.points[root].parent, Some(condition));
    for edges in checker.region_edges.values() {
        let region = &checker.points[id(edges[0].from)];
        let child = &checker.points[id(edges[0].to)];
        assert_eq!(region.owner, child.owner);
        assert_eq!(region.block, child.block);
    }
}

#[test]
pub(crate) fn logic_contents_preserve_errors_and_unknown_call_completion() {
    let mut checker = Checker::new();
    let block = crate::parser::parse("true||1").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E222");
    assert!(checker.region_edges.is_empty());
    let mut checker = Checker::new();
    let block = crate::parser::parse("x<int32>:false&&true").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E207");
    assert_eq!(checker.region_edges.len(), 2);
    let branch = *checker.branch_edges.keys().next().unwrap();
    assert!(!checker.points[branch].complete);
    assert!(checker.bodies.is_empty());
    let checker = check("f<boolean>:(){->true};x:false&&f()");
    let routes = checker.branch_edges.values().next().unwrap();
    let region = id(routes[1].to);
    let call = id(checker.region_edges[&region][0].to);
    assert!(
        !checker
            .region_edges
            .values()
            .flatten()
            .chain(checker.branch_edges.values().flatten())
            .any(|edge| { edge.from == Port::Entry(call) && edge.to == Port::Normal(call) })
    );
}
