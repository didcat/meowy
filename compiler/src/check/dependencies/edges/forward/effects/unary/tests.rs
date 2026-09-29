use super::{super::tests::checked, *};

#[test]
pub(crate) fn unary_effect_stages_preserve_operator_types_and_checked_result_edges() {
    for (source, op, ty, checked) in [
        ("x:!false", UnaryKind::Not, ScalarKind::Bool, false),
        (
            "n<int8>:=7;x:-n",
            UnaryKind::Negate,
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
            true,
        ),
        (
            "n<float32>:1.25;x:-n",
            UnaryKind::Negate,
            ScalarKind::Float { bits: 32 },
            false,
        ),
        (
            "n:1.25;x:-n",
            UnaryKind::Negate,
            ScalarKind::Float { bits: 64 },
            false,
        ),
        (
            "b:@\"bits\";inv:b.not;n<uint64>:7;x:n.(inv)",
            UnaryKind::BitsNot,
            ScalarKind::Int {
                bits: 64,
                signed: false,
            },
            false,
        ),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = self::checked(source, false);
        let (&id, unary) = checker.unaries.first_key_value().unwrap();
        let input = unary.input;
        for (port, kind) in [
            (Port::Operation(id), Kind::Operation),
            (Port::Normal(id), Kind::Result),
        ] {
            let stage = checker
                .unary_effect_stage(&reports, 0, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(
                (stage.input, stage.op, stage.ty, stage.checked, stage.kind),
                (input, op, ty, checked, kind)
            );
        }
    }
}

#[test]
pub(crate) fn unary_effect_stages_keep_primary_projection_and_operation_owners_distinct() {
    let (mut checker, mut reports) = checked("x:-({->2;->tag:true})", false);
    let id = *checker.unaries.first_key_value().unwrap().0;
    let port = Port::Projection { point: id, step: 0 };
    let stage = checker
        .unary_effect_stage(&reports, 0, port, Span::default())
        .unwrap()
        .unwrap();
    assert!(stage.primary && stage.checked);
    assert_eq!(stage.kind, Kind::Projection);
    assert!(
        checker
            .unary_effect_stage(
                &reports,
                0,
                Port::Projection { point: id, step: 1 },
                Span::default()
            )
            .is_err()
    );
    reports.index.operations.remove(&id);
    assert!(
        checker
            .unary_effect_stage(&reports, 0, port, Span::default())
            .unwrap()
            .is_some()
    );
    for port in [Port::Operation(id), Port::Normal(id)] {
        assert!(
            checker
                .unary_effect_stage(&reports, 0, port, Span::default())
                .is_err()
        );
    }
}

#[test]
pub(crate) fn unary_effect_stages_preserve_stops_literals_and_required_only_paths() {
    let (mut checker, reports) = checked("stop<never>:(){'loop{'loop.restart()}};!stop()", false);
    let id = *checker.unaries.first_key_value().unwrap().0;
    for port in [
        Port::Projection { point: id, step: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        assert!(
            checker
                .unary_effect_stage(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    let source = "n<int8>:-128;<T>:{x:-2;-><int32[-x]>};xs<T>:[1]";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.unaries.len(), 1);
    for id in checker.unaries.keys() {
        assert!(!reports.effects.contains_key(id));
    }
    assert!(checked("n<int8>:-128", false).0.unaries.is_empty());
}

#[test]
pub(crate) fn unary_effect_stages_bound_work_and_leave_other_producers_unhandled() {
    let (mut checker, reports) = checked("xs:[1];xs.size();x:!false", false);
    let method = *checker.methods.first_key_value().unwrap().0;
    assert!(
        checker
            .unary_effect_stage(&reports, 0, Port::Normal(method), Span::default())
            .unwrap()
            .is_none()
    );
    let id = *checker.unaries.first_key_value().unwrap().0;
    let before = checker.flow.work;
    let expected = checker
        .unary_effect_stage(&reports, 0, Port::Normal(id), Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.unary_effect_stage(&reports, 0, Port::Normal(id), Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(stage) = result {
            assert_eq!(stage, expected);
        }
        assert_eq!(checker.edge_counts(), counts);
    }
}
