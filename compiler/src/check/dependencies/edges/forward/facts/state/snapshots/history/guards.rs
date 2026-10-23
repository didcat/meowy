use super::super::capture::tests::{capture, capture_with};
use super::super::*;

#[test]
pub(crate) fn scalar_completion_controls_follow_guards_leaves_and_historical_copies() {
    for (source, expected) in [
        (
            "flag:=false;copy:flag;flag=true;v:'out{->7;|copy|'out.leave();1}",
            Some(false),
        ),
        (
            "f:(flag<boolean>)'out{->7;|flag|'out.leave();1}",
            Some(false),
        ),
        ("v:{->7;|true|1}", Some(false)),
        ("v:{->7;|1==1|1}", None),
        ("v:{->7;false&&(1==1)}", Some(false)),
        ("v:{->7;true&&(1==1)}", None),
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = checker.emissions.values().next().unwrap().targets[0].block;
        assert_eq!(
            snapshots.rows[&Port::BlockResult(target)].control,
            expected,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn scalar_completion_controls_preserve_derived_and_missing_guard_evidence() {
    for control in [Some(false), Some(true), None] {
        let (checker, _, _, snapshots) = capture_with("v:'out{->7;|true|'out.leave();1}", |obs| {
            for input in obs.guards.values_mut() {
                input.control = control;
            }
        });
        let target = checker.emissions.values().next().unwrap().targets[0].block;
        assert_eq!(snapshots.rows[&Port::BlockResult(target)].control, control);
        let (&point, guard) = snapshots.guards.iter().next().unwrap();
        if control == Some(true) {
            assert_eq!(guard.authority, guards::Authority::Dependent);
            for &arm in &guard.input.arms {
                let Some(View::Known(state)) = snapshots.before(Port::Entry(arm)) else {
                    panic!()
                };
                assert_ne!(state.reach, FALSE);
            }
        }
        assert_eq!(snapshots.rows[&Port::Normal(point)].control, control);
        let emission = checker.emissions.values().next().unwrap().targets[0].id;
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
pub(crate) fn scalar_completion_controls_exclude_bottom_but_keep_unknown_live_tails() {
    for (source, expected) in [
        ("v:{->7;|false|1+2}", Some(false)),
        ("v:{->7;|true|1+2}", None),
        ("f:(flag<boolean>){->7;|flag|1+2}", None),
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let target = checker.emissions.values().next().unwrap().targets[0].block;
        assert_eq!(
            snapshots.rows[&Port::BlockResult(target)].control,
            expected,
            "{source}"
        );
    }
}
