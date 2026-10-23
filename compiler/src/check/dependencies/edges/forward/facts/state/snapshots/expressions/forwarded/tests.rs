use super::super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_group_values_preserve_source_domains_origins_and_storage() {
    let (checker, _, _, snapshots) = capture("((7));(false);(null);(1.5);f:(n<never>){(7)}");
    for (&point, group) in &checker.group_inputs {
        assert_eq!(
            snapshots.expressions[&point],
            snapshots.expressions[&group.input]
        );
        assert_eq!(
            snapshots.before(Port::Normal(point)),
            snapshots.after(Port::Normal(group.input))
        );
        assert_eq!(
            snapshots.after(Port::Normal(point)),
            snapshots.after(Port::Normal(group.input))
        );
    }
}

#[test]
pub(crate) fn scalar_group_values_leave_missing_unsupported_and_stopped_inputs_unavailable() {
    let (checker, _, _, snapshots) =
        capture("(1+2);({->7});x:8;(9);d:@\"debug\";(d.panic(\"stop\"));(10)");
    for point in checker.group_inputs.keys() {
        if let Some(fact) = snapshots.expressions.get(point) {
            assert!(fact.value.is_none());
        } else {
            assert!(snapshots.before(Port::Normal(*point)).is_none());
        }
    }
}

#[test]
pub(crate) fn scalar_group_values_bound_copies_preserve_controls_and_reject_stale_rows() {
    let (mut checker, _, _, mut snapshots) = capture("(7)");
    let (&point, group) = checker.group_inputs.first_key_value().unwrap();
    let mut forward = Forward {
        owner: group.owner,
        input: Port::Normal(group.input),
        control: true,
    };
    let expected = snapshots.expressions.remove(&point).unwrap();
    let start = checker.flow.work;
    snapshots
        .forward_expression(point, &forward, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let value = snapshots.expressions.remove(&point).unwrap();
    assert!(value.value.as_ref().unwrap().control);
    assert_eq!(
        value.value.as_ref().unwrap().origin,
        expected.value.as_ref().unwrap().origin
    );
    let before = format!(
        "{:?}{:?}{:?}",
        snapshots.states, snapshots.rows, snapshots.expressions
    );
    for (room, extra, pass) in [
        (value.weight() - 1, 0, false),
        (value.weight(), 1, false),
        (value.weight(), 0, true),
    ] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            snapshots
                .forward_expression(point, &forward, &mut checker.flow, Span::default())
                .is_ok(),
            pass
        );
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(
            snapshots.expressions.remove(&point),
            pass.then_some(Fact {
                owner: value.owner,
                value: value.value.clone()
            })
        );
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                snapshots.states, snapshots.rows, snapshots.expressions
            ),
            before
        );
    }
    checker.flow.work = 0;
    checker.flow.full = false;
    snapshots.parts = MAX_EDGES;
    forward.owner += 1;
    assert!(
        snapshots
            .forward_expression(point, &forward, &mut checker.flow, Span::default())
            .is_err()
    );
    forward.owner -= 1;
    let row = snapshots.rows.get_mut(&Port::Normal(point)).unwrap();
    row.before = Value::Missing;
    row.after = Value::Missing;
    snapshots
        .forward_expression(point, &forward, &mut checker.flow, Span::default())
        .unwrap();
    assert!(
        snapshots
            .expressions
            .remove(&point)
            .unwrap()
            .value
            .is_none()
    );
    snapshots.rows.remove(&Port::Normal(group.input));
    assert!(
        snapshots
            .forward_expression(point, &forward, &mut checker.flow, Span::default())
            .is_err()
    );
}
