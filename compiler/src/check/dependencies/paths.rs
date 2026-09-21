use super::tests::statements;
use crate::check::{Checker, Value};

pub(crate) fn local(checker: &Checker, name: &str) -> usize {
    let Value::Local { id, .. } = &checker.scopes.last().unwrap().values[name] else {
        panic!()
    };
    *id
}

#[test]
pub(crate) fn owned_path_writes_track_rhs_indices_and_control() {
    for (prefix, assignment, read) in [
        ("seed:true;row:{->flag:=false}", "row.flag=seed", "row.flag"),
        (
            "seed:true;row:{->inner:{->flag:=false}}",
            "row.inner.flag=seed",
            "row.inner.flag",
        ),
        (
            "seed:true;row<boolean[2]>:=[false,false]",
            "row[1]=seed",
            "row[1]",
        ),
        (
            "seed:1;row<boolean[2]>:=[false,false]",
            "row[seed]=false",
            "row[2]",
        ),
        (
            "seed:true;row:{->flag:=false}",
            "|seed|row.flag=false",
            "row.flag",
        ),
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, prefix);
        checker.derived.insert(0);
        let id = local(&checker, "row");
        assert!(!checker.derived_local(id));
        statements(&mut checker, assignment);
        statements(&mut checker, &format!("copy:{read};plain:false"));
        assert!(checker.derived_local(id), "{assignment}");
        assert!(
            checker.derived_local(local(&checker, "copy")),
            "{assignment}"
        );
        assert!(!checker.derived_local(local(&checker, "plain")));
    }
}

#[test]
pub(crate) fn marked_path_writes_control_later_queries_without_tainting_other_owners() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "seed:true;row:{->flag:=false;->other:false};plain:{->flag:=false};p:@\"proof\"",
    );
    checker.derived.insert(0);
    statements(
        &mut checker,
        "row.flag=seed;|row.other|r:p.can_copy<uint32>();|plain.flag|s:p.can_copy<uint8>()",
    );
    assert!(checker.queries[0].control);
    assert!(!checker.queries[1].control);
    assert_eq!(
        crate::check::queries::finish(&checker.queries, &checker.query_budgets)
            .unwrap_err()
            .code,
        "E225"
    );
}

#[test]
pub(crate) fn invalid_path_writes_preserve_errors_without_marking_targets() {
    for (prefix, assignment, code) in [
        ("seed:true;row:{->flag:false}", "row.flag=seed", "E305"),
        ("seed:true;row:{->n:=7}", "row.n=seed", "E207"),
        ("seed:true;row:{->n:=7}", "row.missing=seed", "E201"),
    ] {
        let mut checker = Checker::new();
        statements(&mut checker, prefix);
        checker.derived.insert(0);
        let block = crate::parser::parse(assignment).unwrap();
        let error = checker.stmt(&block.stmts[0]).unwrap_err();
        assert_eq!(error.code, code);
        assert!(!checker.derived_local(local(&checker, "row")));
    }
}
