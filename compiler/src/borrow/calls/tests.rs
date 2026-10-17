use crate::borrow::{Facts, Source};
use crate::hir::{Program, Stmt};

pub(super) fn checked(source: &str) -> (Program, Facts) {
    crate::compile(source).unwrap();
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = crate::check::Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    checker.proofs.conditions = checker.guards.clone();
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    (program, facts)
}

#[test]
pub(crate) fn call_input_snapshots_retain_unused_primitive_reference_and_aggregate_inputs() {
    for (ty, arg, borrowed) in [
        ("int32", "1", false),
        ("&int32", "&n", true),
        ("&!int32", "&!n", true),
        ("{p<&int32>}", "{->p:&n}", true),
    ] {
        let (program, facts) = checked(&format!("f:(unused<{ty}>){{}};n:=1;f({arg})"));
        let (&site, snapshot) = facts.call_inputs.first_key_value().unwrap();
        assert_eq!(snapshot.header.function, program.functions[0].id);
        assert_eq!(snapshot.header.body, program.body.id);
        assert_eq!(snapshot.header.count, 1);
        assert_eq!(snapshot.normal, 1);
        assert_eq!(snapshot.args.len(), 1);
        assert_eq!(!snapshot.args[0].origins.is_empty(), borrowed);
        assert!(facts.calls[&site].origins.is_empty());
    }
}

#[test]
pub(crate) fn call_input_snapshots_freeze_before_later_argument_writes() {
    let (program, facts) = checked("f:(p<&int32>,unused<int32>){};a:1;b:2;r:=&a;f(r,{r=&b;->0})");
    let Stmt::Bind { id, .. } = program.body.stmts[0] else {
        panic!()
    };
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    assert_eq!(snapshot.args.len(), 2);
    assert_eq!(snapshot.normal, 2);
    assert_eq!(
        snapshot.args[0].origins[0].source,
        Source::Local { id, fields: vec![] }
    );
}

#[test]
pub(crate) fn call_input_snapshots_keep_stopped_prefixes_and_empty_calls() {
    let (_, facts) = checked(
        "d:@\"debug\";f:(a<&int32>,b<int32>,c<&!int32>){};x:1;y:=2;f(&x,d.panic(\"stop\"),&!y)",
    );
    let (&site, snapshot) = facts.call_inputs.first_key_value().unwrap();
    assert_eq!(snapshot.header.count, 3);
    assert_eq!(snapshot.args.len(), 2);
    assert_eq!(snapshot.normal, 1);
    assert_eq!(facts.calls[&site].present, crate::flow::FALSE);
    let (_, facts) = checked("f:(){};f()");
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    assert_eq!(snapshot.header.count, 0);
    assert_eq!(snapshot.normal, 0);
    assert!(snapshot.args.is_empty());
}
