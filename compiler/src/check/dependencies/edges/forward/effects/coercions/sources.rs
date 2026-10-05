use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn coercion_reports_retain_exact_source_shapes_at_independent_stages() {
    let source = "n:7;r:3.{->$;->tag:true};a<int32>:n;b<int32>:r;c<int32><null>:r;f<boolean>:(r<{-><never>;tag<boolean>}>){->r}";
    let (mut checker, reports) = checked(source, false);
    for (&id, op) in &checker.coercions {
        if let Some((_, Effect::Coercion(observed))) = reports.effects.get(&id) {
            assert_eq!(observed.source, op.source);
            assert!(observed.op.valid_source(observed.primary, observed.source));
        }
    }
    let ops: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.primary)
        .map(|(&id, op)| (id, op.owner, op.kind, op.source))
        .collect();
    for (id, owner, kind, source) in ops {
        for port in [
            Port::Projection { point: id, step: 0 },
            Port::Operation(id),
            Port::Normal(id),
        ] {
            if (port == Port::Operation(id) && kind != CoercionKind::Convert)
                || (port == Port::Normal(id) && kind == CoercionKind::Stopped)
            {
                continue;
            }
            let stage = checker
                .coercion_effect_stage(&reports, owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.source, source);
            let mut effects = Effects::new();
            for _ in 0..2 {
                checker
                    .record_coercion_effect(stage, &mut effects, 1, Span::default())
                    .unwrap();
            }
            let (_, Effect::Coercion(observed)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(observed.source, source);
            assert_eq!(
                (observed.projected, observed.operation, observed.result),
                (
                    matches!(port, Port::Projection { .. }),
                    port == Port::Operation(id),
                    port == Port::Normal(id)
                )
            );
        }
    }
}

#[test]
pub(crate) fn coercion_reports_reject_stale_or_invalid_source_shapes_atomically() {
    for projected in [false, true] {
        for fault in 0..6 {
            let source = if projected {
                "r:3.{->$;->tag:true};v<int32><null>:r"
            } else {
                "n:3;v<int32>:n"
            };
            let (mut checker, mut reports) = checked(source, false);
            let id = *checker
                .coercions
                .iter()
                .find(|(_, op)| op.primary == projected)
                .unwrap()
                .0;
            let bad = match fault {
                0 => {
                    if projected {
                        None
                    } else {
                        Some(Shape::Scalar(ScalarKind::Bool))
                    }
                }
                1 | 5 => Some(Shape::Never),
                2 => Some(Shape::Scalar(ScalarKind::Int {
                    bits: 7,
                    signed: true,
                })),
                _ => Some(Shape::Scalar(ScalarKind::Bool)),
            };
            if fault == 3 || fault == 5 {
                checker.coercions.get_mut(&id).unwrap().source = bad;
            }
            if fault != 3 {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = bad;
            }
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert!(
                checker
                    .primary_effect_inputs(
                        &reports,
                        id,
                        0,
                        &reports.effects[&id].1,
                        Span::default()
                    )
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "{projected}, {fault}"
            );
            assert!(
                checker
                    .forward_coercion_input(&reports, id, 0, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn coercion_reports_reject_changed_merge_shapes_and_bound_exact_work() {
    let (mut checker, reports) = checked("r:3.{->$;->tag:true};v<int32><null>:r", false);
    let id = *checker
        .coercions
        .iter()
        .find(|(_, op)| op.primary)
        .unwrap()
        .0;
    let stage = checker
        .coercion_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    checker
        .record_coercion_effect(stage, &mut effects, 1, Span::default())
        .unwrap();
    let expected = effects.clone();
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
    ] {
        let mut changed = stage;
        changed.source = source;
        assert!(
            checker
                .record_coercion_effect(changed, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, expected);
    }
    let start = checker.flow.work;
    let mut fresh = Effects::new();
    checker
        .record_coercion_effect(stage, &mut fresh, 1, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(fresh, expected);
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut fresh = Effects::new();
        let result = checker.record_coercion_effect(stage, &mut fresh, 1, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(fresh, expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(fresh.is_empty());
        }
    }
}
