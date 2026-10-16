use super::*;
use crate::check::dependencies::bodies::completion::MAX_BODY_RESULTS;

#[test]
pub(crate) fn call_result_shapes_reject_invalid_capture_at_every_report_boundary() {
    for shape in [
        Shape::Never,
        Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        }),
        Shape::Scalar(ScalarKind::Float { bits: 16 }),
        Shape::SharedScalar(ScalarKind::Int {
            bits: 128,
            signed: false,
        }),
        Shape::SharedScalar(ScalarKind::Float { bits: 16 }),
        Shape::Record {
            fields: MAX_BODY_RESULTS + 1,
        },
        Shape::List {
            capacity: crate::list::MAX_CAPACITY + 1,
        },
        Shape::Union { members: 1 },
        Shape::Union {
            members: MAX_BODY_RESULTS + 1,
        },
    ] {
        let (mut checker, reports) = checked("f<int32>:(n<int32>){->n};x:f(1);y:f(2)", false);
        let (&site, call) = checker.invocations.last_key_value().unwrap();
        let (id, owner) = (call.point, call.owner);
        let calls = checker.call_effect_index(Span::default()).unwrap();
        let mut stage = checker
            .call_effect_stage(
                &reports,
                &calls,
                owner,
                Port::Operation(id),
                Span::default(),
            )
            .unwrap()
            .unwrap();
        checker.invocations.get_mut(&site).unwrap().shape = shape;
        stage.shape = shape;
        let mut effects = reports.effects.clone();
        let mut parts = 2;
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.invocations,
            checker.edge_counts()
        );
        for error in [
            checker.call_effect_index(Span::default()).unwrap_err(),
            checker
                .call_effect_stage(&reports, &calls, owner, Port::Normal(id), Span::default())
                .unwrap_err(),
            checker
                .record_call_effect(stage, &mut effects, MAX_EDGES, &mut parts, Span::default())
                .unwrap_err(),
            checker.call_graph(&reports, Span::default()).unwrap_err(),
        ] {
            assert_eq!(error.code, "B001");
            assert!(error.message.contains("identity"), "{shape:?}: {error:?}");
        }
        assert_eq!(effects, reports.effects);
        assert_eq!(parts, 2);
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.invocations,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn call_result_shapes_reject_valid_but_stale_partial_report_descriptors() {
    for shape in [
        Shape::Scalar(ScalarKind::Bool),
        Shape::SharedScalar(ScalarKind::Int {
            bits: 16,
            signed: true,
        }),
        Shape::Record { fields: 0 },
        Shape::List { capacity: 0 },
        Shape::Union { members: 2 },
        Shape::Reference(hir::ReferenceMode::Shared),
        Shape::Other,
    ] {
        let (mut checker, mut reports) = checked("f<int32>:(n<int32>){->n};x:f(1);y:f(2)", false);
        let id = checker.invocations.last_key_value().unwrap().1.point;
        let calls = checker.call_effect_index(Span::default()).unwrap();
        let stage = checker
            .call_effect_stage(&reports, &calls, 0, Port::Operation(id), Span::default())
            .unwrap()
            .unwrap();
        let (
            _,
            Effect::Call {
                shape: prior,
                operation,
                ..
            },
        ) = reports.effects.get_mut(&id).unwrap()
        else {
            panic!()
        };
        *prior = shape;
        *operation = false;
        let mut effects = reports.effects.clone();
        let mut parts = 0;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .record_call_effect(stage, &mut effects, MAX_EDGES, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(
            checker
                .call_graph(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, reports.effects);
        assert_eq!(parts, 0);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_result_shape_validation_includes_unobserved_calls_after_never() {
    let (mut checker, reports) = checked(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(){->1};stop();f()",
        false,
    );
    let (&site, call) = checker
        .invocations
        .iter()
        .find(|(_, call)| call.may_return)
        .unwrap();
    assert!(!reports.effects.contains_key(&call.point));
    checker.invocations.get_mut(&site).unwrap().shape = Shape::Never;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let error = checker
        .operation_effects(&reports, Span::default())
        .unwrap_err();
    assert!(error.message.contains("call-effect identity"));
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
