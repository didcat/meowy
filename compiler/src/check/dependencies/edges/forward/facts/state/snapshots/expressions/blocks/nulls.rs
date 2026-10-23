use super::super::super::capture::tests::{capture, capture_with};
use super::*;

#[test]
pub(crate) fn scalar_block_nulls_materialize_only_after_completion_without_changing_slots() {
    let (checker, _, _, snapshots) = capture("x:=1;a:{};b:{x=2};copy:b;a;b;copy");
    let mut origins = Vec::new();
    for (&block, body) in &checker.bodies {
        let Some(parent) = body.parent else { continue };
        let value = snapshots.expressions[&parent].value.as_ref().unwrap();
        assert_eq!(value.domain, Domain::NULL);
        assert_eq!(value.origin, Some(Origin::Expression(body.owner, parent)));
        assert!(!value.control);
        origins.push(value.origin);
        for port in [Port::BlockEntry(block), Port::BlockResult(block)] {
            let Some(View::Known(state)) = snapshots.after(port) else {
                panic!()
            };
            assert_eq!(state.slot(block).unwrap().data, Data::Uninitialized);
        }
    }
    assert_eq!(origins.len(), 2);
    assert_ne!(origins[0], origins[1]);
    let mut copies = Vec::new();
    for (&point, read) in &checker.local_reads {
        if [2, 3].contains(&read.local) {
            copies.push(snapshots.expressions[&point].value.as_ref().unwrap().origin);
        }
    }
    assert_eq!(copies.len(), 3);
    assert!(copies.iter().all(|origin| *origin == copies[0]));
}

#[test]
pub(crate) fn scalar_block_nulls_feed_call_arguments_without_specializing_callees() {
    let (mut checker, _, _, snapshots) = capture("f<boolean>:(n<null>){->true};v:{->f({})};v");
    let call = checker.invocations.values().next().unwrap();
    let arg = snapshots.expressions[&call.args[0]].value.as_ref().unwrap();
    assert_eq!(arg.domain, Domain::NULL);
    assert!(!arg.control);
    let value = snapshots.expressions[&call.point].value.as_ref().unwrap();
    assert_eq!(
        value.domain,
        Domain::scalar(
            crate::check::dependencies::ScalarKind::Bool,
            &mut checker.flow,
            Span::default()
        )
        .unwrap()
        .unwrap()
    );
    let target = checker.points[call.point].block.unwrap();
    let parent = checker.bodies[&target].parent.unwrap();
    assert_eq!(
        snapshots.expressions[&parent]
            .value
            .as_ref()
            .unwrap()
            .domain,
        value.domain
    );
    assert_eq!(
        snapshots.expressions[&parent]
            .value
            .as_ref()
            .unwrap()
            .origin,
        value.origin
    );
}

#[test]
pub(crate) fn scalar_block_nulls_do_not_recover_partial_or_unknown_completion() {
    for (source, known) in [
        ("v:{|true|->null}", true),
        ("v:{|false|->null}", true),
        ("f:(flag<boolean>){v:{|flag|->null};v}", false),
        ("v:{1+2}", false),
        ("v:'out{'out.leave();1}", false),
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let body = checker
            .bodies
            .values()
            .find(|body| body.parent.is_some())
            .unwrap();
        let value = &snapshots.expressions[&body.parent.unwrap()].value;
        assert_eq!(value.is_some(), known, "{source}");
        if let Some(value) = value {
            assert_eq!(value.domain, Domain::NULL);
        }
    }
}

#[test]
pub(crate) fn scalar_block_nulls_retain_completion_control_and_feed_nested_guards() {
    for control in [Some(false), Some(true), None] {
        let (checker, _, _, snapshots) = capture_with("v:{|true|1}", |obs| {
            for input in obs.guards.values_mut() {
                input.control = control;
            }
        });
        let body = checker
            .bodies
            .values()
            .find(|body| body.parent.is_some())
            .unwrap();
        let value = &snapshots.expressions[&body.parent.unwrap()].value;
        assert_eq!(value.as_ref().map(|value| value.control), control);
    }
    let (mut checker, _, _, snapshots) = capture("x:=0;|{n:{};->true}|x=1;x");
    let point = checker.local_reads.keys().next_back().unwrap();
    assert_eq!(
        snapshots.expressions[point].value.as_ref().unwrap().domain,
        Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
    );
}
