use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn reborrow_stages_keep_sites_and_parent_before_child_order_without_loads() {
    let source = "n:=1;p:&!n;q:&!*((p));r:&!*q;*r=2";
    crate::compile(source).unwrap();
    let (checker, block) = check(source);
    assert_eq!(checker.reborrows, 2);
    assert_eq!(checker.reborrow_ops.len(), 2);
    assert!(checker.derefs.is_empty());
    for stmt in &block.stmts[2..4] {
        let hir::Stmt::Bind { value, .. } = stmt else {
            panic!()
        };
        let hir::ExprKind::Reborrow { site, .. } = value.kind else {
            panic!()
        };
        let (&id, op) = checker
            .reborrow_ops
            .iter()
            .find(|(_, op)| op.site == Some(site))
            .unwrap();
        assert_eq!(op.mode, hir::ReferenceMode::Exclusive);
        assert_eq!(op.parent_mode, Some(hir::ReferenceMode::Exclusive));
        assert_eq!(op.span, value.span);
        assert_eq!(checker.points[op.parent].parent, Some(id));
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(op.parent), Route::Next),
                Edge::new(Port::Normal(op.parent), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
    }
    let source = "id<&!int32>:(p<&!int32>){->p};n:=1;q:&!*(id(&!n));*q=2";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let op = checker.reborrow_ops.values().next().unwrap();
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(
        checker.region_edges[&op.parent][0].to,
        Port::Entry(call.point)
    );
}

#[test]
pub(crate) fn reborrow_stages_preserve_stopped_parents_owners_and_other_reborrow_sites() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){&!*(stop())};f()"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    assert_eq!(op.site, None);
    assert_ne!(op.owner, 0);
    assert_eq!(checker.reborrows, 0);
    assert_eq!(
        op.edges,
        [Edge::new(
            Port::Entry(id),
            Port::Entry(op.parent),
            Route::Next
        )]
    );
    assert_eq!(op.span, checker.points[id].span);
    let source = "n:=1;p:&!n;shared<&int32>:&!*p";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.reborrows, 2);
    assert_eq!(checker.reborrow_ops.len(), 1);
    assert_eq!(checker.reborrow_ops.values().next().unwrap().site, Some(0));
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;n:=1;p:&!n;|flag|q:&!*p").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.reborrow_ops.values().next().unwrap().control);
}

#[test]
pub(crate) fn reborrow_stages_validate_existing_sites_and_publish_with_shared_budgets() {
    let (mut checker, block) = check("n:=1;p:&!n;q:&!*p");
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
    for case in 0..3 {
        let mut invalid = value.clone();
        let hir::ExprKind::Reborrow { site, fields, .. } = &mut invalid.kind else {
            panic!()
        };
        match case {
            0 => *site = checker.reborrows,
            1 => fields.push(0),
            2 => invalid.ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
            _ => unreachable!(),
        }
        assert!(
            checker
                .reborrow_operation(id, op.parent, op.mode, &invalid, op.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.points[op.parent].parent = None;
    assert!(
        checker
            .reborrow_operation(id, op.parent, op.mode, value, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[op.parent].parent = Some(id);
    checker.deref_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .reborrow_operation(id, op.parent, op.mode, value, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.reborrow_ops.is_empty());
    assert_eq!(checker.reborrow_edges, 0);
}
