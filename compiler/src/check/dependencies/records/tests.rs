use super::*;
use crate::check::dependencies::tests::statements;
use std::collections::BTreeSet;

#[test]
pub(crate) fn completed_record_copies_retain_field_origins_and_later_marks() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;y:=true;row:{->a:&x;->b:&y};copy:row;r:copy.a;s:copy.b",
    );
    let r = checker.locals.len() - 2;
    let s = r + 1;
    assert_eq!(checker.pointees[&r].roots, BTreeSet::from([0]));
    assert_eq!(checker.pointees[&s].roots, BTreeSet::from([1]));
    assert!(checker.pointees[&r].complete);
    checker.mark_derived(1);
    assert!(!checker.derived_local(r));
    assert!(checker.derived_local(s));
    statements(
        &mut checker,
        "p:@\"proof\";|*(copy.b)|q:p.can_copy<uint32>()",
    );
    assert!(checker.queries[0].control);
}

#[test]
pub(crate) fn completed_reference_projections_preserve_ordinary_borrow_validation() {
    let source = "x:=false;row:{->r:&x};copy:row;r:copy.r;flag:*r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let id = checker.locals.len() - 2;
    assert_eq!(checker.pointees[&id].roots, BTreeSet::from([0]));
}

#[test]
pub(crate) fn unknown_record_initializers_stay_incomplete() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;row:{->r:{->&x}};r:row.r");
    let id = checker.locals.len() - 1;
    assert!(!checker.pointees[&id].complete);
}

#[test]
pub(crate) fn unmarked_record_copy_does_not_invent_dependencies() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;row:{->r:&x};copy:row;flag:*(copy.r);p:@\"proof\";|flag|q:p.can_copy<uint32>()",
    );
    assert!(!checker.queries[0].control);
    assert!(checker.derived.is_empty());
}

#[test]
pub(crate) fn record_origin_capacity_preserves_existing_metadata() {
    let mut checker = Checker::new();
    for count in [MAX_FIELDS, MAX_FIELDS + 1] {
        let value = Expr {
            kind: ExprKind::Local(1),
            ty: Type::Record {
                primary: Box::new(Type::Null),
                fields: (0..count)
                    .map(|index| crate::hir::Field {
                        name: format!("r{index}"),
                        ty: Type::Reference(Box::new(Type::Bool)),
                        mutable: false,
                    })
                    .collect(),
            },
            span: crate::ast::Span::new(1, 2),
        };
        let result = checker.track_record_references(0, &value, false);
        if count == MAX_FIELDS {
            result.unwrap();
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, "B001");
            assert_eq!(error.span, value.span);
        }
        assert_eq!(checker.record_pointees[&0].len(), MAX_FIELDS);
        assert!(
            checker.record_pointees[&0]
                .values()
                .all(|origins| !origins.complete)
        );
    }
}

#[test]
pub(crate) fn nested_field_paths_keep_distinct_origins_and_later_marks() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;y:=true;row:{->left:{->r:&x};->right:{->r:&y}}",
    );
    let row = super::writes::id(&checker, "row");
    checker.record_pointees.insert(
        row,
        BTreeMap::from([
            (
                vec![0, 0],
                Origins {
                    roots: BTreeSet::from([0]),
                    complete: true,
                },
            ),
            (
                vec![1, 0],
                Origins {
                    roots: BTreeSet::from([1]),
                    complete: true,
                },
            ),
        ]),
    );
    statements(&mut checker, "a:row.left.r;part:row.right;b:part.r");
    let a = super::writes::id(&checker, "a");
    let b = super::writes::id(&checker, "b");
    assert_eq!(checker.pointees[&a].roots, BTreeSet::from([0]));
    assert_eq!(checker.pointees[&b].roots, BTreeSet::from([1]));
    checker.mark_derived(1);
    assert!(!checker.derived_local(a));
    assert!(checker.derived_local(b));
    statements(
        &mut checker,
        r#"p:@"proof";|*(row.right.r)|q:p.can_copy<uint32>()"#,
    );
    assert!(checker.queries[0].control);
}

#[test]
pub(crate) fn missing_nested_paths_do_not_reuse_flat_field_origins() {
    let mut checker = Checker::new();
    statements(&mut checker, "x:=false;row:{->inner:{->r:&x}}");
    let row = super::writes::id(&checker, "row");
    checker.record_pointees.insert(
        row,
        BTreeMap::from([(
            vec![0],
            Origins {
                roots: BTreeSet::from([0]),
                complete: true,
            },
        )]),
    );
    statements(&mut checker, "r:row.inner.r");
    let r = super::writes::id(&checker, "r");
    assert!(checker.pointees[&r].roots.is_empty());
    assert!(!checker.pointees[&r].complete);
}
