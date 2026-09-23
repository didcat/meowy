use super::tests::statements;
use crate::check::{Checker, Value};
use std::collections::BTreeSet;

pub(crate) fn id(checker: &Checker, name: &str) -> usize {
    let Value::Local { id, .. } = checker.scopes.last().unwrap().values[name] else {
        panic!()
    };
    id
}

#[test]
pub(crate) fn shared_element_borrows_and_view_copies_keep_the_container_owner() {
    for source in [
        "x<boolean[2]>:=[false,true];r:&(x[1])",
        "x<boolean[2]>:=[false,true];view:&x;copy:view;r:&(copy[1])",
        "x<boolean[2][2]>:=[[false,true],[true]];view:&x;r:&(view[1][1])",
        "x<{flag<boolean>}[1]>:=[{->flag:false}];view:&(x[1]);r:view.&flag",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let owner = id(&checker, "x");
        let r = id(&checker, "r");
        assert_eq!(
            checker.pointees[&r].roots,
            BTreeSet::from([owner]),
            "{source}"
        );
        assert!(checker.pointees[&r].complete);
        checker.mark_derived(owner);
        assert!(checker.derived_local(r));
        statements(&mut checker, "p:@\"proof\";|*r|q:p.can_copy<uint32>()");
        assert!(checker.queries[0].control);
    }
}

#[test]
pub(crate) fn element_selection_marks_and_unrelated_containers_remain_separate() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "index:1;x<boolean[2]>:=[false,true];y<boolean[1]>:=[false]",
    );
    checker.mark_derived(0);
    statements(&mut checker, "r:&(x[index]);other:&(y[1])");
    assert!(checker.derived_local(id(&checker, "r")));
    assert!(!checker.derived_local(id(&checker, "other")));
    assert_eq!(
        checker.pointees[&id(&checker, "r")].roots,
        BTreeSet::from([id(&checker, "x")])
    );
}

#[test]
pub(crate) fn returned_list_views_do_not_invent_known_owners() {
    let source =
        "f<&boolean[2]>:(v<&boolean[2]>){->v};x<boolean[2]>:=[false,true];view:f(&x);r:&(view[1])";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "r")];
    assert!(!origins.complete);
    assert!(origins.roots.is_empty());
}
