use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn temporary_stages_keep_cells_and_source_before_materialization_order() {
    for source in ["x:*(&7)", "x:*(&{->n:1})", "x:*(&[1,2])", "n:1;cell:&(&n)"] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, temporary) = checker.temporary_borrows.first_key_value().unwrap();
        let cell = temporary.cell.unwrap();
        assert_eq!(checker.proofs.temporaries[&cell.local], cell.statement);
        assert_eq!(temporary.span, checker.points[id].span);
        assert_eq!(
            temporary.edges,
            [
                Edge::new(Port::Entry(id), Port::Entry(temporary.input), Route::Next),
                Edge::new(
                    Port::Normal(temporary.input),
                    Port::Operation(id),
                    Route::Next
                ),
                Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
            ]
        );
        if source.starts_with("n:") {
            assert!(matches!(
                checker.locals[cell.local],
                hir::Type::Reference(_)
            ));
            assert_ne!(
                cell.local,
                checker.place_borrows.values().next().unwrap().storage
            );
        }
    }
    let source = "make<int32>:(){->7};x:*(&(make()))";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let temporary = checker.temporary_borrows.values().next().unwrap();
    assert_eq!(
        temporary.input,
        checker.invocations.values().next().unwrap().point
    );
    assert_eq!(checker.invocations.len(), 1);
}

#[test]
pub(crate) fn temporary_stages_preserve_stopped_inputs_owner_and_control() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){&(stop())};f()"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let (&id, temporary) = checker.temporary_borrows.first_key_value().unwrap();
    assert_eq!(temporary.cell, None);
    assert_ne!(temporary.owner, 0);
    assert_eq!(
        temporary.edges,
        [Edge::new(
            Port::Entry(id),
            Port::Entry(temporary.input),
            Route::Next
        )]
    );
    assert!(checker.proofs.temporaries.is_empty());
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;|flag|x:*(&1)").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.temporary_borrows.values().next().unwrap().control);
}

#[test]
pub(crate) fn temporary_stages_preserve_lifetimes_and_avoid_other_parent_staging() {
    for source in ["x:*(&(({->n:1}).n))", "x:*(&([1][1]))"] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        assert!(!checker.proofs.temporaries.is_empty());
        assert!(checker.temporary_borrows.is_empty());
        assert!(!checker.projections.is_empty() || !checker.elements.is_empty());
    }
    for (source, code) in [
        ("p:&1;x:*p", "E303"),
        ("n:=1;p:&(&!n)", "B001"),
        ("p:&missing", "E201"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}

#[test]
pub(crate) fn temporary_stages_validate_storage_identity_and_publish_atomically() {
    let (mut checker, block) = check("p:&1");
    let hir::Stmt::Statement { stmts, .. } = &block.stmts[0] else {
        panic!()
    };
    let hir::Stmt::Bind { value, .. } = &stmts[0] else {
        panic!()
    };
    let (&id, temporary) = checker.temporary_borrows.first_key_value().unwrap();
    let temporary = temporary.clone();
    let count = checker.temporary_borrow_edges;
    checker
        .temporary_operation(id, temporary.input, value, temporary.span)
        .unwrap();
    assert_eq!(checker.temporary_borrow_edges, count);
    checker.temporary_borrows.clear();
    checker.temporary_borrow_edges = 0;
    for case in 0..3 {
        let mut invalid = value.clone();
        let hir::ExprKind::TemporaryBorrow {
            id: local,
            statement,
            ..
        } = &mut invalid.kind
        else {
            panic!()
        };
        match case {
            0 => *local = checker.locals.len(),
            1 => *statement += 1,
            2 => invalid.ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
            _ => unreachable!(),
        }
        assert!(
            checker
                .temporary_operation(id, temporary.input, &invalid, temporary.span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.points[temporary.input].parent = None;
    assert!(
        checker
            .temporary_operation(id, temporary.input, value, temporary.span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[temporary.input].parent = Some(id);
    checker.place_borrow_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .temporary_operation(id, temporary.input, value, temporary.span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.temporary_borrows.is_empty());
    assert_eq!(checker.temporary_borrow_edges, 0);
}
