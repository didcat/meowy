use super::super::super::capture::tests::{capture, capture_with};
use super::*;

#[test]
pub(crate) fn primary_values_keep_original_emissions_across_tail_writes_and_operand_steps() {
    for source in [
        "x:=7;-(({->x;->tag:false;x=9}))",
        "x:=7;{->x;->tag:false;x=9}+{->x;->tag:true;x=11}",
    ] {
        let (mut checker, _, reports, snapshots) = capture(source);
        assert_eq!(snapshots.projections.len(), reports.slot_uses.len());
        for (&port, fact) in &snapshots.projections {
            let Port::Projection { point, step } = port else {
                panic!()
            };
            let value = fact.value.as_ref().unwrap();
            let block = reports.slot_uses[&port].1.block;
            let op = checker
                .emissions
                .values()
                .find(|op| op.targets[0].block == block && op.targets[0].field.is_none())
                .unwrap();
            let emitted = snapshots.emissions[&op.targets[0].id]
                .value
                .as_ref()
                .unwrap();
            assert_eq!(value.domain, emitted.domain);
            assert_eq!(value.origin, emitted.origin);
            assert_eq!(
                value.domain,
                Domain::integer(
                    if step == 0 { 7 } else { 9 },
                    &mut checker.flow,
                    Span::default()
                )
                .unwrap()
            );
            assert_eq!(value.local, None);
            assert_eq!(value.version, None);
            assert!(!value.control);
            assert_eq!(snapshots.before(port), snapshots.after(port));
            assert!(!snapshots.expressions.contains_key(&point));
            assert!(
                !snapshots
                    .expressions
                    .contains_key(&checker.bodies[&block].parent.unwrap())
            );
        }
    }
}

#[test]
pub(crate) fn primary_values_preserve_boolean_parameter_and_nested_leave_histories() {
    for source in [
        "!(({->true;->tag:7}))",
        "f:(n<uint64>){n=={->n;->tag:false}}",
        "-'out{->7;->tag:false;{'out.leave();1};2}",
    ] {
        let (checker, _, reports, snapshots) = capture(source);
        assert_eq!(snapshots.projections.len(), 1);
        for (&port, fact) in &snapshots.projections {
            let block = reports.slot_uses[&port].1.block;
            let op = checker
                .emissions
                .values()
                .find(|op| op.targets[0].block == block && op.targets[0].field.is_none())
                .unwrap();
            let emitted = snapshots.emissions[&op.targets[0].id]
                .value
                .as_ref()
                .unwrap();
            let value = fact.value.as_ref().unwrap();
            assert_eq!(value.domain, emitted.domain);
            assert_eq!(value.origin, emitted.origin);
        }
    }
}

#[test]
pub(crate) fn primary_values_keep_projection_only_and_unknown_tail_boundaries() {
    for (source, known) in [
        ("d:@\"debug\";{->7;->tag:false}+{d.panic(\"stop\")}", false),
        (
            "d:@\"debug\";f:(flag<boolean>) 'out{|flag|'out.leave();{->7;->tag:false}+{d.panic(\"stop\")}}",
            true,
        ),
    ] {
        let (mut checker, _, reports, snapshots) = capture(source);
        let (&port, fact) = snapshots.projections.iter().next().unwrap();
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        assert_eq!(
            fact.value.as_ref().map(|value| value.domain.clone()),
            known.then(|| Domain::integer(7, &mut checker.flow, Span::default()).unwrap())
        );
        let Effect::Binary(op) = &reports.effects[&point].1 else {
            panic!()
        };
        assert!(!op.operation && !op.result);
        assert!(!snapshots.rows.contains_key(&Port::Operation(point)));
    }
    for source in ["-{->7;->tag:false;1+2}", "-{->7;->tag:1+2}"] {
        let (_, _, _, snapshots) = capture(source);
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
pub(crate) fn primary_values_require_known_completion_control_and_retain_bottom() {
    for control in [Some(false), Some(true), None] {
        let (_, _, _, snapshots) = capture_with("-{->7;->tag:false;|true|1}", |obs| {
            for input in obs.guards.values_mut() {
                input.control = control;
            }
        });
        let fact = snapshots.projections.values().next().unwrap();
        assert_eq!(fact.value.as_ref().map(|value| value.control), control);
    }
    let (_, _, _, snapshots) = capture("f:(n<never>){-{->7;->tag:false}}");
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
}
