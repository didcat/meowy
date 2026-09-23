use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn mutable_union_replacements_retain_old_owners_and_prior_copies() {
    for write in [
        "wide={->r:&y}",
        "|c|wide={->r:&y}",
        "wide=wide;wide={->r:&y}",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;c:=false;wide<A><B>:={{->r:&x}};old:wide;{write};new:wide;|old<A>|before:old.r;|new<A>|after:new.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        let after = checker.locals.len() - 1;
        let before = after - 1;
        assert_eq!(checker.pointees[&before].roots, BTreeSet::from([x]));
        assert_eq!(checker.pointees[&after].roots, BTreeSet::from([x, y]));
        assert!(checker.pointees[&after].complete);
        checker.mark_derived(y);
        assert!(checker.derived_local(id(&checker, "wide")));
        assert!(!checker.derived_local(id(&checker, "old")));
        assert!(checker.derived_local(id(&checker, "new")));
        statements(&mut checker, "p:@\"proof\";|new<A>|q:p.can_copy<uint32>()");
        assert!(checker.queries.last().unwrap().control);
    }
}

#[test]
pub(crate) fn mutable_union_replacements_preserve_null_and_unknown_alternatives() {
    for (value, complete, has_y) in [
        ("null", true, false),
        ("unknown", false, false),
        ("known", true, true),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;unknown<A>:{{->r:{{->&y}}}};known<A>:{{->r:&y}};wide<A><B><null>:={{->r:&x}};wide={value};copy:wide;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if has_y {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
        assert_eq!(origins.complete, complete);
    }
}

#[test]
pub(crate) fn whole_record_replacements_merge_nested_union_carrier_snapshots() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{other<boolean>}>;x:=false;y:=true;a:&x;b:&y;row:={->inner<A><B>:{->r:&a}};old:row;row={->inner<A><B>:{->r:&b}};copy:row.inner;|copy<A>|out:*(copy.r)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let out = checker.locals.len() - 1;
    assert!(checker.pointees[&out].complete);
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x, y]));
    checker.mark_derived(y);
    assert!(checker.derived_local(id(&checker, "row")));
    assert!(!checker.derived_local(id(&checker, "old")));
}

#[test]
pub(crate) fn mutable_union_shape_changes_keep_distinct_field_orders_and_null_copies() {
    let source = "<A>:<{a<boolean>;r<&boolean>}>;<B>:<{r<&int32>;z<boolean>}>;x:=false;n:=7;left<A>:{->a:false;->r:&x};right<B>:{->r:&n;->z:false};wide<A><B><null>:=null;empty:wide;wide=left;wide=right;copy:wide;|copy<A>|ar:copy.r;|copy<B>|br:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let b = checker.locals.len() - 1;
    let a = b - 1;
    assert_eq!(
        checker.pointees[&a].roots,
        BTreeSet::from([id(&checker, "x")])
    );
    assert_eq!(
        checker.pointees[&b].roots,
        BTreeSet::from([id(&checker, "n")])
    );
    checker.mark_derived(id(&checker, "n"));
    assert!(!checker.derived_local(a));
    assert!(checker.derived_local(b));
    assert!(!checker.derived_local(id(&checker, "empty")));
}

#[test]
pub(crate) fn mutable_union_replacement_failures_preserve_metadata_and_loan_checks() {
    let mut checker = Checker::new();
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;wide<A><B>:={->r:&x};next<A><B>:{->r:&y}";
    statements(&mut checker, source);
    let wide = id(&checker, "wide");
    let next = id(&checker, "next");
    let x = id(&checker, "x");
    let limit = crate::check::dependencies::references::MAX_ROOTS;
    let snapshot = checker
        .record_shapes
        .get_mut(&next)
        .unwrap()
        .entries
        .values_mut()
        .find(|value| !value.origins.roots.is_empty())
        .unwrap();
    snapshot.origins.roots = (1000..1000 + limit).collect();
    let value = Expr {
        kind: crate::hir::ExprKind::Local(next),
        ty: checker.locals[next].clone(),
        span: Span::new(1, 2),
    };
    assert_eq!(
        checker
            .track_record_shapes(wide, &value, true)
            .unwrap_err()
            .code,
        "B001"
    );
    let roots = checker
        .shaped(wide)
        .unwrap()
        .snapshots(&[])
        .flat_map(|value| value.origins.roots.iter().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(roots, BTreeSet::from([x]));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.track_record_shapes(wide, &value, true).is_err());
    assert!(
        checker
            .shaped(wide)
            .unwrap()
            .snapshots(&[])
            .any(|value| value.origins.roots == BTreeSet::from([x]))
    );
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;wide<A><B>:={->r:&x};old:wide;wide={->r:&y};x=true;|old<A>|out:old.r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}
