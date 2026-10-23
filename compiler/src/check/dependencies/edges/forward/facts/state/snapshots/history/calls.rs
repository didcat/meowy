use super::super::capture::tests::{capture, capture_with};
use super::super::*;

#[test]
pub(crate) fn scalar_completion_calls_retain_independent_scalar_tail_history() {
    for source in [
        "f:(){};v:{->7;f()}",
        "f<boolean>:(flag<boolean>){->flag};v:{->7;f(true)}",
        "f<int32>:(){->f()};v:{->7;f()}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = checker
            .bodies
            .iter()
            .find(|(_, body)| body.owner == 0 && body.parent.is_some())
            .unwrap()
            .0;
        assert_eq!(
            snapshots.rows[&Port::BlockResult(*target)].control,
            Some(false),
            "{source}"
        );
        for call in checker.invocations.values().filter(|call| call.owner == 0) {
            assert_eq!(
                snapshots.rows[&Port::Normal(call.point)].control,
                Some(false)
            );
        }
    }
}

#[test]
pub(crate) fn scalar_completion_calls_keep_unknown_and_derived_tails_separate_from_emissions() {
    for control in [Some(false), Some(true), None] {
        let (checker, _, _, snapshots) = capture_with("f<boolean>:(){->true};v:{->7;f()}", |obs| {
            for entry in obs.call_controls.values_mut() {
                *entry = control;
            }
        });
        let call = checker.invocations.values().next().unwrap();
        let target = checker.points.get(call.point).unwrap().block.unwrap();
        assert_eq!(snapshots.rows[&Port::Normal(call.point)].control, control);
        assert_eq!(snapshots.rows[&Port::BlockResult(target)].control, control);
        let emission = checker
            .emissions
            .values()
            .find(|op| op.owner == 0)
            .unwrap()
            .targets[0]
            .id;
        assert_eq!(
            snapshots.rows[&Port::Emission(emission)].control,
            Some(false)
        );
        assert!(
            !snapshots.emissions[&emission]
                .value
                .as_ref()
                .unwrap()
                .control
        );
    }
}

#[test]
pub(crate) fn scalar_completion_calls_do_not_infer_dependencies_from_callee_bodies() {
    for source in [
        "f:(text<string>){};v:{->7;f(\"cat\")}",
        "f:(){->field:1};v:{->7;f()}",
        "f:(flag<boolean>){};v:{->7;f(1==1)}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let call = checker.invocations.values().next().unwrap();
        let block = checker.points.get(call.point).unwrap().block.unwrap();
        assert_eq!(
            snapshots.rows[&Port::BlockResult(block)].control,
            None,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_completion_calls_require_observed_result_sources() {
    let (checker, _, _, snapshots) = capture_with("f:(){};v:{->7;f()}", |obs| {
        for port in obs.call_controls.keys() {
            obs.sources.remove(port);
        }
    });
    let call = checker.invocations.values().next().unwrap();
    let block = checker.points.get(call.point).unwrap().block.unwrap();
    assert!(!snapshots.expressions.contains_key(&call.point));
    assert_eq!(snapshots.rows[&Port::BlockResult(block)].control, None);
}
