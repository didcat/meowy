use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn shared_union_dereferences_retain_named_and_stored_view_origins() {
    for tail in [
        "view:&wide;alias:view;copy:*alias",
        "copy:*(&wide)",
        "view:&wide;copy:*(&*view)",
        "row:{->view:&wide};copy:*(row.view)",
        "row:{->item:wide};view:row.&item;copy:*view",
        "row:{->item:wide};view:&row;copy:*(view.&item)",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;wide<A><B>:{{->r:&x}};{tail};|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = checker.locals.len() - 1;
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(id(&checker, "copy")));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn shared_union_retargets_merge_owners_without_changing_prior_copies() {
    for (rhs, complete) in [("&right", true), ("{->&right}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;left<A><B>:{{->r:&x}};right<A><B>:{{->r:&y}};view:=&left;old:*view;view={rhs};copy:*view;|old<A>|before:old.r;|copy<A>|after:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let after = checker.locals.len() - 1;
        let before = after - 1;
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        assert_eq!(checker.pointees[&before].roots, BTreeSet::from([x]));
        assert_eq!(checker.pointees[&after].complete, complete);
        assert_eq!(
            checker.pointees[&after].roots,
            if complete {
                BTreeSet::from([x, y])
            } else {
                BTreeSet::from([x])
            }
        );
        checker.mark_derived(y);
        assert!(!checker.derived_local(id(&checker, "old")));
    }
}

#[test]
pub(crate) fn shared_union_views_keep_null_and_carrier_contents_distinct() {
    for (init, empty) in [("{->r:&a}", false), ("null", true)] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{other<boolean>}}>;x:=false;a:&x;wide<A><B><null>:{init};view:&wide;copy:*view;|copy<A>|out:*(copy.r)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            if empty {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(&checker, "x")])
            }
        );
    }
}

#[test]
pub(crate) fn shared_union_reads_preserve_owner_lifetimes_and_known_returns() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;view:{local<A><B>:{->r:&x};->&local};copy:*view";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};view:&(*(&({->item:wide}.item)));copy:*view";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;wide<A><B>:={->r:&x};view:&wide;wide={->r:&y};copy:*view";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<&U>:(p<&U>){->p};x:=false;wide<U>:{->r:&x};view:f(&wide);copy:*view;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(checker.pointees[&(checker.locals.len() - 1)].complete);
}

#[test]
pub(crate) fn shared_union_reads_validate_location_prefixes_and_budgets() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;wide<A><B>:{->r:&x};view:&wide",
    );
    let wide = id(&checker, "wide");
    let view = id(&checker, "view");
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let (key, snapshot) = checker.record_shapes[&wide]
        .entries
        .iter()
        .find(|(_, value)| !value.origins.roots.is_empty())
        .unwrap();
    let key = key.clone();
    let mut snapshot = snapshot.clone();
    let span = crate::ast::Span::new(1, 2);
    let value = Expr {
        kind: ExprKind::Local(view),
        ty: checker.locals[view].clone(),
        span,
    };
    let narrowed = Expr {
        kind: ExprKind::Coerce {
            value: Box::new(Expr {
                kind: ExprKind::Deref(Box::new(value.clone())),
                ty: checker.locals[wide].clone(),
                span,
            }),
        },
        ty: key.variants[0].1.clone(),
        span,
    };
    assert_eq!(
        checker
            .record_shape_snapshot(&narrowed, &[0])
            .unwrap()
            .unwrap()
            .origins
            .roots,
        BTreeSet::from([x])
    );
    let invalid =
        ShapeKey::new(&[0, 0], &[(1, &key.variants[0].1)], &mut checker.flow, span).unwrap();
    snapshot.origins.roots = BTreeSet::from([y]);
    checker
        .record_shapes
        .get_mut(&wide)
        .unwrap()
        .insert(invalid, snapshot, &mut checker.flow, span)
        .unwrap();
    checker
        .reference_cells
        .get_mut(&view)
        .unwrap()
        .places
        .insert((wide, vec![0]));
    let snapshot = checker.view_shape_source(&value, &key).unwrap();
    assert!(!snapshot.origins.complete);
    assert_eq!(snapshot.origins.roots, BTreeSet::from([x]));
    checker.reference_cells.get_mut(&view).unwrap().places =
        (0..MAX_ROOTS + 1).map(|root| (root, vec![])).collect();
    let error = checker.view_shape_source(&value, &key).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("location capacity"));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.view_shape_source(&value, &key).is_err());
}
