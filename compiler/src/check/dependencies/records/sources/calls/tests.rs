use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn record_call_reference_fields_survive_copies_and_nested_results() {
    for source in [
        "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){->r:p};x:=false;row:f(&x);out:row.r",
        "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){->r:p};x:=false;row:f(&x);copy:row;out:copy.r",
        "<R>:<{r<&boolean>}>;<N>:<{inner<R>}>;f<N>:(p<&boolean>){->inner:{->r:p}};x:=false;row:f(&x);out:row.inner.r",
        "<R>:<{r<&boolean[2]>}>;f<R>:(p<&boolean[2]>){->r:p};x<boolean[2]>:=[false,true];row:f(&x);out:row.r",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let out = id(&checker, "out");
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete, "{source}");
        assert_eq!(
            checker.pointees[&out].roots,
            BTreeSet::from([x]),
            "{source}"
        );
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn returned_record_fields_keep_every_compatible_argument() {
    let source = "<R>:<{r<&boolean>}>;f<R>:(a<&boolean>,b<&boolean>){->r:a};x:=false;y:=true;row:f(&x,&y);out:row.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "out")].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
}

#[test]
pub(crate) fn nested_record_calls_preserve_origins_and_unknown_arguments() {
    for (arg, complete) in [("&x", true), ("{->&x}", false)] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;make<R>:(p<&boolean>){{->r:p}};copy<R>:(p<R>){{->r:p.r}};x:=false;row:copy(copy(make({arg})));out:row.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            if complete {
                BTreeSet::from([id(&checker, "x")])
            } else {
                BTreeSet::new()
            }
        );
        let crate::hir::Stmt::Bind { value, .. } = &stmts[stmts.len() - 2] else {
            panic!()
        };
        let error = checker
            .record_source_origins_at(value, &[0], crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("record origin call depth"));
    }
}

#[test]
pub(crate) fn record_call_origin_traversal_keeps_depth_paths_and_lifetimes_checked() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){->r:p};x:=false;row:f(&x)",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    checker.record_source_origins(value, &[0]).unwrap();
    assert_eq!(checker.calls, calls);
    assert!(!checker.record_source_origins(value, &[1]).unwrap().complete);
    let error = checker
        .record_source_origins_at(
            value,
            &[0],
            crate::check::dependencies::calls::MAX_DEPTH + 1,
        )
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("call depth"));
    let source =
        "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){local:false;->r:&local};x:=false;row:f(&x)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn record_call_source_paths_preserve_order_and_bounds() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;<N>:<{aaa<boolean>;inner<R>}>;f<N>:(p<&boolean>){->aaa:false;->inner:{->r:p}};x:=false;out:f(&x).inner.r",
    );
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let (base, path) = checker.record_source_path(value, &[]).unwrap();
    assert!(matches!(base.kind, crate::hir::ExprKind::Call { .. }));
    assert_eq!(path, vec![1, 0]);
    assert_eq!(
        checker.record_call_field_type(value, &[]).unwrap(),
        Some(&value.ty)
    );
    let path = vec![0; super::super::super::MAX_DEPTH];
    let error = checker.record_source_path(value, &path).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("source path budget"));
    assert!(checker.record_source_path(base, &path).is_ok());
    assert!(checker.record_source_path(base, &[0; 33]).is_err());
}
