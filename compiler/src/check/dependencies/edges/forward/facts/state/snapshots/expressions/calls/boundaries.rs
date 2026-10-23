use super::super::super::capture::tests::capture;
use super::*;

pub(super) fn setup() -> (Checker, Reports, Snapshots, PointId, PointId) {
    let (checker, _, reports, mut snapshots) =
        capture("f<boolean>:(a<boolean>,b<boolean>){->a};f(false,true)");
    let call = checker.invocations.values().next().unwrap();
    let (point, arg) = (call.point, call.args[1]);
    snapshots.expressions.remove(&point).unwrap();
    (checker, reports, snapshots, point, arg)
}

#[test]
pub(crate) fn scalar_call_value_boundaries_reject_late_stale_metadata_without_publication() {
    for fault in 0..13 {
        let (mut checker, mut reports, mut snapshots, point, arg) = setup();
        let source = reports.effects.get_mut(&point).unwrap();
        let Effect::Call {
            result,
            may_return,
            control,
            shape,
            ..
        } = &mut source.1
        else {
            panic!()
        };
        match fault {
            0 => {
                snapshots.rows.remove(&Port::Normal(point));
            }
            1 => snapshots.rows.get_mut(&Port::Normal(point)).unwrap().owner += 1,
            2 => snapshots.rows.get_mut(&Port::Normal(point)).unwrap().after = Value::Missing,
            3 => {
                snapshots
                    .rows
                    .get_mut(&Port::Operation(point))
                    .unwrap()
                    .after = Value::Missing
            }
            4 => {
                snapshots.rows.remove(&Port::Operation(point));
            }
            5 => snapshots.expressions.get_mut(&arg).unwrap().owner += 1,
            6 => {
                snapshots.rows.remove(&Port::Normal(arg));
            }
            7 => snapshots.rows.get_mut(&Port::Normal(arg)).unwrap().after = Value::Missing,
            8 => {
                let row = snapshots.rows.get_mut(&Port::Normal(arg)).unwrap();
                row.before = Value::Known(usize::MAX);
                row.after = row.before;
            }
            9 => *result = false,
            10 => *may_return = false,
            11 => *control = true,
            12 => *shape = Shape::Never,
            _ => unreachable!(),
        }
        let before = format!("{snapshots:?}{source:?}");
        assert!(
            snapshots
                .call_expression(
                    point,
                    source,
                    Some(false),
                    &mut checker.flow,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{snapshots:?}{source:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_call_value_boundaries_do_not_recover_missing_inputs_or_result_only_state() {
    for fault in 0..6 {
        let (mut checker, mut reports, mut snapshots, point, arg) = setup();
        let mut control = Some(false);
        match fault {
            0 => {
                snapshots.expressions.remove(&arg);
            }
            1 => snapshots.expressions.get_mut(&arg).unwrap().value = None,
            2 => {
                snapshots.expressions.remove(&arg);
                snapshots.rows.remove(&Port::Normal(arg));
            }
            3 | 4 => {
                let row = snapshots.rows.get_mut(&Port::Normal(point)).unwrap();
                row.before = Value::Missing;
                row.after = row.before;
                if fault == 3 {
                    snapshots
                        .rows
                        .get_mut(&Port::Operation(point))
                        .unwrap()
                        .after = Value::Missing;
                } else {
                    snapshots.rows.remove(&Port::Operation(point));
                    let Effect::Call { operation, .. } =
                        &mut reports.effects.get_mut(&point).unwrap().1
                    else {
                        panic!()
                    };
                    *operation = false;
                }
            }
            5 => control = None,
            _ => unreachable!(),
        }
        let before = format!(
            "{:?}{:?}{:?}",
            snapshots.rows, snapshots.states, snapshots.guards
        );
        snapshots
            .call_expression(
                point,
                &reports.effects[&point],
                control,
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert!(snapshots.expressions[&point].value.is_none(), "{fault}");
        assert_eq!(
            format!(
                "{:?}{:?}{:?}",
                snapshots.rows, snapshots.states, snapshots.guards
            ),
            before
        );
    }
}

#[test]
pub(crate) fn scalar_call_value_boundaries_keep_argument_facts_before_later_mutation() {
    for dependent in [false, true] {
        let (mut checker, _, reports, mut snapshots) = capture(
            "f<boolean>:(a<boolean>,b<boolean>){->a};g<boolean>:(n<&!boolean>){->true};x:=true;f(x,g(&!x));x",
        );
        let call = checker
            .invocations
            .values()
            .find(|call| call.args.len() == 2)
            .unwrap();
        let point = call.point;
        let arg = call.args[0];
        let value = snapshots
            .expressions
            .get_mut(&arg)
            .unwrap()
            .value
            .as_mut()
            .unwrap();
        assert_eq!(value.domain, Domain::boolean(true));
        value.control = dependent;
        let local = value.local.unwrap();
        let Value::Known(id) = snapshots.rows[&Port::Normal(point)].after else {
            panic!()
        };
        let cell = snapshots.states[id].get(local).unwrap();
        assert!(!cell.control);
        assert!(matches!(
            cell.data,
            Data::Initialized {
                version: None,
                origin: None,
                ..
            }
        ));
        assert_ne!(snapshots.rows[&Port::Normal(arg)].after, Value::Known(id));
        snapshots.expressions.remove(&point).unwrap();
        let before = format!("{:?}{:?}", snapshots.rows, snapshots.states);
        let control = snapshots
            .call_expression(
                point,
                &reports.effects[&point],
                Some(false),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        assert_eq!(control, dependent.then_some(true));
        assert_eq!(
            snapshots.expressions[&point]
                .value
                .as_ref()
                .map(|value| value.control),
            dependent.then_some(true)
        );
        assert_eq!(
            format!("{:?}{:?}", snapshots.rows, snapshots.states),
            before
        );
    }
}

#[test]
pub(crate) fn scalar_call_value_boundaries_preserve_transitive_argument_dependencies() {
    let (mut checker, _, reports, mut snapshots) =
        capture("f<boolean>:(a<boolean>){->a};g<boolean>:(b<boolean>){->b};g(f(false))");
    let outer = checker
        .invocations
        .values()
        .find(|call| call.function == 1)
        .unwrap();
    let (outer, arg) = (outer.point, outer.args[0]);
    let inner = checker
        .invocations
        .values()
        .find(|call| call.function == 0)
        .unwrap();
    let (inner, input) = (inner.point, inner.args[0]);
    assert_eq!(
        checker
            .forward_coercion_input(&reports, arg, 0, Span::default())
            .unwrap(),
        Some(inner)
    );
    snapshots
        .expressions
        .get_mut(&input)
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .control = true;
    for point in [inner, outer] {
        if point == outer {
            snapshots.expressions.remove(&arg).unwrap();
            snapshots
                .forward_expression(
                    arg,
                    &observations::Forward {
                        owner: 0,
                        input: inner,
                        control: false,
                    },
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap();
        }
        snapshots.expressions.remove(&point).unwrap();
        snapshots
            .call_expression(
                point,
                &reports.effects[&point],
                Some(false),
                &mut checker.flow,
                Span::default(),
            )
            .unwrap();
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert!(value.control);
        assert_eq!(value.origin, Some(Origin::Expression(0, point)));
    }
}

#[test]
pub(crate) fn scalar_call_value_boundaries_precharge_exact_work_payload_and_argument_limits() {
    let (mut checker, mut reports, mut snapshots, point, _) = setup();
    let definitions = definitions::Definitions::default();
    let observations = observations::Observations {
        sources: [(Port::Normal(point), &reports.effects[&point])]
            .into_iter()
            .collect(),
        call_controls: [(Port::Normal(point), Some(false))].into_iter().collect(),
        ..observations::Observations::new(&definitions)
    };
    let start = checker.flow.work;
    snapshots
        .expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        )
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
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = snapshots.expression(
            Port::Normal(point),
            &observations,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        assert_eq!(snapshots.room(), if pass { 0 } else { room });
        assert_eq!(snapshots.expressions.get(&point), pass.then_some(&expected));
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
            .expression(
                Port::Normal(point),
                &observations,
                &mut checker.flow,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{snapshots:?}"), before);
    snapshots.expressions.remove(&point);
    let source = reports.effects.get_mut(&point).unwrap();
    let Effect::Call { args, .. } = &mut source.1 else {
        panic!()
    };
    args.resize(MAX_ITEMS + 1, point);
    let before = format!("{snapshots:?}");
    assert!(
        snapshots
            .call_expression(
                point,
                source,
                Some(false),
                &mut checker.flow,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{snapshots:?}"), before);
}
