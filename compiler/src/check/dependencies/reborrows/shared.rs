use super::{tests::check, *};

#[test]
pub(crate) fn shared_reborrow_stages_keep_parent_modes_shapes_and_no_load_order() {
    for (source, mode) in [
        ("n:1;p:&n;q:&*p", hir::ReferenceMode::Shared),
        ("n:=1;p:&!n;q:&*((p))", hir::ReferenceMode::Exclusive),
        ("r:{->n:1;->xs:[2]};p:&r;q:&*p", hir::ReferenceMode::Shared),
        ("xs:[1,2];p:&xs;q:&*p", hir::ReferenceMode::Shared),
        ("n:1;r:&n;p:&r;q:&*p", hir::ReferenceMode::Shared),
        (
            "id<&int32>:(p<&int32>){->p};n:1;q:&*(id(&n))",
            hir::ReferenceMode::Shared,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
        assert_eq!(op.mode, hir::ReferenceMode::Shared);
        assert_eq!(op.parent_mode, Some(mode));
        assert_eq!(op.site, Some(0));
        assert_eq!(op.span, checker.points[id].span);
        assert_eq!(
            op.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(op.parent), Route::Next),
                Edge::new(Port::Normal(op.parent), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
        assert!(checker.derefs.is_empty());
    }
    let (checker, _) = check("n:1;p:&n;q:&*(&*p)");
    assert_eq!(checker.reborrow_ops.len(), 2);
    assert_eq!(checker.reborrows, 2);
}

#[test]
pub(crate) fn shared_reborrow_stages_keep_stopped_modes_control_and_source_boundaries() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){&*(stop())};f()"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    assert_eq!(op.mode, hir::ReferenceMode::Shared);
    assert_eq!(op.parent_mode, None);
    assert_eq!(op.site, None);
    assert_ne!(op.owner, 0);
    assert_eq!(
        op.edges,
        [Edge::new(
            Port::Entry(id),
            Port::Entry(op.parent),
            Route::Next
        )]
    );
    assert_eq!(checker.reborrows, 0);
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;n:1;p:&n;|flag|q:&*p").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.reborrow_ops.values().next().unwrap().control);
    for source in ["r:{->n:1};p:&r;q:&(p.n)", "n:=1;p:&!n;q<&int32>:p"] {
        crate::compile(source).unwrap();
        assert!(check(source).0.reborrow_ops.is_empty());
    }
}

#[test]
pub(crate) fn shared_reborrow_stages_bound_shape_comparison_and_atomic_publication() {
    let (mut checker, block) = check("r:{->n:1};p:&r;q:&*p");
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
    let mut invalid = value.clone();
    invalid.ty = hir::Type::Reference(Box::new(hir::Type::Bool));
    assert!(
        checker
            .reborrow_operation(id, op.parent, op.mode, &invalid, op.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let fields = (0..=crate::borrow_value::MAX_PARTS)
        .map(|i| hir::Field {
            name: i.to_string(),
            ty: hir::Type::Bool,
            mutable: false,
        })
        .collect();
    invalid.ty = hir::Type::Reference(Box::new(hir::Type::Record {
        primary: Box::new(hir::Type::Null),
        fields,
    }));
    assert!(
        checker
            .reborrow_operation(id, op.parent, op.mode, &invalid, op.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
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
