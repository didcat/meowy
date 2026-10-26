use super::super::super::super::capture::tests::{capture, capture_with};
use super::*;

#[test]
pub(crate) fn stored_primary_values_preserve_original_values_across_copies_and_tail_writes() {
    for source in [
        "x:=7;r:{->x;->tag:false;x=9};copy:((r));x=11;v<int32>:copy",
        "x:=7;r:{->x;->tag:false;x=9};copy:r;x=11;r+copy",
        "x:=7;r:{->x;->tag:false;x=9};x=11;-(r~<{-><int32>;tag<boolean>}>)",
    ] {
        let (mut checker, _, _, snapshots) = capture(source);
        assert!(!snapshots.projections.is_empty());
        let op = checker
            .emissions
            .values()
            .find(|op| op.targets[0].field.is_none())
            .unwrap();
        let saved = snapshots.emissions[&op.targets[0].id]
            .value
            .as_ref()
            .unwrap();
        for (&port, fact) in &snapshots.projections {
            let value = fact.value.as_ref().unwrap();
            assert_eq!(
                value.domain,
                Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
            );
            assert_eq!(value.origin, saved.origin);
            assert_eq!(value.local, None);
            assert_eq!(value.version, None);
            assert!(!value.control);
            assert_eq!(snapshots.before(port), snapshots.after(port));
            let Port::Projection { point, .. } = port else {
                panic!()
            };
            assert!(!snapshots.expressions.contains_key(&point));
        }
    }
}

#[test]
pub(crate) fn stored_primary_values_keep_unknown_current_control_and_state_unavailable() {
    for source in [
        "r:{->7;->tag:false};1+2;v<int32>:r",
        "r:{->7;->tag:false};|true|1;v<int32>:r",
    ] {
        let (_, _, _, snapshots) = capture_with(source, |obs| {
            for guard in obs.guards.values_mut() {
                guard.control = None;
            }
        });
        assert_eq!(snapshots.projections.len(), 1);
        assert!(
            snapshots
                .projections
                .values()
                .next()
                .unwrap()
                .value
                .is_none()
        );
    }
}

#[test]
pub(crate) fn stored_primary_values_preserve_bottom_and_do_not_specialize_whole_record_equality() {
    let (_, _, _, snapshots) = capture("f:(n<never>){r:{->7;->tag:false};v<int32>:r}");
    let value = snapshots
        .projections
        .values()
        .next()
        .unwrap()
        .value
        .as_ref()
        .unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(value.origin, None);
    let (_, _, _, snapshots) = capture("r:{->7;->tag:false};copy:r;r==copy");
    assert!(snapshots.projections.is_empty());
}
