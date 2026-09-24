use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn shared_union_views_keep_owner_locations_through_aliases_and_fields() {
    for tail in [
        "view:&wide;copy:view",
        "row:{->view:&wide};copy:row.view",
        "view:&wide;copy:&*view",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;wide<A><B>:{{->r:&x}};{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let copy = id(&checker, "copy");
        let cells = &checker.reference_cells[&copy];
        assert!(cells.complete, "{tail}");
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "wide"), vec![])])
        );
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(copy));
    }
}

#[test]
pub(crate) fn shared_union_location_retargets_preserve_copies_and_unknowns() {
    for (rhs, complete) in [("&right", true), ("{->&right}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;left<A><B>:{{->r:&x}};right<A><B>:{{->r:&y}};view:=&left;old:view;view={rhs}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        let mut places = BTreeSet::from([(id(&checker, "left"), vec![])]);
        if complete {
            places.insert((id(&checker, "right"), vec![]));
        }
        assert_eq!(cells.complete, complete);
        assert_eq!(cells.places, places);
        assert_eq!(
            checker.reference_cells[&id(&checker, "old")].places,
            BTreeSet::from([(id(&checker, "left"), vec![])])
        );
    }
}

#[test]
pub(crate) fn shared_union_field_locations_exclude_sibling_marks() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;row:{->left<A><B>:{->r:&x};->right<A><B>:{->r:&y}};view:row.&left";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let view = id(&checker, "view");
    assert_eq!(
        checker.reference_cells[&view].places,
        BTreeSet::from([(id(&checker, "row"), vec![0])])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(view));
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(view));
}

#[test]
pub(crate) fn shared_union_storage_classification_keeps_kind_and_budget_boundaries() {
    let record = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![crate::hir::Field {
            name: "r".into(),
            ty: Type::Reference(Box::new(Type::Bool)),
            mutable: false,
        }],
    };
    let span = crate::ast::Span::new(1, 2);
    let mut checker = Checker::new();
    let union = Type::Union(vec![record.clone(), Type::Null]);
    assert!(
        checker
            .origin_carrier(&Type::Reference(Box::new(union)), span)
            .unwrap()
    );
    let mixed = Type::Union(vec![record.clone(), Type::Reference(Box::new(Type::Bool))]);
    assert!(
        !checker
            .origin_carrier(&Type::Reference(Box::new(mixed)), span)
            .unwrap()
    );
    let large = Type::Reference(Box::new(Type::Union(vec![record; MAX_ROOTS + 1])));
    assert_eq!(
        checker.origin_carrier(&large, span).unwrap_err().code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.origin_carrier(&large, span).is_err());
}
