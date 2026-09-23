use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn immutable_union_widening_copies_and_narrowing_keep_real_origins() {
    for tail in [
        "wide<A><B>:source;copy:wide;|copy<A>|out:copy.r",
        "wide<A><B><null>:source;copy:wide;|copy<A>|out:copy.r",
    ] {
        let source =
            format!("<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;source:{{->r:&x}};{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = checker.locals.len() - 1;
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        for name in ["wide", "copy"] {
            assert!(checker.derived_local(id(&checker, name)));
        }
        assert!(checker.derived_local(out));
        statements(&mut checker, "p:@\"proof\";|copy<A>|q:p.can_copy<uint32>()");
        assert!(checker.queries.last().unwrap().control);
    }
}

#[test]
pub(crate) fn immutable_union_carrier_copies_follow_later_pointee_marks() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{r<& &int32>}>;x:=false;a:&x;wide<A><B>:{->r:&a};copy:wide;|copy<A>|out:*(copy.r)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let x = id(&checker, "x");
    let out = checker.locals.len() - 1;
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
    assert!(checker.pointees[&out].complete);
    checker.mark_derived(x);
    assert!(checker.derived_local(id(&checker, "copy")));
    assert!(checker.derived_local(out));
}

#[test]
pub(crate) fn union_producers_distinguish_null_unknown_and_mutable_sources() {
    for (init, mutable, complete) in [
        ("null", "", true),
        ("{->r:{->&x}}", "", false),
        ("{->r:&x}", "=", true),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;source<A><null>:{init};wide<A><B><null>:{mutable}source;copy:wide;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete, "{source}");
        assert_eq!(
            origins.roots,
            if mutable.is_empty() {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(&checker, "x")])
            }
        );
    }
}

#[test]
pub(crate) fn union_snapshots_preserve_prior_sources_and_distinct_field_orders() {
    let source = "<A>:<{a<boolean>;r<&boolean>}>;<B>:<{r<&boolean>;z<boolean>}>;x:=false;y:=true;source<A>:={->a:false;->r:&x};wide<A><B>:source;source={->a:true;->r:&y};copy:wide";
    crate::compile(&format!(
        "{source};|copy<A>|out:copy.r;|copy<B>|other:copy.r"
    ))
    .unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    statements(&mut checker, "|copy<A>|out:copy.r");
    let out = checker.locals.len() - 1;
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x")])
    );
    statements(&mut checker, "|copy<B>|other:copy.r");
    let other = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(other.complete && other.roots.is_empty());
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(id(&checker, "wide")));
    assert!(!checker.derived_local(out));
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(id(&checker, "copy")));
}

#[test]
pub(crate) fn union_capture_keeps_work_depth_and_loan_checks() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}",
    );
    let wide = id(&checker, "wide");
    let key = checker.record_shapes[&wide]
        .entries
        .keys()
        .next()
        .unwrap()
        .clone();
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    checker.record_shape_source(value, &key).unwrap();
    assert_eq!(checker.calls, calls);
    let mut wrapped = Expr {
        kind: ExprKind::Local(wide),
        ty: value.ty.clone(),
        span: value.span,
    };
    for _ in 0..MAX_DEPTH {
        wrapped = Expr {
            ty: value.ty.clone(),
            span: value.span,
            kind: ExprKind::Coerce {
                value: Box::new(wrapped),
            },
        };
    }
    let error = checker.record_shape_source(&wrapped, &key).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("source depth"));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.capture_record_shapes(wide, value).is_err());
    assert!(
        checker.record_shapes[&wide]
            .snapshots(&[])
            .any(|value| !value.origins.roots.is_empty())
    );
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};copy:wide;x=true;|copy<A>|out:copy.r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}
