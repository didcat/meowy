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
pub(crate) fn mutable_fields_records_and_unknown_initializers_stay_incomplete() {
    for source in [
        "x:=false;row:{->r:=&x};r:row.r",
        "x:=false;row:{->r:{->&x}};r:row.r",
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let id = checker.locals.len() - 1;
        assert!(!checker.pointees[&id].complete, "{source}");
    }
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
