use crate::check::Checker;
use crate::check::dependencies::{records::writes::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn composed_reference_fields_map_names_across_destination_positions() {
    for source in [
        "x:=false;source:{->z:&x};result:{->a:7;->source};r:result.z",
        "x:=false;source:{->z:{->r:&x}};result:{->a:7;->source};r:result.z.r",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let origins = &checker.pointees[&id(&checker, "r")];
        assert_eq!(origins.roots, BTreeSet::from([0]));
        assert!(origins.complete);
        checker.mark_derived(0);
        assert!(checker.derived_local(id(&checker, "r")));
    }
}

#[test]
pub(crate) fn composition_snapshots_survive_later_source_field_updates() {
    let source =
        "x:=false;y:=true;source:{->r:=&x};result:{->source};source.r=&y;a:result.r;b:source.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "a")].roots,
        BTreeSet::from([0])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "b")].roots,
        BTreeSet::from([0, 1])
    );
}

#[test]
pub(crate) fn conditional_compositions_merge_all_possible_owners() {
    let source = "x:=false;y:=true;c:=false;left:{->r:&x};right:{->r:&y};result:{|c|->left;|!c|->right};r:result.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert_eq!(origins.roots, BTreeSet::from([0, 1]));
    assert!(origins.complete);
}

#[test]
pub(crate) fn composition_preserves_unknown_origin_and_ordinary_error_boundaries() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "x:=false;source:{->r:{->&x}};result:{->source};r:result.r",
    );
    assert!(!checker.pointees[&id(&checker, "r")].complete);
    let source = "x:=false;source:{->r:&x};result:{->r:&x;->source}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E205");
}

#[test]
pub(crate) fn conditional_composition_keeps_known_owners_when_an_alternative_is_unknown() {
    let source = "x:=false;y:=true;c:=false;left:{->r:&x};right:{->r:{->&y}};result:{|c|->left;|!c|->right};r:result.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert_eq!(origins.roots, BTreeSet::from([0]));
    assert!(!origins.complete);
}

#[test]
pub(crate) fn record_source_locations_distinguish_direct_and_alternative_storage() {
    use super::RecordSource;
    use crate::hir::Stmt;
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "x:=false;row:{->r:&x};copy:row");
    let Stmt::Bind { value, .. } = &stmts[2] else {
        panic!()
    };
    let work = checker.flow.work;
    let RecordSource::Direct(place) = checker.record_source_locations(value, &[0]).unwrap() else {
        panic!()
    };
    assert_eq!(place.root, id(&checker, "row"));
    assert_eq!(place.fields, [0]);
    assert_eq!(checker.flow.work, work);
    let Stmt::Bind { value, .. } = &stmts[1] else {
        panic!()
    };
    let RecordSource::Alternatives(places) = checker.record_source_locations(value, &[0]).unwrap()
    else {
        panic!()
    };
    assert_eq!(places.len(), 1);
    assert!(places[0].fields.is_empty());
    assert_eq!(checker.pointees[&places[0].root].roots, BTreeSet::from([0]));
}

#[test]
pub(crate) fn record_source_locations_distinguish_known_null_from_unknown_calls() {
    use super::RecordSource;
    use crate::hir::Stmt;
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, "<R>:<{r<&boolean>}>;none<R><null>:null");
    let Stmt::Bind { value, .. } = &stmts[0] else {
        panic!()
    };
    assert!(matches!(
        checker.record_source_locations(value, &[0]).unwrap(),
        RecordSource::Empty
    ));
    let mut checker = Checker::new();
    let source = "f<{r<&boolean>}>:(v<&boolean>){->r:v};x:=false;row:f(&x)";
    crate::compile(source).unwrap();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(matches!(
        checker.record_source_locations(value, &[0]).unwrap(),
        RecordSource::Unknown
    ));
    assert_eq!(checker.calls, calls);
}
