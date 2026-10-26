use super::super::super::capture::tests::{capture, capture_with};
use super::*;

#[test]
pub(crate) fn record_values_preserve_primary_origins_without_whole_scalar_facts() {
    let (checker, _, _, snapshots) = capture("x:=7;r:{->x;->tag:false;x=9}");
    assert_eq!(snapshots.records.len(), 1);
    let (&point, (block, fact)) = snapshots.records.iter().next().unwrap();
    assert!(!snapshots.expressions.contains_key(&point));
    assert_eq!(checker.bodies[block].parent, Some(point));
    let op = checker
        .emissions
        .values()
        .find(|op| op.targets[0].field.is_none())
        .unwrap();
    let saved = snapshots.emissions[&op.targets[0].id]
        .value
        .as_ref()
        .unwrap();
    let value = fact.value.as_ref().unwrap();
    assert_eq!(value.domain, saved.domain);
    assert_eq!(value.origin, saved.origin);
    assert_eq!(value.local, None);
    assert_eq!(value.version, None);
}

#[test]
pub(crate) fn record_values_keep_unknown_tails_controls_and_unsupported_layouts_unavailable() {
    for source in ["r:{->7;->tag:false;1+2}", "r:{->7;->tag:1+2}"] {
        let (_, _, _, snapshots) = capture(source);
        assert_eq!(snapshots.records.len(), 1);
        assert!(snapshots.records.values().next().unwrap().1.value.is_none());
    }
    for control in [Some(false), Some(true), None] {
        let (_, _, _, snapshots) = capture_with("r:{->7;->tag:false;|true|1}", |obs| {
            for guard in obs.guards.values_mut() {
                guard.control = control;
            }
        });
        assert_eq!(
            snapshots
                .records
                .values()
                .next()
                .unwrap()
                .1
                .value
                .as_ref()
                .map(|value| value.control),
            control
        );
    }
    for source in [
        "r:{->7;->tag:=false}",
        "n:7;r:{->7;->tag:&n}",
        "r:{->7;->tag:\"cat\"}",
    ] {
        assert!(capture(source).3.records.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn record_values_preserve_implicit_null_and_known_bottom() {
    let (_, _, _, snapshots) = capture("r:{->tag:false}");
    assert_eq!(
        snapshots
            .records
            .values()
            .next()
            .unwrap()
            .1
            .value
            .as_ref()
            .unwrap()
            .domain,
        Domain::NULL
    );
    let (_, _, _, snapshots) = capture("f:(n<never>){r:{->7;->tag:false}}");
    assert_eq!(
        snapshots
            .records
            .values()
            .next()
            .unwrap()
            .1
            .value
            .as_ref()
            .unwrap()
            .domain,
        Domain::BOTTOM
    );
}
