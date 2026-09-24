use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn union_record_inputs_keep_nullable_and_unknown_contents() {
    for (init, field, complete, extra) in [
        ("{->r:&y}", "&row", true, true),
        ("null", "&row", true, false),
        ("{->r:{->&y}}", "&row", false, false),
        ("{->r:&y}", "{->&row}", false, false),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<M>:<R><null>;<A>:<{{view<&M>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){{->q}};x:=false;y:=true;row<M>:{init};unknown<&M>:{field};wide<U>:{{->view:unknown}};out:f(wide,&x)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if extra {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn union_record_inputs_keep_deeper_nested_views_and_inline_calls() {
    let source = "<R>:<{r<&boolean>}>;<N>:<{inner<&R>}>;<A>:<{view<& &N>}>;<B>:<{other<boolean>}>;<U>:<A><B>;make<U>:(p<& &N>){->{->view:p}};f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true;row<R>:{->r:&y};nested<N>:{->inner:&row};link:&nested;out:f(make(&link),&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let out = id(&checker, "out");
    assert!(checker.pointees[&out].complete);
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(checker.derived_local(out));
}

#[test]
pub(crate) fn union_record_inputs_merge_owned_projections_and_stored_references() {
    let source = "<R>:<{flag<boolean>;r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true;row<R>:{->flag:false;->r:&y};wide<U>:{->view:&row};out:f(wide,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "out")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y"), id(&checker, "row")])
    );
}

#[test]
pub(crate) fn union_record_input_queries_keep_depth_work_and_no_replay() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;make<U>:(p<&R>){->{->view:p}};f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;row<R>:{->r:&x};out:f(make(&row),&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_origins(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker.reference_origins_at(value, 32).err().unwrap().code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.reference_origins(value).is_err());
}

#[test]
pub(crate) fn union_record_inputs_preserve_lifetimes_and_heterogeneous_contents() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true";
    let source = format!(
        "{prefix};row<R>:={{->r:&y}};wide<U>:{{->view:&row}};out:f(wide,&x);row={{->r:&x}};copy:*out"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!(
        "{prefix};out:{{row<R>:{{->r:&y}};wide<U>:{{->view:&row}};->f(wide,&x)}};copy:*out"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
    let source = "<R>:<{r<&boolean>}>;<S>:<{other<boolean>}>;<V>:<R><S>;<A>:<{view<&V>}>;<B>:<{other<int32>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;row<V>:{->r:&x};wide<U>:{->view:&row};out:f(wide,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(checker.pointees[&id(&checker, "out")].complete);
}
