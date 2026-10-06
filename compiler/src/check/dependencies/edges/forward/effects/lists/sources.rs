use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};|v<boolean>|xs<T[2]><U[2]>:[r,v]}";

#[test]
pub(crate) fn list_reports_retain_source_shapes_at_each_independent_stage() {
    let (mut checker, reports) = checked(SOURCE, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let plans = checker.list_inputs[&id].clone();
    for port in [
        Port::Projection { point: id, step: 0 },
        Port::Conversion { point: id, part: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        let mut effects = Effects::new();
        let mut room = plans.len() * 4;
        for _ in 0..2 {
            checker
                .record_list_effect(owner, port, &mut effects, 1, &mut room, Span::default())
                .unwrap();
        }
        let (_, Effect::List(observed)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(room, 0);
        assert_eq!(observed.constructed, port == Port::Operation(id));
        assert_eq!(observed.result, port == Port::Normal(id));
        for (index, input) in observed.inputs.iter().enumerate() {
            assert_eq!(input.source, plans[index].source);
            assert_eq!(
                input.projected,
                index == 0 && matches!(port, Port::Projection { .. })
            );
            assert_eq!(
                input.converted,
                index == 0 && matches!(port, Port::Conversion { .. })
            );
        }
        checker
            .validate_list_report(&reports, id, owner, observed, Span::default())
            .unwrap();
    }
}

#[test]
pub(crate) fn list_reports_reject_source_shape_conflicts_and_invalid_unobserved_suffixes() {
    for fault in 0..8 {
        let (mut checker, reports) = checked(SOURCE, false);
        let (&id, op) = checker.lists.first_key_value().unwrap();
        let owner = op.owner;
        let Effect::List(mut observed) = reports.effects[&id].1.clone() else {
            panic!()
        };
        let bad = match fault {
            0 => None,
            1 => Some(Shape::Never),
            2 => Some(Shape::Scalar(ScalarKind::Int {
                bits: 7,
                signed: true,
            })),
            3 => Some(Shape::Union { members: 1 }),
            _ => Some(Shape::Scalar(ScalarKind::Bool)),
        };
        if fault < 4 || fault == 5 {
            checker.list_inputs.get_mut(&id).unwrap()[0].source = bad;
        }
        if fault < 5 {
            observed.inputs[0].source = bad;
        }
        if fault == 6 {
            observed.inputs[1].source = bad;
        }
        if fault == 7 {
            checker.list_inputs.get_mut(&id).unwrap()[1].source = bad;
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.list_inputs,
            checker.edge_counts()
        );
        assert!(
            checker
                .validate_list_report(&reports, id, owner, &observed, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.list_inputs,
                checker.edge_counts()
            ),
            before
        );
    }
    let source = "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[3]><string[3]>:[v,v,{x:1;->x}]}";
    let (mut checker, reports) = checked(source, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let (_, Effect::List(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(observed.inputs[0].projected && !observed.inputs[1].projected);
    assert_eq!(observed.inputs[1].source, Some(Shape::Never));
    checker
        .validate_list_report(&reports, id, owner, observed, Span::default())
        .unwrap();
    checker.list_inputs.get_mut(&id).unwrap()[1].source = None;
    assert!(
        checker
            .validate_list_report(&reports, id, owner, observed, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    let mut fresh = Effects::new();
    let mut room = 12;
    assert!(
        checker
            .record_list_effect(
                owner,
                Port::Projection { point: id, step: 0 },
                &mut fresh,
                1,
                &mut room,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(fresh.is_empty());
    assert_eq!(room, 12);
}

#[test]
pub(crate) fn list_reports_preserve_merges_and_exact_shape_payload_and_work_boundaries() {
    let (mut checker, reports) = checked(SOURCE, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let (owner, count) = (op.owner, op.count);
    let port = Port::Projection { point: id, step: 0 };
    let mut effects = Effects::new();
    let mut room = count * 4;
    checker
        .record_list_effect(owner, port, &mut effects, 1, &mut room, Span::default())
        .unwrap();
    let expected = effects.clone();
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
    ] {
        let (_, Effect::List(observed)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        observed.inputs[0].source = source;
        let before = effects.clone();
        assert!(
            checker
                .record_list_effect(
                    owner,
                    Port::Normal(id),
                    &mut effects,
                    1,
                    &mut room,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        assert_eq!(room, 0);
    }
    let mut fresh = Effects::new();
    room = count * 4 - 1;
    assert!(
        checker
            .record_list_effect(owner, port, &mut fresh, 1, &mut room, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(fresh.is_empty());
    assert_eq!(room, count * 4 - 1);
    room = count * 4;
    let start = checker.flow.work;
    checker
        .record_list_effect(owner, port, &mut fresh, 1, &mut room, Span::default())
        .unwrap();
    assert_eq!(fresh, expected);
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut fresh = Effects::new();
        let mut room = count * 4;
        let result =
            checker.record_list_effect(owner, port, &mut fresh, 1, &mut room, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(fresh, expected);
            assert_eq!(room, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(fresh.is_empty());
            assert_eq!(room, count * 4);
        }
    }
    assert_eq!(reports.effects[&id].0, owner);
}
