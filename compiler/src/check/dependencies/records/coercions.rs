use super::*;
use crate::check::dependencies::tests::statements;
use std::collections::BTreeSet;

#[test]
pub(crate) fn nullable_record_wrapping_and_narrowing_keep_owners() {
    for tail in [
        "row<Row><null>:source;|row<Row>|r:row.r",
        "row<Row><null>:=null;row=source;|row<Row>|r:row.r",
        "row<Row><null>:=source;row=null;row=source;|row<Row>|r:row.r",
        "row:{->inner<Row><null>:source};|row.inner<Row>|r:row.inner.r",
    ] {
        let source = format!("<Row>:<{{r<&boolean>}}>;x:=false;source:{{->r:&x}};{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let id = checker.locals.len() - 1;
        assert_eq!(checker.pointees[&id].roots, BTreeSet::from([0]), "{tail}");
        assert!(checker.pointees[&id].complete, "{tail}");
        checker.mark_derived(0);
        assert!(checker.derived_local(id));
    }
}

#[test]
pub(crate) fn known_null_has_no_reference_owners() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<Row>:<{r<&boolean>}>;row<Row><null>:null;copy:row",
    );
    for fields in checker.record_pointees.values() {
        assert_eq!(fields.len(), 1);
        assert!(
            fields
                .values()
                .all(|origins| origins.complete && origins.roots.is_empty())
        );
    }
    assert_eq!(checker.record_pointees.len(), 2);
}

#[test]
pub(crate) fn heterogeneous_record_unions_do_not_share_field_indices() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;source:{->r:&x};wide<A><B>:source;|wide<A>|r:wide.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(!origins.complete);
    assert!(origins.roots.is_empty());
}

#[test]
pub(crate) fn nullable_wrappers_do_not_make_unknown_initializers_complete() {
    let source = "<Row>:<{r<&boolean>}>;x:=false;source:{->r:{->&x}};row<Row><null>:source;|row<Row>|r:row.r";
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&(checker.locals.len() - 1)].complete);
}
