use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup(
    source: &str,
    check: impl FnOnce(&mut Checker, &mut Snapshots, PointId, &mut (usize, Effect)),
) {
    let (mut checker, _, reports, mut snapshots) = capture(source);
    let point = *checker
        .coercions
        .iter()
        .find(|(_, op)| op.scalar.is_some())
        .unwrap()
        .0;
    let mut source = reports.effects[&point].clone();
    snapshots.parts += snapshots.expressions.remove(&point).unwrap().weight();
    check(&mut checker, &mut snapshots, point, &mut source);
}

#[test]
pub(crate) fn nullable_conversion_publication_reserves_exact_work_and_payload_atomically() {
    for source in [
        "n:=7;v<int32><null>:n",
        "v<boolean><null>:true",
        "v<uint64><null>:null",
        "f:(n<uint64>){v<uint64><null>:n}",
    ] {
        setup(source, |checker, snapshots, point, source| {
            let start = checker.flow.work;
            snapshots
                .converted_expression(point, source, &mut checker.flow, Span::default())
                .unwrap();
            let cost = checker.flow.work - start;
            let expected = snapshots.expressions.remove(&point).unwrap();
            for (room, extra, pass) in [
                (expected.weight() - 1, 0, false),
                (expected.weight(), 1, false),
                (expected.weight(), 0, true),
            ] {
                snapshots.parts = room;
                checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                checker.flow.full = false;
                let saved = format!("{snapshots:?}");
                let result = snapshots.converted_expression(
                    point,
                    source,
                    &mut checker.flow,
                    Span::default(),
                );
                assert_eq!(result.is_ok(), pass);
                if pass {
                    assert_eq!(snapshots.expressions[&point], expected);
                    assert_eq!(snapshots.parts, 0);
                } else {
                    assert!(result.unwrap_err().message.contains("budget"));
                    assert_eq!(format!("{snapshots:?}"), saved);
                }
            }
            checker.flow.work = 0;
            checker.flow.full = false;
            snapshots.parts = MAX_EDGES;
            let saved = format!("{snapshots:?}");
            assert!(
                snapshots
                    .converted_expression(point, source, &mut checker.flow, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{snapshots:?}"), saved);
        });
    }
}

#[test]
pub(crate) fn nullable_conversion_values_keep_missing_control_sources_and_incompatible_domains_unavailable()
 {
    for fault in 0..8 {
        setup(
            "n:=7;v<int32><null>:n",
            |checker, snapshots, point, source| {
                let Effect::Coercion(op) = &mut source.1 else {
                    panic!()
                };
                let input = op.input;
                op.control = true;
                match fault {
                    0 => {
                        snapshots.expressions.remove(&input);
                    }
                    1 => snapshots.expressions.get_mut(&input).unwrap().value = None,
                    2 => {
                        op.operation = false;
                        snapshots.rows.remove(&Port::Operation(point));
                        let row = snapshots.rows.get_mut(&Port::Normal(point)).unwrap();
                        row.before = Value::Missing;
                        row.after = Value::Missing;
                        row.control = None;
                    }
                    3 => {
                        snapshots.rows.remove(&Port::Normal(input));
                        snapshots.expressions.remove(&input);
                        for port in [Port::Operation(point), Port::Normal(point)] {
                            let row = snapshots.rows.get_mut(&port).unwrap();
                            row.before = Value::Missing;
                            row.after = Value::Missing;
                            row.control = None;
                        }
                    }
                    4 => {
                        for port in [
                            Port::Normal(input),
                            Port::Operation(point),
                            Port::Normal(point),
                        ] {
                            snapshots.rows.get_mut(&port).unwrap().control = None;
                        }
                    }
                    5..=7 => {
                        snapshots
                            .expressions
                            .get_mut(&input)
                            .unwrap()
                            .value
                            .as_mut()
                            .unwrap()
                            .domain = match fault {
                            5 => Domain::boolean(true),
                            6 => Domain::NULL,
                            _ => Domain::integer(
                                u64::MAX as i128,
                                &mut checker.flow,
                                Span::default(),
                            )
                            .unwrap(),
                        }
                    }
                    _ => unreachable!(),
                }
                snapshots
                    .converted_expression(point, source, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.expressions[&point].value.is_none(), "{fault}");
            },
        );
    }
}

#[test]
pub(crate) fn nullable_conversion_publication_rejects_stale_identity_and_history_atomically() {
    for fault in 0..12 {
        setup(
            "n:=7;v<int32><null>:n",
            |checker, snapshots, point, source| {
                let Effect::Coercion(op) = &mut source.1 else {
                    panic!()
                };
                let input = op.input;
                match fault {
                    0 => source.0 += 1,
                    1 => op.scalar = None,
                    2 => op.scalar.as_mut().unwrap().target.nullable = false,
                    3 => op.result = false,
                    4 => snapshots.rows.get_mut(&Port::Normal(point)).unwrap().owner += 1,
                    5 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Operation(point))
                            .unwrap()
                            .before = Value::Missing
                    }
                    6 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Operation(point))
                            .unwrap()
                            .after = Value::Missing
                    }
                    7 => {
                        snapshots.rows.get_mut(&Port::Normal(point)).unwrap().before =
                            Value::Missing
                    }
                    8 => snapshots.expressions.get_mut(&input).unwrap().owner += 1,
                    9 => {
                        snapshots.rows.remove(&Port::Normal(input));
                    }
                    10 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Operation(point))
                            .unwrap()
                            .control = None
                    }
                    11 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Normal(point))
                            .unwrap()
                            .control = None
                    }
                    _ => unreachable!(),
                }
                let saved = format!("{snapshots:?}");
                assert!(
                    snapshots
                        .converted_expression(point, source, &mut checker.flow, Span::default())
                        .unwrap_err()
                        .message
                        .contains("identity"),
                    "{fault}"
                );
                assert_eq!(format!("{snapshots:?}"), saved);
            },
        );
    }
}

#[test]
pub(crate) fn nullable_conversion_values_accumulate_value_control_without_rewriting_histories() {
    for (input, own) in [(false, false), (true, false), (false, true)] {
        setup(
            "n:=7;v<int32><null>:n",
            |checker, snapshots, point, source| {
                let Effect::Coercion(op) = &mut source.1 else {
                    panic!()
                };
                op.control = own;
                let fact = snapshots.expressions.get_mut(&op.input).unwrap();
                fact.value.as_mut().unwrap().control = input;
                let expected = fact.value.clone().unwrap();
                let saved = format!("{:?}{:?}", snapshots.rows, snapshots.states);
                snapshots
                    .converted_expression(point, source, &mut checker.flow, Span::default())
                    .unwrap();
                let value = snapshots.expressions[&point].value.as_ref().unwrap();
                assert_eq!(value.control, input || own);
                assert_eq!(value.domain, expected.domain);
                assert_eq!(value.origin, expected.origin);
                assert_eq!(format!("{:?}{:?}", snapshots.rows, snapshots.states), saved);
            },
        );
    }
}

#[test]
pub(crate) fn nullable_conversion_values_distinguish_known_bottom_from_unobserved_operations() {
    let (checker, _, _, snapshots) = capture("flag:=false;|flag|v<int32><null>:7");
    let point = checker
        .coercions
        .iter()
        .find(|(_, op)| op.scalar.is_some())
        .unwrap()
        .0;
    let value = snapshots.expressions[point].value.as_ref().unwrap();
    assert_eq!(value.domain, Domain::BOTTOM);
    assert_eq!(
        (value.local, value.version, value.origin),
        (None, None, None)
    );
    let (checker, _, _, snapshots) = capture("d:@\"debug\";d.panic(\"stop\");v<int32><null>:7");
    let point = checker
        .coercions
        .iter()
        .find(|(_, op)| op.scalar.is_some())
        .unwrap()
        .0;
    assert!(!snapshots.expressions.contains_key(point));
}
