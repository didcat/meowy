use super::super::super::capture::tests::{capture, capture_with};
use super::super::*;

#[test]
pub(crate) fn scalar_block_values_preserve_emitted_origins_across_tail_writes_and_copies() {
    let (mut checker, _, _, snapshots) = capture("x:=7;v:{->x;x=9};copy:v;x=11;v;copy;x");
    let op = checker.emissions.values().next().unwrap();
    let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
    let value = snapshots
        .expressions
        .get(&parent)
        .expect("completed block fact")
        .value
        .as_ref()
        .unwrap();
    let emitted = snapshots.emissions[&op.targets[0].id]
        .value
        .as_ref()
        .unwrap();
    assert_eq!(value.domain, emitted.domain);
    assert_eq!(value.origin, emitted.origin);
    assert_eq!(value.local, None);
    assert_eq!(value.version, None);
    assert!(!value.control);
    for (&point, read) in &checker.local_reads {
        if [1, 2].contains(&read.local) {
            let copy = snapshots.expressions[&point].value.as_ref().unwrap();
            assert_eq!(copy.domain, value.domain);
            assert_eq!(copy.origin, value.origin);
        }
    }
    let point = checker.local_reads.keys().next_back().unwrap();
    assert_eq!(
        snapshots.expressions[point].value.as_ref().unwrap().domain,
        Domain::integer(11, &mut checker.flow, Span::default()).unwrap()
    );
}

#[test]
pub(crate) fn scalar_block_values_follow_nested_results_and_guarded_completion() {
    for source in [
        "x:=7;v:{->{->x};x=9};v",
        "f:(flag<boolean>){v:'out{->7;|flag|'out.leave();1};v}",
    ] {
        let (mut checker, _, _, snapshots) = capture(source);
        for op in checker.emissions.values() {
            let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
            let value = snapshots
                .expressions
                .get(&parent)
                .expect("completed block fact")
                .value
                .as_ref()
                .unwrap();
            assert_eq!(
                value.domain,
                Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
            );
        }
    }
}

#[test]
pub(crate) fn scalar_block_values_require_completion_history_after_a_known_emission() {
    for source in [
        "f:(text<string>){};v:{->7;f(\"cat\")}",
        "v:{->7;1+2}",
        "v:'out{->7;'out.leave();1}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let op = checker.emissions.values().next().unwrap();
        let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
        assert!(snapshots.emissions[&op.targets[0].id].value.is_some());
        assert!(
            snapshots
                .expressions
                .get(&parent)
                .expect("unavailable block fact")
                .value
                .is_none(),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_block_values_retain_tail_control_separately_from_emission_control() {
    for control in [Some(false), Some(true), None] {
        let (checker, _, _, snapshots) = capture_with("v:{->7;|true|1}", |obs| {
            for input in obs.guards.values_mut() {
                input.control = control;
            }
        });
        let op = checker.emissions.values().next().unwrap();
        let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
        let fact = snapshots
            .expressions
            .get(&parent)
            .expect("completed block fact");
        assert_eq!(fact.value.as_ref().map(|value| value.control), control);
        assert!(
            !snapshots.emissions[&op.targets[0].id]
                .value
                .as_ref()
                .unwrap()
                .control
        );
    }
}

#[test]
pub(crate) fn scalar_block_values_keep_guarded_leaves_and_partial_initialization_distinct() {
    for (flag, expected) in [("true", Some(1)), ("false", Some(2)), ("param", None)] {
        let prefix = if flag == "param" {
            "f:(flag<boolean>)".to_owned()
        } else {
            format!("flag:{flag};")
        };
        let source = format!("{prefix}{{v:'out{{|flag|'out->1;|flag|'out.leave();'out->2}};v}}");
        let (mut checker, _, _, snapshots) = capture(&source);
        let op = checker.emissions.values().next().unwrap();
        let parent = checker.bodies[&op.targets[0].block].parent.unwrap();
        let actual = snapshots.expressions[&parent]
            .value
            .as_ref()
            .map(|value| value.domain.clone());
        assert_eq!(
            actual,
            expected.map(|n| Domain::integer(n, &mut checker.flow, Span::default()).unwrap())
        );
    }
}

#[test]
pub(crate) fn scalar_block_values_distinguish_bottom_from_unsupported_shapes() {
    let (checker, _, _, snapshots) = capture("f:(n<never>){v:{->7}}");
    let body = checker
        .bodies
        .values()
        .find(|body| body.parent.is_some())
        .unwrap();
    let value = snapshots.expressions[&body.parent.unwrap()]
        .value
        .as_ref()
        .unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(value.origin, None);
    for source in [
        "false&&{->true}",
        "v:{->\"cat\"}",
        "v:{->1.5}",
        "v:{->field:1}",
        "x:1;v:{->&x}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let body = checker
            .bodies
            .values()
            .find(|body| body.parent.is_some())
            .unwrap();
        assert!(
            !snapshots.expressions.contains_key(&body.parent.unwrap()),
            "{source}"
        );
    }
}
