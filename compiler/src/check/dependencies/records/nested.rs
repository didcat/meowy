use super::writes::id;
use super::*;
use crate::check::dependencies::tests::statements;
use std::collections::BTreeSet;

#[test]
pub(crate) fn nested_record_sources_copies_and_projections_retain_owners() {
    let source = "x:=false;y:=true;row:{->left:{->r:&x};->right:{->r:&y}};copy:row;part:copy.right;a:copy.left.r;b:part.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "a")].roots,
        BTreeSet::from([0])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "b")].roots,
        BTreeSet::from([1])
    );
    assert!(checker.pointees[&id(&checker, "b")].complete);
    checker.mark_derived(1);
    assert!(!checker.derived_local(id(&checker, "a")));
    assert!(checker.derived_local(id(&checker, "b")));
}

#[test]
pub(crate) fn nested_writes_merge_only_the_selected_prefix_and_preserve_copies() {
    for update in [
        "row.inner.r=&y",
        "row.inner={->r:=&y}",
        "|condition|row.inner={->r:=&y}",
    ] {
        let source = format!(
            "x:=false;y:=true;z:=false;condition:=false;row:{{->inner:={{->r:=&x}};->other:{{->r:&z}}}};old:row;{update};a:row.inner.r;b:old.inner.r;c:row.other.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        assert_eq!(
            checker.pointees[&id(&checker, "a")].roots,
            BTreeSet::from([0, 1])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "b")].roots,
            BTreeSet::from([0])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "c")].roots,
            BTreeSet::from([2])
        );
        assert!(checker.pointees[&id(&checker, "a")].complete);
    }
}

#[test]
pub(crate) fn nested_unknown_updates_keep_known_owners_incomplete() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;y:=true;row:{->inner:={->r:&x}};row.inner={->r:{->&y}};r:row.inner.r",
    );
    let origins = &checker.pointees[&id(&checker, "r")];
    assert_eq!(origins.roots, BTreeSet::from([0]));
    assert!(!origins.complete);
}

#[test]
pub(crate) fn nested_record_paths_enforce_depth_limits() {
    for depth in [MAX_DEPTH, MAX_DEPTH + 1] {
        let mut ty = Type::Reference(Box::new(Type::Bool));
        for _ in 0..depth {
            ty = Type::Record {
                primary: Box::new(Type::Null),
                fields: vec![crate::hir::Field {
                    name: "r".into(),
                    ty,
                    mutable: false,
                }],
            };
        }
        let value = Expr {
            kind: ExprKind::Local(0),
            ty,
            span: crate::ast::Span::new(1, 2),
        };
        let result = Checker::new().record_paths(&value);
        if depth == MAX_DEPTH {
            assert_eq!(result.unwrap(), vec![vec![0; depth]]);
        } else {
            assert_eq!(result.unwrap_err().code, "B001");
        }
    }
}

#[test]
pub(crate) fn invalid_nested_writes_keep_ordinary_errors_and_origin_metadata() {
    for (source, code) in [
        ("x:=false;row:{->inner:{->r:&x}}", "E305"),
        ("x:=false;row:{->inner:{->r:=&x}}", "E207"),
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let block = crate::parser::parse("row.inner.r=7").unwrap();
        assert_eq!(checker.stmt(&block.stmts[0]).unwrap_err().code, code);
        let origins = &checker.record_pointees[&id(&checker, "row")][&vec![0, 0]];
        assert!(origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([0]));
    }
}
