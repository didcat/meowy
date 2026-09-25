use super::{tests::check, *};

#[test]
pub(crate) fn exclusive_place_ops_preserve_scalar_modes_and_field_permissions() {
    for source in [
        "n<uint8>:=7;p:&!n",
        "n:=true;p:&!n",
        "n<float32>:=1.25;p:&!n",
        "r:{->n:=1};p:&!(r.n)",
        "r:={->inner:{->n:=1}};p:&!(((r).inner).n)",
    ] {
        crate::compile(source).unwrap();
        let (checker, _) = check(source);
        let (&id, borrow) = checker.place_borrows.first_key_value().unwrap();
        assert_eq!(borrow.mode, hir::ReferenceMode::Exclusive);
        assert_eq!(borrow.storage, borrow.place.root);
        assert_eq!(borrow.span, checker.points[id].span);
        assert!(checker.points.iter().all(|point| point.parent != Some(id)));
        assert!(borrow.edges.contains(&Edge::new(
            Port::Address {
                point: id,
                step: borrow.place.fields.len()
            },
            Port::Operation(id),
            Route::Next,
        )));
        assert!(borrow.edges.contains(&Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            Route::Next
        )));
    }
    for source in [
        "n:1;p:&!n",
        "r:={->n:1};p:&!(r.n)",
        "r:={->inner:={->n:1}};p:&!(r.inner.n)",
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, "E305");
    }
}

#[test]
pub(crate) fn exclusive_place_ops_keep_canonical_aliases_owners_and_control() {
    let source = "f:(flag<boolean>)'out{|flag|{'out->n:=1;p:&!n};|!flag|{'out->n:=2;p:&!n}}";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let borrows = checker.place_borrows.values().collect::<Vec<_>>();
    assert_eq!(borrows.len(), 2);
    assert_ne!(borrows[0].place.root, borrows[1].place.root);
    assert_eq!(borrows[0].storage, borrows[1].storage);
    for borrow in borrows {
        let alias = &checker.proofs.aliases[&borrow.place.root];
        assert_eq!(alias.exclusive, Some(borrow.span));
        assert_eq!(alias.borrowed, None);
        assert_ne!(borrow.owner, 0);
    }
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;n:=1;|flag|p:&!n").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.place_borrows.values().next().unwrap().control);
}

#[test]
pub(crate) fn exclusive_place_ops_preserve_errors_and_separate_indexed_and_reborrow_paths() {
    for (source, code) in [
        ("r:{->n:=1};p:&!(r.missing)", "E201"),
        ("n:=1;p:&!n;copy:p;x:*p", "E301"),
        ("n:=1;p:&!n;n=2;x:*p", "E302"),
        ("p:{n:=1;->&!n}", "E303"),
        ("r:={->n:=1};p:&!r", "B001"),
        ("n:=1;r:{->p:&n;->v:=2};p:&!(r.v)", "B001"),
        (
            "f:(flag<boolean>)'out{|flag|{'out->n:=1;p:&!n};|!flag|'out->n:=true}",
            "B001",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    let (checker, _) = check("xs:=[1];p:&!(xs[1])");
    assert!(checker.place_borrows.is_empty());
    assert_eq!(checker.exclusives.len(), 1);
    let (checker, _) = check("n:=1;p:&!n;q:&!*p");
    assert_eq!(checker.place_borrows.len(), 1);
    assert_eq!(checker.reborrow_ops.len(), 1);
}

#[test]
pub(crate) fn exclusive_place_ops_validate_scalar_results_and_share_publication_budgets() {
    let (mut checker, block) = check("n:=1;p:&!n");
    let hir::Stmt::Bind { value, .. } = &block.stmts[1] else {
        panic!()
    };
    let (&id, borrow) = checker.place_borrows.first_key_value().unwrap();
    let span = borrow.span;
    let count = checker.place_borrow_edges;
    checker.place_borrow_operation(id, value, span).unwrap();
    assert_eq!(checker.place_borrow_edges, count);
    checker.place_borrows.clear();
    checker.place_borrow_edges = 0;
    for ty in [
        hir::Type::Bool,
        hir::Type::String,
        hir::Type::Reference(Box::new(hir::Type::Bool)),
    ] {
        let mut invalid = value.clone();
        invalid.ty = hir::Type::Exclusive(Box::new(ty));
        assert!(
            checker
                .place_borrow_operation(id, &invalid, span)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    checker.points[id].owner += 1;
    assert!(
        checker
            .place_borrow_operation(id, value, span)
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[id].owner -= 1;
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
}
