use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Expr, ExprKind, Stmt, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn coerced_record_calls_keep_reference_origins() {
    for tail in [
        "row<R><null>:f(&x);|row<R>|out:row.r",
        "row:{->inner<R><null>:f(&x)};|row.inner<R>|out:row.inner.r",
        "row<R><null>:=null;row=f(&x);|row<R>|out:row.r",
        "row<R><null>:g(f(&x),&x);|row<R>|out:row.r",
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;f<R>:(p<&boolean>){{->r:p}};g<R>:(p<R><null>,q<&boolean>){{|p<R>|->r:p.r;|p<null>|->r:q}};x:=false;{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]), "{tail}");
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn coerced_record_calls_keep_carriers_and_unknown_inputs() {
    for (arg, complete) in [("&a", true), ("{->&a}", false)] {
        let source = format!(
            "<R>:<{{c<& &boolean>}}>;<N>:<{{inner<R>}}>;f<N>:(p<& &boolean>){{->inner:{{->c:p}}}};x:=false;a:&x;row<R><null>:f({arg}).inner;|row<R>|out:*(row.c)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
        assert_eq!(checker.pointees[&out].complete, complete);
        assert_eq!(
            checker.pointees[&out].roots,
            if complete {
                BTreeSet::from([id(&checker, "x")])
            } else {
                BTreeSet::new()
            }
        );
        let cells = &checker.record_cells[&id(&checker, "row")][&vec![0]];
        assert_eq!(cells.complete, complete);
        assert_eq!(
            cells.places,
            if complete {
                BTreeSet::from([(id(&checker, "a"), vec![])])
            } else {
                BTreeSet::new()
            }
        );
    }
}

pub(super) fn wrap(value: Expr, ty: Type) -> Expr {
    Expr {
        span: value.span,
        ty,
        kind: ExprKind::Coerce {
            value: Box::new(value),
        },
    }
}

#[test]
pub(crate) fn record_call_coercions_keep_shapes_null_and_limits() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){->r:p};x:=false;row:f(&x)",
    );
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let ty = Type::Union(vec![value.ty.clone(), Type::Null]);
    let mut wrapped = value.clone();
    for _ in 0..3 {
        wrapped = wrap(wrapped, ty.clone());
    }
    let calls = checker.calls;
    let origins = checker.record_source_origins(&wrapped, &[0]).unwrap();
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    assert_eq!(checker.calls, calls);
    let incompatible = wrap(
        value.clone(),
        Type::Union(vec![value.ty.clone(), Type::Bool]),
    );
    assert!(
        !checker
            .record_source_origins(&incompatible, &[0])
            .unwrap()
            .complete
    );
    assert!(
        !checker
            .record_source_cells(&incompatible, &[0])
            .unwrap()
            .complete
    );
    let null = Expr {
        span: value.span,
        ty: Type::Null,
        kind: ExprKind::Null,
    };
    let null = wrap(null, ty.clone());
    let origins = checker.record_source_origins(&null, &[0]).unwrap();
    assert!(origins.complete && origins.roots.is_empty());
    let cells = checker.record_source_cells(&null, &[0]).unwrap();
    assert!(cells.complete && cells.places.is_empty());
    for _ in 3..super::super::MAX_DEPTH - 1 {
        wrapped = wrap(wrapped, ty.clone());
    }
    assert!(
        checker
            .record_source_origins(&wrapped, &[0])
            .unwrap()
            .complete
    );
    wrapped = wrap(wrapped, ty.clone());
    let error = checker.record_source_origins(&wrapped, &[0]).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("source path budget"));
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker.record_source_cells(&null, &[0]).err().unwrap().code,
        "B001"
    );
}

#[test]
pub(crate) fn wrapped_calls_share_call_depth_and_keep_lifetime_checks() {
    for (field, param, arg) in [
        ("r<&boolean>", "&boolean", "&x"),
        ("r<& &boolean>", "& &boolean", "&a"),
    ] {
        let source = format!(
            "<R>:<{{{field}}}>;f<R>:(p<{param}>){{->r:p}};g<R>:(p<R><null>,q<{param}>){{|p<R>|->r:p.r;|p<null>|->r:q}};x:=false;a:&x;row<R><null>:g(f({arg}),{arg})"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        let stmts = statements(&mut checker, &source);
        let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
            panic!()
        };
        let calls = checker.calls;
        let depth = crate::check::dependencies::calls::MAX_DEPTH;
        let error = if param == "&boolean" {
            assert!(checker.record_source_origins(value, &[0]).unwrap().complete);
            checker
                .record_source_origins_at(value, &[0], depth)
                .err()
                .unwrap()
        } else {
            assert!(checker.record_source_cells(value, &[0]).unwrap().complete);
            checker
                .record_source_cells_at(value, &[0], depth)
                .err()
                .unwrap()
        };
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("call depth"));
        assert_eq!(checker.calls, calls);
    }
    let source = "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){local:false;->r:&local};x:=false;row<R><null>:f(&x)";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E303");
    let source = "<R>:<{r<&boolean>}>;f<R>:(p<&boolean>){->r:p};x:=false;row<R><null>:{->f(&x)};x=true;|row<R>|out:row.r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}
