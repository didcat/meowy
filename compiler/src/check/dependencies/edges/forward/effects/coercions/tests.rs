use super::{super::tests::checked, *};

#[test]
pub(crate) fn coercion_effects_preserve_forwarding_projection_and_conversion_plans() {
    let source = "flag:false;n:7;r:{->n;->tag:true};a<int32>:n;b<int32><null>:n;c<int32>:r;e<int32><null>:r;|flag|x<int32><null>:n;f<int32><null>:(n<int32>){->n}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    for primary in [false, true] {
        for kind in [CoercionKind::Forward, CoercionKind::Convert] {
            assert!(
                checker
                    .coercions
                    .values()
                    .any(|op| op.primary == primary && op.kind == kind)
            );
        }
    }
    assert!(checker.coercions.values().any(|op| op.control));
    assert!(checker.coercions.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.coercions {
        let (owner, Effect::Coercion(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.input, op.input);
        assert_eq!(observed.op, op.kind);
        assert_eq!(observed.primary, op.primary);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.projected, op.primary);
        assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
        assert!(observed.result);
    }
}

#[test]
pub(crate) fn coercion_effects_preserve_inner_calls_branches_and_composed_fallbacks() {
    let source = "f<int32>:(){->1};a<int32><null>:f();b<boolean>:false||true;c<boolean>:true&&false;r:{->n:1};same:r==r";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert!(checker.coercions.values().any(|op| op.input == call.point));
    for kind in [PointKind::And, PointKind::Or] {
        assert!(
            checker
                .coercions
                .values()
                .any(|op| checker.points[op.input].kind == kind)
        );
    }
    let binary = checker.binaries.values().next().unwrap();
    for id in binary.inputs {
        assert!(matches!(reports.effects[&id].1, Effect::Coercion(_)));
    }
}

#[test]
pub(crate) fn coercion_effects_keep_stage_observations_and_operation_registration_separate() {
    let (mut checker, mut reports) = checked("r:{->7;->tag:true};x<int32><null>:r", false);
    let (&id, _) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
        .unwrap();
    for (port, projected, operation, result) in [
        (Port::Projection { point: id, step: 0 }, true, false, false),
        (Port::Operation(id), false, true, false),
        (Port::Normal(id), false, false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Coercion(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(
            (op.projected, op.operation, op.result),
            (projected, operation, result)
        );
    }
    reports.index.operations.remove(&id);
    assert!(
        checker
            .coercion_effect_stage(
                &reports,
                0,
                Port::Projection { point: id, step: 0 },
                Span::default()
            )
            .unwrap()
            .is_some()
    );
    for port in [
        Port::Operation(id),
        Port::Normal(id),
        Port::Projection { point: id, step: 1 },
    ] {
        assert!(
            checker
                .coercion_effect_stage(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    let (mut checker, reports) = checked("n:7;x<int32>:n", false);
    let (&id, _) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.kind == CoercionKind::Forward)
        .unwrap();
    assert!(!reports.index.operations.contains_key(&id));
    assert!(
        checker
            .coercion_effect_stage(&reports, 0, Port::Normal(id), Span::default())
            .unwrap()
            .is_some()
    );
    assert!(
        checker
            .coercion_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .is_err()
    );
}
