use super::{tests::check, *};

#[test]
pub(crate) fn implicit_reborrow_stages_follow_raw_effects_without_result_bypasses() {
    for (source, site) in [
        ("n:=1;p:&!n;q<&int32>:p", 0),
        ("n:=1;q<&int32>:&!n", 0),
        ("n:=1;p:&!n;q<&int32>:&!*p", 1),
        ("id<&!int32>:(p<&!int32>){->p};n:=1;q<&int32>:id(&!n)", 0),
    ] {
        crate::compile(source).unwrap();
        let (checker, block) = check(source);
        let hir::Stmt::Bind { value, .. } = block.stmts.last().unwrap() else {
            panic!()
        };
        let hir::ExprKind::Reborrow { site: actual, .. } = value.kind else {
            panic!()
        };
        assert_eq!(site, actual);
        let (&id, op) = checker
            .reborrow_ops
            .iter()
            .find(|(_, op)| op.site == Some(site))
            .unwrap();
        assert_eq!(op.parent_mode, Some(hir::ReferenceMode::Exclusive));
        assert_eq!(op.mode, hir::ReferenceMode::Shared);
        assert_eq!(checker.points[op.parent].parent, Some(id));
        assert_eq!(op.span, checker.points[op.parent].span);
        assert!(!checker.region_edges.contains_key(&id));
        assert!(!checker.place_borrows.contains_key(&id));
        assert!(!checker.invocations.values().any(|call| call.point == id));
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(op.parent), Route::Next),
                Edge::new(Port::Normal(op.parent), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
        if let Some(call) = checker
            .invocations
            .values()
            .find(|call| call.point == op.parent)
        {
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
            assert_eq!(call.edges.last().unwrap().to, Port::Normal(op.parent));
        }
        if let Some(raw) = checker.reborrow_ops.get(&op.parent) {
            assert_eq!(raw.mode, hir::ReferenceMode::Exclusive);
            assert_eq!(raw.edges.last().unwrap().to, Port::Normal(op.parent));
        }
    }
}

#[test]
pub(crate) fn implicit_reborrow_stages_forward_groups_and_unchanged_shared_results() {
    let source = "n:=1;p:&!n;q<&int32>:((p))";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.reborrows, 1);
    assert_eq!(checker.reborrow_ops.len(), 1);
    let (&id, _) = checker.reborrow_ops.first_key_value().unwrap();
    let mut inner = id;
    let mut count = 0;
    while let Some(outer) = checker.points[inner].parent {
        if checker.points[outer].kind == PointKind::Stmt {
            break;
        }
        assert_eq!(
            checker.region_edges[&outer],
            [
                Edge::new(Port::Entry(outer), Port::Entry(inner), Route::Next),
                Edge::new(Port::Normal(inner), Port::Normal(outer), Route::Next),
            ]
        );
        inner = outer;
        count += 1;
    }
    assert_eq!(count, 4);
    let (checker, _) = check("n:1;r<&int32>:&n");
    assert!(checker.reborrow_ops.is_empty());
    let (&raw, _) = checker.place_borrows.first_key_value().unwrap();
    let outer = checker.points[raw].parent.unwrap();
    assert_eq!(checker.region_edges[&outer][1].from, Port::Normal(raw));
    assert_eq!(checker.region_edges[&outer][1].to, Port::Normal(outer));
    let (checker, _) = check("n:1;r:{->&n;->tag:0};q<&int32>:r");
    let outer = checker.operations.values().last().unwrap().input.unwrap();
    assert!(!checker.region_edges.contains_key(&outer));
    assert!(!checker.reborrow_ops.contains_key(&outer));
}

#[test]
pub(crate) fn implicit_reborrow_stages_keep_never_owners_and_control() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};q<&int32>:stop()"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    assert_eq!(op.site, None);
    assert_eq!(op.parent_mode, None);
    assert_eq!(op.mode, hir::ReferenceMode::Shared);
    assert_eq!(checker.reborrows, 0);
    assert_eq!(
        op.edges,
        [Edge::new(
            Port::Entry(id),
            Port::Entry(op.parent),
            Route::Next
        )]
    );
    assert!(!checker.region_edges.contains_key(&id));
    let call = checker
        .invocations
        .values()
        .find(|call| call.point == op.parent)
        .unwrap();
    assert!(!call.may_return);
    let source = "f<&int32>:(p<&!int32>){->p};flag:false;n:=1;p:&!n;|flag|q<&int32>:p";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(1);
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    assert!(checker.reborrow_ops.values().any(|op| op.owner != 0));
    assert!(
        checker
            .reborrow_ops
            .values()
            .any(|op| op.owner == 0 && op.control)
    );
}

#[test]
pub(crate) fn implicit_reborrow_stages_preserve_permissions_and_atomic_edge_budgets() {
    for (source, code) in [
        ("x:=1;p:&!x;s<&int32>:p;*p=2;v:*s", "E302"),
        ("n:=1;p:&!n;q<&boolean>:p", "E207"),
        ("p:@\"proof\";q:p.can_copy<uint8>();q.always", "B001"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let (mut checker, block) = check("n:=1;p:&!n;q<&int32>:p");
    let hir::Stmt::Bind { value, .. } = &block.stmts[2] else {
        panic!()
    };
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    let op = op.clone();
    let count = checker.reborrow_edges;
    checker
        .reborrow_operation(id, op.parent, op.mode, value, op.span)
        .unwrap();
    assert_eq!(checker.reborrow_edges, count);
    checker.reborrow_ops.clear();
    checker.reborrow_edges = 0;
    checker.temporary_borrow_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .reborrow_operation(id, op.parent, op.mode, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(checker.reborrow_edges, 0);
    assert!(checker.reborrow_ops.is_empty());
}
