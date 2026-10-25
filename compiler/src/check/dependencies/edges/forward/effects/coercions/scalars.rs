use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn scalar_conversion_reports_preserve_exact_types_and_independent_visits() {
    let source = "f:(flag<boolean>,n<uint8>){a<uint8><null>:n;|flag|b<boolean><null>:false;c<int64><null>:null}";
    let (mut checker, mut reports) = checked(source, false);
    let ids = checker
        .coercions
        .iter()
        .filter_map(|(&id, op)| op.scalar.map(|scalar| (id, op.owner, scalar)))
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    for (id, owner, scalar) in ids {
        for port in [Port::Operation(id), Port::Normal(id)] {
            let mut partial = Effects::new();
            let stage = checker
                .coercion_effect_stage(&reports, owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.scalar, Some(scalar));
            checker
                .record_coercion_effect(stage, &mut partial, 1, Span::default())
                .unwrap();
            let (_, Effect::Coercion(observed)) = &partial[&id] else {
                panic!()
            };
            assert_eq!(observed.scalar, Some(scalar));
            assert_eq!(
                (observed.operation, observed.result),
                (port == Port::Operation(id), port == Port::Normal(id))
            );
            assert!(!observed.projected);
            reports
                .effects
                .insert(id, (owner, Effect::Coercion(observed.clone())));
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, id, owner, Span::default())
                    .unwrap(),
                None
            );
        }
    }
}

#[test]
pub(crate) fn scalar_conversion_reports_reject_missing_and_changed_descriptors_atomically() {
    let (mut checker, mut reports) = checked("n:7;v<int32><null>:n", false);
    let (&id, op) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.scalar.is_some())
        .unwrap();
    let owner = op.owner;
    let scalar = op.scalar.unwrap();
    let stage = checker
        .coercion_effect_stage(&reports, owner, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    for changed in [
        None,
        Some(Types {
            source: crate::check::dependencies::bodies::Scalar {
                kind: ScalarKind::Null,
                nullable: false,
            },
            ..scalar
        }),
        Some(Types {
            target: crate::check::dependencies::bodies::Scalar {
                kind: ScalarKind::Bool,
                nullable: true,
            },
            ..scalar
        }),
    ] {
        let mut effects = reports.effects.clone();
        let saved = effects.clone();
        assert!(
            checker
                .record_coercion_effect(
                    Stage {
                        scalar: changed,
                        ..stage
                    },
                    &mut effects,
                    MAX_EDGES,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, saved);
        let (_, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        observed.scalar = changed;
        let saved = reports.effects.clone();
        assert!(
            checker
                .forward_coercion_input(&reports, id, owner, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.effects, saved);
        reports.effects = effects;
    }
}

#[test]
pub(crate) fn scalar_conversion_reports_reject_invalid_captured_types_before_publication() {
    for fault in 0..3 {
        let (mut checker, reports) = checked("n:7;v<int32><null>:n", false);
        let (&id, op) = checker
            .coercions
            .iter_mut()
            .find(|(_, op)| op.scalar.is_some())
            .unwrap();
        let owner = op.owner;
        let types = op.scalar.as_mut().unwrap();
        match fault {
            0 => types.source.nullable = true,
            1 => types.target.nullable = false,
            2 => {
                types.target.kind = ScalarKind::Int {
                    bits: 8,
                    signed: true,
                }
            }
            _ => unreachable!(),
        }
        let saved = reports.effects.clone();
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .coercion_effect_stage(&reports, owner, port, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
        assert_eq!(reports.effects, saved);
    }
}
