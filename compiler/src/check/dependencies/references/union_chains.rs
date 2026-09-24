use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn shared_union_chains_retain_named_stored_and_deeper_origins() {
    for tail in [
        "view:&wide;cell:&view;copy:**cell",
        "view:&wide;cell:&view;outer:&cell;copy:***outer",
        "view:&wide;row:{->cell:&view};copy:**(row.cell)",
        "view:&wide;cell:&view;row:{->inner:{->cell:&cell}};copy:***(row.inner.cell)",
        "view:&wide;cell:&view;alias:cell;copy:**alias",
        "copy:**(&(&wide))",
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
pub(crate) fn union_chain_retargets_preserve_old_copies_and_unknown_layers() {
    for (rhs, complete) in [("&b", true), ("{->&b}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;left<A><B>:{{->r:&x}};right<A><B>:{{->r:&y}};a:&left;b:&right;cell:=&a;old:cell;cell={rhs};before:**old;copy:**cell;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            if complete {
                BTreeSet::from([x, y])
            } else {
                BTreeSet::from([x])
            }
        );
        checker.mark_derived(y);
        assert!(!checker.derived_local(id(&checker, "old")));
        assert!(!checker.derived_local(id(&checker, "before")));
        assert_eq!(checker.derived_local(id(&checker, "cell")), complete);
    }
}

#[test]
pub(crate) fn union_chain_nullable_carriers_keep_locations_and_contents_distinct() {
    for (init, empty) in [("null", true), ("{->r:&a}", false)] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{other<boolean>}}>;x:=false;a:&x;wide<A><B><null>:{init};view:&wide;cell:&view;copy:**cell;|copy<A>|out:*(copy.r)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert!(checker.reference_cells[&id(&checker, "cell")].complete);
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
pub(crate) fn union_chain_classification_preserves_shared_modes_and_limits() {
    let record = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![crate::hir::Field {
            name: "r".into(),
            ty: Type::Reference(Box::new(Type::Bool)),
            mutable: false,
        }],
    };
    let other = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![],
    };
    let union = Type::Union(vec![record, other]);
    let span = crate::ast::Span::new(1, 2);
    let mut checker = Checker::new();
    for count in [1, 2, MAX_CELL_DEPTH + 1] {
        let mut ty = union.clone();
        for _ in 0..count {
            ty = Type::Reference(Box::new(ty));
        }
        assert!(checker.origin_carrier(&ty, span).unwrap());
    }
    let mut ty = union.clone();
    for _ in 0..MAX_CELL_DEPTH + 2 {
        ty = Type::Reference(Box::new(ty));
    }
    assert_eq!(checker.origin_carrier(&ty, span).unwrap_err().code, "B001");
    for ty in [
        Type::Exclusive(Box::new(Type::Reference(Box::new(union.clone())))),
        Type::Reference(Box::new(Type::Exclusive(Box::new(union)))),
    ] {
        assert!(!checker.origin_carrier(&ty, span).unwrap());
    }
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.origin_carrier(&ty, span).is_err());
}

#[test]
pub(crate) fn returned_union_carriers_remain_incomplete_and_keep_lifetime_checks() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<& &U>:(p<& &U>){->p};x:=false;wide<U>:{->r:&x};view:&wide;cell:f(&view);copy:**cell;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "cell")].complete);
    assert!(!checker.pointees[&(checker.locals.len() - 1)].complete);
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<& &U>:(p<&U>){local:p;->&local};x:=false;wide<U>:{->r:&x};cell:f(&wide)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn union_chains_preserve_live_cell_loans_and_temporary_expiry() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;left<A><B>:{->r:&x};right<A><B>:{->r:&y}";
    let source = format!("{prefix};view:=&left;cell:&view;view=&right;copy:**cell");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};cell:&(&left);copy:**cell");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
