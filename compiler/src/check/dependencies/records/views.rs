use super::writes::id;
use super::*;
use crate::check::dependencies::tests::statements;
use std::collections::BTreeSet;

#[test]
pub(crate) fn aggregate_view_fields_retain_container_owners_through_copies() {
    for source in [
        "x<boolean[2]>:=[false,true];row:{->view:&x};copy:row;view:copy.view;r:&(view[1])",
        "x<boolean[2]>:=[false,true];row:{->inner:{->view:&x}};copy:{->row};r:&(copy.inner.view[1])",
        "x:{->flag:false};row:{->view:&x};view:row.view;r:view.&flag",
        "<Box>:<{view<&boolean[2]>}>;x<boolean[2]>:=[false,true];row<Box><null>:{->view:&x};|row<Box>|r:&(row.view[1])",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let owner = id(&checker, "x");
        let r = checker.locals.len() - 1;
        assert_eq!(
            checker.pointees[&r].roots,
            BTreeSet::from([owner]),
            "{source}"
        );
        assert!(checker.pointees[&r].complete);
        checker.mark_derived(owner);
        assert!(checker.derived_local(r));
    }
}

#[test]
pub(crate) fn mutable_view_field_updates_keep_prior_record_snapshots() {
    for update in ["row.view=&y", "|condition|row.view=&y"] {
        let source = format!(
            "x:{{->flag:false}};y:{{->flag:true}};condition:=false;row:{{->view:=&x}};old:row;{update};a:row.view;b:old.view;r:a.&flag;s:b.&flag"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        assert_eq!(
            checker.pointees[&id(&checker, "r")].roots,
            BTreeSet::from([x, y])
        );
        assert_eq!(
            checker.pointees[&id(&checker, "s")].roots,
            BTreeSet::from([x])
        );
        assert!(checker.pointees[&id(&checker, "r")].complete);
    }
}

#[test]
pub(crate) fn returned_and_reference_bearing_view_fields_remain_incomplete() {
    let source = "f<&boolean[2]>:(v<&boolean[2]>){->v};x<boolean[2]>:=[false,true];row:{->view:f(&x)};r:&(row.view[1])";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "r")].complete);
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "n:false;x:{->r:&n};row:{->view:&x};copy:row.view",
    );
    assert!(!checker.pointees.contains_key(&id(&checker, "copy")));
    let errors = crate::compile("x<boolean[1]>:=[false];row:{->view:=&x}").unwrap_err();
    assert_eq!(errors[0].code, "B001");
    assert!(
        errors[0]
            .message
            .contains("mutable reference-bearing record fields")
    );
}
