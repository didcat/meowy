use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

const PRELUDE: &str = "<R>:<{r<&boolean>}>;f<&boolean>:(p<R><null>,q<&boolean>){|p<R>|->p.r;|p<null>|->q};x:=false;y:=true;";

#[test]
pub(crate) fn nullable_record_arguments_keep_wrapped_and_copied_owners() {
    for tail in [
        "r:f({->r:&x},&y)",
        "a<R><null>:{->r:&x};r:f(a,&y)",
        "a<R><null>:{->r:&x};b:a;r:f(b,&y)",
        "a<R><null>:=null;a={->r:&x};r:f(a,&y)",
    ] {
        let source = format!("{PRELUDE}{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        let r = id(&checker, "r");
        assert_eq!(checker.pointees[&r].roots, BTreeSet::from([x, y]), "{tail}");
        assert!(checker.pointees[&r].complete, "{tail}");
        checker.mark_derived(x);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn known_null_arguments_add_no_owners_and_keep_complete_results() {
    for tail in ["r:f(null,&y)", "a<R><null>:null;r:f(a,&y)"] {
        let source = format!("{PRELUDE}{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let r = id(&checker, "r");
        assert_eq!(
            checker.pointees[&r].roots,
            BTreeSet::from([id(&checker, "y")])
        );
        assert!(checker.pointees[&r].complete);
        checker.mark_derived(id(&checker, "x"));
        assert!(!checker.derived_local(r));
    }
}

#[test]
pub(crate) fn nested_nullable_record_fields_reuse_origin_snapshots() {
    for (value, owners) in [("{->r:&x}", vec!["x", "y"]), ("null", vec!["y"])] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<N>:<{{inner<R><null>}}>;
             f<&boolean>:(p<N>,q<&boolean>){{|p.inner<R>|->p.inner.r;|p.inner<null>|->q}};
             x:=false;y:=true;a<N>:{{->inner<R><null>:{value}}};r:f(a,&y)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            owners.iter().map(|name| id(&checker, name)).collect()
        );
    }
}

#[test]
pub(crate) fn nullable_candidates_preserve_unknown_origin_completeness() {
    let source = format!("{PRELUDE}a<R><null>:{{->r:{{->&x}}}};r:f(a,&y)");
    crate::compile(&source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, &source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "y")]));
    assert!(!origins.complete);
}

#[test]
pub(crate) fn heterogeneous_record_arguments_do_not_share_paths() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{n<&int32>}>;f<&boolean>:(p<A><B>,q<&boolean>){|p<A>|->p.r;|p<B>|->q};x:=false;y:=true;a<A><B>:{->r:&x};r:f(a,&y)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "r")].complete);
}
