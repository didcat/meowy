use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn deref_stages_keep_modes_and_pointer_before_load_result_order() {
    for (source, mode) in [
        ("n:1;p:&n;x:*p", hir::ReferenceMode::Shared),
        ("n:=1;p:&!n;x:*p", hir::ReferenceMode::Exclusive),
        ("row:{->n:1};p:&row;x:*p", hir::ReferenceMode::Shared),
        ("xs:[1];p:&xs;x:*p", hir::ReferenceMode::Shared),
        ("n:1;p:&n;q:&p;x:*q", hir::ReferenceMode::Shared),
        ("x:*(&(1+2))", hir::ReferenceMode::Shared),
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, deref) = checker.derefs.first_key_value().unwrap();
        assert_eq!(deref.mode, Some(mode));
        assert_eq!(deref.span, checker.points[id].span);
        assert_eq!(
            deref.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(deref.input), Route::Next),
                Edge::new(Port::Normal(deref.input), Port::Operation(id), Route::Next),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
    }
    let source = "id<&int32>:(p<&int32>){->p};n:1;x:*(id(&n))";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let deref = checker.derefs.values().next().unwrap();
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(checker.points[call.point].parent, Some(deref.input));
    assert_eq!(
        checker.region_edges[&deref.input][0].to,
        Port::Entry(call.point)
    );
}

#[test]
pub(crate) fn deref_stages_preserve_stopped_inputs_owners_control_and_bottom_referents() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){*(stop())};*(f())"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    for (&id, deref) in &checker.derefs {
        assert_eq!(deref.mode, None);
        assert_eq!(
            deref.edges,
            [Edge::new(
                Port::Entry(id),
                Port::Entry(deref.input),
                Route::Next
            )]
        );
    }
    assert_eq!(checker.derefs.len(), 2);
    assert!(checker.derefs.values().any(|deref| deref.owner != 0));
    let (checker, _) = check("f<never>:(p<&never>){->*p}");
    let (&id, deref) = checker.derefs.first_key_value().unwrap();
    assert_eq!(deref.mode, Some(hir::ReferenceMode::Shared));
    assert!(
        deref
            .edges
            .iter()
            .any(|edge| edge.to == Port::Operation(id))
    );
    assert!(!deref.edges.iter().any(|edge| edge.to == Port::Normal(id)));
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;n:1;p:&n;|flag|x:*p").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.derefs.values().next().unwrap().control);
}

#[test]
pub(crate) fn deref_stages_publish_atomically_with_valid_identity_and_shared_budgets() {
    let (mut checker, block) = check("n:1;p:&n;x:*p");
    let hir::Stmt::Bind { value, .. } = &block.stmts[2] else {
        panic!()
    };
    let hir::ExprKind::Deref(pointer) = &value.kind else {
        panic!()
    };
    let (&id, deref) = checker.derefs.first_key_value().unwrap();
    let deref = deref.clone();
    let count = checker.deref_edges;
    checker
        .deref_operation(id, deref.input, pointer, deref.span)
        .unwrap();
    assert_eq!(checker.deref_edges, count);
    checker.derefs.clear();
    checker.deref_edges = 0;
    let mut invalid = *pointer.clone();
    invalid.ty = hir::Type::Bool;
    assert!(
        checker
            .deref_operation(id, deref.input, &invalid, deref.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[deref.input].owner += 1;
    assert!(
        checker
            .deref_operation(id, deref.input, pointer, deref.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[deref.input].owner -= 1;
    checker.unary_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .deref_operation(id, deref.input, pointer, deref.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.derefs.is_empty());
    assert_eq!(checker.deref_edges, 0);
}
