use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn nullable_record_references_keep_wrapped_and_null_origins() {
    for (init, names) in [("{->r:&x}", vec!["x", "y"]), ("null", vec!["y"])] {
        for tail in ["r:f(&row,&y)", "view:&row;copy:view;r:f(copy,&y)"] {
            let source = format!(
                "<R>:<{{r<&boolean>}}>;<M>:<R><null>;f<&boolean>:(p<&M>,q<&boolean>){{value:*p;|value<R>|->value.r;|value<null>|->q}};x:=false;y:=true;row<M>:{init};{tail}"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let r = id(&checker, "r");
            assert!(checker.pointees[&r].complete, "{source}");
            assert_eq!(
                checker.pointees[&r].roots,
                names.iter().map(|name| id(&checker, name)).collect()
            );
            checker.mark_derived(id(&checker, "x"));
            assert_eq!(checker.derived_local(r), init != "null");
        }
    }
}

#[test]
pub(crate) fn nullable_record_targets_work_through_shared_chains_and_nested_views() {
    for (init, names) in [("{->r:&x}", vec!["x", "y"]), ("null", vec!["y"])] {
        for (defs, tail) in [
            (
                "f<&boolean>:(p<& &M>,q<&boolean>){value:**p;|value<R>|->value.r;|value<null>|->q}",
                "view:&row;r:f(&view,&y)",
            ),
            (
                "<W>:<{inner<&M>}>;f<&boolean>:(p<&W>,q<&boolean>){value:*(p.inner);|value<R>|->value.r;|value<null>|->q}",
                "outer<W>:{->inner:&row};r:f(&outer,&y)",
            ),
        ] {
            let source = format!(
                "<R>:<{{r<&boolean>}}>;<M>:<R><null>;{defs};x:=false;y:=true;row<M>:{init};{tail}"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&id(&checker, "r")];
            assert!(origins.complete, "{source}");
            assert_eq!(
                origins.roots,
                names.iter().map(|name| id(&checker, name)).collect()
            );
        }
    }
}

#[test]
pub(crate) fn nullable_record_targets_keep_unknown_fields_and_locations_incomplete() {
    for (init, arg) in [("{->r:{->&x}}", "&row"), ("{->r:&x}", "{->&row}")] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<M>:<R><null>;f<&boolean>:(p<&M>,q<&boolean>){{value:*p;|value<R>|->value.r;|value<null>|->q}};x:=false;y:=true;row<M>:{init};r:f({arg},&y)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(!origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "y")]));
    }
}

#[test]
pub(crate) fn heterogeneous_record_targets_do_not_merge_layouts() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{n<&int32>}>;<M>:<A><B>;f<&boolean>:(p<&M>,q<&boolean>){value:*p;|value<A>|->value.r;|value<B>|->q};x:=false;y:=true;row<M>:{->r:&x};r:f(&row,&y)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}
