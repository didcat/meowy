use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    (checker, block)
}

#[test]
pub(crate) fn place_borrows_capture_addresses_without_operand_or_pointee_reads() {
    let source = "n:1;p:&((n));cell:&p;r:{->child:{->x:2}};q:&(r.child.x)";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    assert_eq!(checker.place_borrows.len(), 3);
    for (&id, borrow) in &checker.place_borrows {
        assert_eq!(borrow.mode, hir::ReferenceMode::Shared);
        assert_eq!(borrow.storage, borrow.place.root);
        assert!(checker.points.iter().all(|point| point.parent != Some(id)));
        assert_eq!(
            borrow.edges[0],
            Edge::new(
                Port::Entry(id),
                Port::Address { point: id, step: 0 },
                Route::Next
            )
        );
        assert!(borrow.edges.contains(&Edge::new(
            Port::Address {
                point: id,
                step: borrow.place.fields.len()
            },
            Port::Operation(id),
            Route::Next
        )));
        assert!(borrow.edges.contains(&Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            Route::Next
        )));
    }
    let cell = checker
        .place_borrows
        .values()
        .find(|borrow| matches!(checker.locals[borrow.place.root], hir::Type::Reference(_)))
        .unwrap();
    let pointee = checker
        .place_borrows
        .values()
        .find(|borrow| matches!(checker.locals[borrow.place.root], hir::Type::Int { .. }))
        .unwrap();
    assert_ne!(cell.storage, pointee.storage);
    assert!(cell.place.fields.is_empty());
    assert!(
        checker
            .place_borrows
            .values()
            .any(|borrow| borrow.place.fields == [0, 0])
    );
    assert!(checker.derefs.is_empty());
}

#[test]
pub(crate) fn place_borrows_preserve_canonical_alias_storage_and_source_member_types() {
    let source =
        r#"choose:(flag<boolean>)'out{|flag|{'out->n:=1;p:&n};|!flag|{'out->n:="x";p:&n}}"#;
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let borrows = checker.place_borrows.values().collect::<Vec<_>>();
    assert_eq!(borrows.len(), 2);
    assert_ne!(borrows[0].place.root, borrows[1].place.root);
    assert_eq!(borrows[0].storage, borrows[1].storage);
    assert_ne!(
        checker.locals[borrows[0].place.root],
        checker.locals[borrows[1].place.root]
    );
    for borrow in borrows {
        let alias = &checker.proofs.aliases[&borrow.place.root];
        assert_eq!(borrow.storage, alias.root);
        assert_eq!(alias.borrowed, Some(borrow.span));
        assert_ne!(borrow.owner, 0);
    }
}

#[test]
pub(crate) fn place_borrows_preserve_control_errors_and_other_borrow_families() {
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;n:1;|flag|p:&n").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.place_borrows.values().next().unwrap().control);
    for (source, code) in [
        ("p:&missing", "E201"),
        ("r:{->n:1};p:&(r.missing)", "E201"),
        ("n:=1;p:&n;n=2;x:*p", "E302"),
        ("p:{n:1;->&n}", "E303"),
        ("n:=1;p:&!n;q:&p", "B001"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
    for source in ["n:=1;p:&!n", "p:&1"] {
        crate::compile(source).unwrap();
        assert!(check(source).0.place_borrows.is_empty());
    }
    let (checker, _) = check("n:1;p:&n;q:&*p");
    assert_eq!(checker.place_borrows.len(), 1);
    assert_eq!(checker.reborrow_ops.len(), 1);
}

#[test]
pub(crate) fn place_borrows_validate_paths_types_and_atomic_shared_budget_publication() {
    let (mut checker, block) = check("r:{->n:1};p:&(r.n)");
    let hir::Stmt::Bind { value, .. } = &block.stmts[1] else {
        panic!()
    };
    let (&id, borrow) = checker.place_borrows.first_key_value().unwrap();
    let span = borrow.span;
    let count = checker.place_borrow_edges;
    let points = checker.points.len();
    checker.place_borrow_operation(id, value, span).unwrap();
    assert_eq!(checker.place_borrow_edges, count);
    checker.place_borrows.clear();
    checker.place_borrow_edges = 0;
    for case in 0..3 {
        let mut invalid = value.clone();
        let hir::ExprKind::Borrow(place) = &mut invalid.kind else {
            panic!()
        };
        match case {
            0 => place.fields[0] = usize::MAX,
            1 => invalid.ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
            2 => place.root = checker.locals.len(),
            _ => unreachable!(),
        }
        assert!(
            checker
                .place_borrow_operation(id, &invalid, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    let mut full = value.clone();
    let hir::ExprKind::Borrow(place) = &mut full.kind else {
        panic!()
    };
    place.fields = vec![0; crate::list::MAX_WRITE_PATH + 1];
    assert!(
        checker
            .place_borrow_operation(id, &full, span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    checker.reborrow_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .place_borrow_operation(id, value, span)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.place_borrows.is_empty());
    assert_eq!(checker.place_borrow_edges, 0);
    assert_eq!(checker.points.len(), points);
}
