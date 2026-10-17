use super::{checked, prepared};
use crate::borrow::{MAX_FACT_ORIGINS, Source, Step};
use crate::flow::{FALSE, TRUE};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn call_input_snapshots_keep_nested_calls_and_unused_function_frames_separate() {
    let (program, facts) =
        checked("id<&int32>:(p<&int32>){->p};f:(p<&int32>){id(p)};n:1;id(id(&n))");
    assert_eq!(facts.call_inputs.len(), 3);
    let function = &program.functions[1];
    let mut outer = 0;
    let mut inner = 0;
    for snapshot in facts.call_inputs.values() {
        assert_eq!(snapshot.header.function, program.functions[0].id);
        assert_eq!(snapshot.normal, 1);
        let source = &snapshot.args[0].1.origins[0].source;
        if snapshot.header.body == program.body.id {
            outer += 1;
            assert!(matches!(source, Source::Local { .. }));
        } else {
            inner += 1;
            assert_eq!(snapshot.header.body, function.body.id);
            assert!(matches!(source, Source::Input { id, .. } if *id == function.params[0]));
        }
    }
    assert_eq!((outer, inner), (2, 1));
}

#[test]
pub(crate) fn call_input_snapshots_preserve_guarded_union_alternatives() {
    let (_, facts) = checked(
        "sink:(p<&int32><null>){};f:(flag<boolean>){x:1;p<&int32><null>:{|flag|->&x;|!flag|->null};|flag|sink(p)}",
    );
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    assert_ne!(snapshot.header.entered, TRUE);
    assert_ne!(snapshot.header.entered, FALSE);
    let state = &snapshot.args[0].1;
    assert_eq!(state.active.len(), 2);
    assert!(
        state
            .active
            .iter()
            .all(|active| active.guard != TRUE && active.guard != FALSE)
    );
    assert_eq!(state.origins.len(), 1);
    assert!(matches!(
        state.origins[0].component.as_slice(),
        [Step::Variant(_)]
    ));
    assert_ne!(state.origins[0].guard, TRUE);
}

#[test]
pub(crate) fn call_input_snapshots_preserve_alias_storage_and_caller_body() {
    let (program, facts) = checked("sink:(p<&int32>){};r:{->n:1;sink(&n)}");
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    assert_eq!(snapshot.header.body, program.body.id);
    let Source::Slot { target, fields, .. } = &snapshot.args[0].1.origins[0].source else {
        panic!()
    };
    assert_ne!(*target, program.body.id);
    assert!(fields.is_empty());
}

#[test]
pub(crate) fn call_input_snapshots_publish_converged_restart_origins_once() {
    let (program, facts) = checked(
        "sink:(p<&int32>){};a:1;b:2;r:=&a;n:=0;'loop{sink(r);r=&b;n=n+1;|n<2|'loop.restart()}",
    );
    assert!(!facts.headers.is_empty());
    assert_eq!(facts.call_inputs.len(), 1);
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    assert_eq!(snapshot.header.body, program.body.id);
    let roots = snapshot.args[0]
        .1
        .origins
        .iter()
        .map(|origin| {
            let Source::Local { id, .. } = origin.source else {
                panic!()
            };
            id
        })
        .collect::<BTreeSet<_>>();
    let expected = program.body.stmts[..2]
        .iter()
        .map(|stmt| {
            let Stmt::Bind { id, .. } = stmt else {
                panic!()
            };
            *id
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(roots, expected);
}

#[test]
pub(crate) fn call_input_snapshots_keep_abstract_allocator_bounds_without_inventing_loans() {
    let (program, facts) = checked(
        "m:@\"memory\";bound<m.Allocator>:(p<&int32>){->m.heap};sink:(a<m.Allocator>){};f:(p<&int32>){sink(bound(p))}",
    );
    let snapshot = facts
        .call_inputs
        .values()
        .find(|snapshot| snapshot.header.function == program.functions[1].id)
        .unwrap();
    let state = &snapshot.args[0].1;
    assert!(state.origins.is_empty());
    assert_eq!(state.bounds.len(), 1);
    assert!(
        matches!(state.bounds[0].source, Source::Input { id, .. } if id == program.functions[2].params[0])
    );
    assert_eq!(state.present, TRUE);
    assert_eq!(state.proof, TRUE);
}

#[test]
pub(crate) fn call_input_snapshots_share_the_exact_body_fact_budget() {
    let source = "f<&int32>:(p<&int32>){->p};n:1;f(&n)";
    let (program, mut checker) = prepared(source);
    let (_, cost) = crate::borrow::replay::body(
        &program,
        &mut checker.flow,
        &checker.proofs,
        &program.body,
        &[],
        0,
    )
    .unwrap();
    for extra in [0, 1] {
        let (program, mut checker) = prepared(source);
        let result = crate::borrow::replay::body(
            &program,
            &mut checker.flow,
            &checker.proofs,
            &program.body,
            &[],
            MAX_FACT_ORIGINS - cost + extra,
        );
        if extra == 0 {
            let (facts, used) = result.unwrap();
            assert_eq!(used, MAX_FACT_ORIGINS);
            assert_eq!(facts.call_inputs.len(), 1);
            assert_eq!(facts.calls.len(), 1);
            assert_eq!(facts.returns.len(), 1);
        } else {
            let Err(error) = result else { panic!() };
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("budget"));
        }
    }
}
