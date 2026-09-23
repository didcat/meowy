use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn nested_union_construction_copies_and_projections_keep_real_origins() {
    for tail in [
        "row:{->inner<A><B>:{->r:&x}};|row.inner<A>|out:row.inner.r",
        "row:{->inner<A><B>:{->r:&x}};copy:row.inner;|copy<A>|out:copy.r",
        "row:{->deep:{->inner<A><B>:{->r:&x}}};copy:row.deep;|copy.inner<A>|out:copy.inner.r",
        "wide<C><D>:{->inner<A><B>:{->r:&x}};|wide<C>|{copy:wide.inner;|copy<A>|out:copy.r}",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<C>:<{{inner<A><B>}}>;<D>:<{{other<boolean>}}>;x:=false;{tail}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = checker.locals.len() - 1;
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]), "{tail}");
        checker.mark_derived(x);
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn nested_union_carriers_and_nullable_fields_keep_locations() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{r<& &int32>}>;x:=false;a:&x;row:{->inner<A><B><null>:{->r:&a}};copy:row.inner;|copy<A>|out:*(copy.r)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let x = id(&checker, "x");
    let out = checker.locals.len() - 1;
    assert!(checker.pointees[&out].complete);
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
    checker.mark_derived(x);
    assert!(checker.derived_local(id(&checker, "row")));
    assert!(checker.derived_local(id(&checker, "copy")));
}

#[test]
pub(crate) fn conditional_nested_union_sources_merge_known_and_unknown_alternatives() {
    for (source, complete, both) in [
        ("right", true, true),
        ("unknown", false, false),
        ("null", true, false),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;c:=false;left<A>:{{->r:&x}};right<A>:{{->r:&y}};unknown<A>:{{->r:{{->&y}}}};row:{{|c|->inner<A><B><null>:left;|!c|->inner<A><B><null>:{source}}};copy:row.inner;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if both {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn composed_union_alternatives_stay_incomplete_and_keep_known_sources() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;c:=false;other:{->inner<A><B>:{->r:&y}};row:{|c|->inner<A><B>:{->r:&x};|!c|->other};copy:row.inner;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}

#[test]
pub(crate) fn nested_union_block_reads_keep_budget_and_loan_checks() {
    let mut checker = Checker::new();
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;row:{->inner<A><B>:{->r:&x}}";
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let row = id(&checker, "row");
    let key = checker.record_shapes[&row]
        .entries
        .keys()
        .next()
        .unwrap()
        .clone();
    let calls = checker.calls;
    checker.record_shape_source(value, &key).unwrap();
    assert_eq!(checker.calls, calls);
    let mut large = value.clone();
    let crate::hir::ExprKind::Block(block) = &mut large.kind else {
        panic!()
    };
    let dummy = Stmt::Expr(Expr {
        kind: crate::hir::ExprKind::Bool(false),
        ty: Type::Bool,
        span: value.span,
    });
    block.stmts.extend(vec![dummy; MAX_FIELDS + 1]);
    let error = checker.record_shape_source(&large, &key).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("block budget"));
    assert!(
        checker.record_shapes[&row]
            .snapshots(&[])
            .any(|snapshot| !snapshot.origins.roots.is_empty())
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.capture_record_shapes(row, value).is_err());
    let source = format!("{source};copy:row.inner;x=true;|copy<A>|out:copy.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
}

#[test]
pub(crate) fn nested_union_source_fields_keep_independent_sibling_origins() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;row:{->left<A><B>:{->r:&x};->right<A><B>:{->r:&y}};copy:row.left;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let out = checker.locals.len() - 1;
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(id(&checker, "copy")));
    assert!(!checker.derived_local(out));
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(id(&checker, "copy")));
    assert!(checker.derived_local(out));
}
