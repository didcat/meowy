use super::super::super::capture::tests::capture;
use super::super::super::guards::Authority;
use super::*;

pub(super) fn setup(source: &str) -> (Checker, Snapshots, PointId, Logical) {
    let (checker, _, _, mut snapshots) = capture(source);
    let point = *checker
        .branch_inputs
        .keys()
        .find(|point| checker.points[**point].kind == PointKind::And)
        .unwrap();
    let (&condition, guard) = snapshots
        .guards
        .iter()
        .find(|(_, guard)| guard.input.point == point)
        .unwrap();
    let Port::Entry(right) = checker.region_edges[&guard.input.arms[0]][0].to else {
        panic!()
    };
    snapshots.expressions.remove(&point).unwrap();
    (
        checker,
        snapshots,
        point,
        Logical {
            condition,
            right,
            or: false,
        },
    )
}

#[test]
pub(crate) fn scalar_logical_boundaries_keep_skipped_unknowns_stopped_inputs_and_bottom_distinct() {
    for (source, expected) in [
        ("false&&(1==1)", Some(false)),
        ("true||(1==1)", Some(true)),
        ("true&&(1==1)", None),
        ("false||(1==1)", None),
        ("f<boolean>:(){->true};true||f()", Some(true)),
        ("f<boolean>:(){->true};false||f()", None),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};false&&stop()",
            None,
        ),
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};true||stop()",
            None,
        ),
    ] {
        let (checker, _, _, snapshots) = capture(source);
        let point = *checker.branch_inputs.keys().next().unwrap();
        assert_eq!(
            snapshots.expressions[&point]
                .value
                .as_ref()
                .map(|value| &value.domain),
            expected.map(Domain::boolean).as_ref(),
            "{source}"
        );
    }
    let (checker, _, _, snapshots) = capture("f:(n<never>){true&&false}");
    let point = *checker.branch_inputs.keys().next().unwrap();
    let value = snapshots.expressions[&point].value.as_ref().unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(
        (value.local, value.version, value.origin),
        (None, None, None)
    );
    let (checker, _, _, snapshots) =
        capture("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop()&&true");
    assert!(
        checker
            .branch_inputs
            .keys()
            .all(|point| !snapshots.expressions.contains_key(point))
    );
}

#[test]
pub(crate) fn scalar_logical_boundaries_keep_missing_values_and_control_unavailable() {
    for fault in 0..4 {
        let (mut checker, mut snapshots, point, logical) = setup("f:(a<boolean>,b<boolean>){a&&b}");
        let left = snapshots.guards[&logical.condition].input.source.input;
        match fault {
            0 => {
                snapshots.expressions.remove(&left);
            }
            1 => {
                snapshots.expressions.remove(&logical.right);
            }
            2 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .control = None
            }
            3 => {
                let row = snapshots.rows.get_mut(&Port::Normal(point)).unwrap();
                row.before = Value::Missing;
                row.after = Value::Missing;
            }
            _ => unreachable!(),
        }
        snapshots
            .logical_expression(point, &logical, &mut checker.flow, Span::default())
            .unwrap();
        assert!(snapshots.expressions[&point].value.is_none());
    }
}

#[test]
pub(crate) fn scalar_logical_boundaries_preserve_each_control_source_in_later_guards() {
    for source in 0..3 {
        let (mut checker, mut snapshots, point, logical) = setup("|true&&false|1");
        let left = snapshots.guards[&logical.condition].input.source.input;
        match source {
            0 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .control = Some(true)
            }
            1 | 2 => {
                snapshots
                    .expressions
                    .get_mut(&if source == 1 { left } else { logical.right })
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .control = true
            }
            _ => unreachable!(),
        }
        snapshots
            .logical_expression(point, &logical, &mut checker.flow, Span::default())
            .unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert!(value.control);
        assert_eq!(value.domain, Domain::boolean(false));
        let (&condition, guard) = snapshots
            .guards
            .iter()
            .find(|(_, guard)| guard.input.source.input == point)
            .unwrap();
        let input = guard.input;
        snapshots.guards.remove(&condition);
        snapshots
            .guard(
                Port::Normal(condition),
                &input,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(snapshots.guards[&condition].authority, Authority::Dependent);
    }
}

#[test]
pub(crate) fn scalar_logical_boundaries_reject_stale_sources_histories_and_shapes_atomically() {
    for fault in 0..12 {
        let (mut checker, mut snapshots, point, mut logical) =
            setup("f:(a<boolean>,b<boolean>){a&&b}");
        let left = snapshots.guards[&logical.condition].input.source.input;
        match fault {
            0 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .point += 1
            }
            1 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .source
                    .owner += 1
            }
            2 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .source
                    .normal = false
            }
            3 => {
                snapshots
                    .guards
                    .get_mut(&logical.condition)
                    .unwrap()
                    .input
                    .source
                    .control = true
            }
            4 => logical.right = left,
            5 => snapshots.rows.get_mut(&Port::Normal(point)).unwrap().before = Value::Missing,
            6 => snapshots.expressions.get_mut(&logical.right).unwrap().owner += 1,
            7 => {
                snapshots
                    .expressions
                    .get_mut(&logical.right)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .domain
                    .null = true
            }
            8 => {
                snapshots
                    .expressions
                    .get_mut(&logical.right)
                    .unwrap()
                    .value
                    .as_mut()
                    .unwrap()
                    .domain
                    .values = Some(Values::Bool(4))
            }
            9 => {
                snapshots
                    .rows
                    .get_mut(&Port::Normal(logical.right))
                    .unwrap()
                    .after = Value::Missing
            }
            10 => {
                snapshots.rows.remove(&Port::Normal(logical.right));
            }
            11 => {
                snapshots
                    .rows
                    .get_mut(&Port::Normal(logical.condition))
                    .unwrap()
                    .owner += 1
            }
            _ => unreachable!(),
        }
        let before = format!("{snapshots:?}");
        assert!(
            snapshots
                .logical_expression(point, &logical, &mut checker.flow, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{snapshots:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_logical_boundaries_reserve_exact_work_and_payload_before_publication() {
    let (mut checker, mut snapshots, point, logical) = setup("true&&false");
    let start = checker.flow.work;
    snapshots
        .logical_expression(point, &logical, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let expected = snapshots.expressions.remove(&point).unwrap();
    let weight = expected.weight();
    let before = format!(
        "{:?}{:?}{:?}",
        snapshots.rows, snapshots.states, snapshots.guards
    );
    for (room, extra, pass) in [
        (weight - 1, 0, false),
        (weight, 1, false),
        (weight, 0, true),
    ] {
        snapshots.parts = room;
        checker.flow.full = false;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        let result =
            snapshots.logical_expression(point, &logical, &mut checker.flow, Span::default());
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(
            snapshots.expressions.get(&point),
            if pass { Some(&expected) } else { None }
        );
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                snapshots.rows, snapshots.states, snapshots.guards
            ),
            before
        );
    }
    checker.flow.work = 0;
    snapshots.parts = MAX_EDGES;
    let before = format!("{snapshots:?}");
    assert!(
        snapshots
            .logical_expression(point, &logical, &mut checker.flow, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{snapshots:?}"), before);
}
