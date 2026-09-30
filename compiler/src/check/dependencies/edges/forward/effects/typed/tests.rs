use super::{super::tests::checked, *};

#[test]
pub(crate) fn typed_effects_keep_predicates_and_erased_ascriptions_distinct() {
    for (source, kind) in [
        ("7~<int32>", TypedKind::Ascription),
        ("7<boolean>", TypedKind::Predicate),
        ("7<never>", TypedKind::Predicate),
        ("f<int32>:(){->7};f()~<int32>", TypedKind::Ascription),
        ("f<int32>:(){->7};f()<int32>", TypedKind::Predicate),
        ("r:{->n:1};r.n~<int32>", TypedKind::Ascription),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.typed_ops.len(), 1);
        let (&id, op) = checker.typed_ops.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                op.owner,
                Effect::Typed(Observed {
                    input: op.input,
                    op: kind,
                    normal: true,
                    control: false,
                    operation: true,
                    result: true,
                })
            )
        );
        assert_ne!(checker.points[id].span, checker.points[op.input].span);
        for call in checker.invocations.values() {
            assert_eq!(call.point, op.input);
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
        }
    }
}

#[test]
pub(crate) fn typed_effects_preserve_nested_branches_owners_and_mutable_guards() {
    let source = "flag:false;v<int32><null>:=1;|flag|x:v<int32>;f<null>:(v<int32><null>){|v<int32>|x:v~<int32>};a:((false&&true)<boolean>)~<boolean>;b:(true||false)<boolean>";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.typed_ops.len(), 6);
    assert!(checker.typed_ops.values().any(|op| op.control));
    assert!(checker.typed_ops.values().any(|op| op.owner != 0));
    assert!(!checker.proofs.observations.is_empty());
    for (&id, op) in &checker.typed_ops {
        let (owner, Effect::Typed(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.op, op.kind);
        assert_eq!(observed.input, op.input);
        assert_eq!(observed.control, op.control);
        assert!(observed.operation && observed.result && observed.normal);
    }
    let (checker, _) = checked("x:(false&&true)<boolean>;y:(false||true)<boolean>", false);
    for kind in [PointKind::And, PointKind::Or] {
        assert!(checker.typed_ops.values().any(|op| {
            let group = &checker.region_edges[&op.input];
            group
                .iter()
                .any(|edge| matches!(edge.to, Port::Entry(id) if checker.points[id].kind == kind))
        }));
    }
}

#[test]
pub(crate) fn typed_effects_keep_operation_and_result_visits_independent() {
    for source in ["7<int32>", "7~<int32>"] {
        let (mut checker, mut reports) = checked(source, false);
        let id = *checker.typed_ops.first_key_value().unwrap().0;
        for port in [Port::Operation(id), Port::Normal(id)] {
            reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
            let effects = checker
                .operation_effects(&reports, Span::default())
                .unwrap();
            assert_eq!(effects.len(), 1);
            let (_, Effect::Typed(op)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(op.operation, port == Port::Operation(id));
            assert_eq!(op.result, port == Port::Normal(id));
        }
        reports.index.operations.remove(&id);
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .typed_effect_stage(&reports, 0, port, Span::default())
                    .is_err()
            );
        }
        assert!(
            checker
                .typed_effect_stage(&reports, 0, Port::Entry(id), Span::default())
                .unwrap()
                .is_none()
        );
    }
}
