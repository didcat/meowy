use super::{super::tests::checked, *};

pub(super) const SOURCE: &str = "n:1;p:&n;r:{->p;->tag:true};a<&int32>:r;b<&int32><null>:r;f<&int32>:(p<&int32>){r:{->p;->tag:true;->more:1};->r}";

#[test]
pub(crate) fn coercion_record_reports_preserve_counts_at_independent_projection_and_result_stages()
{
    let (mut checker, reports) = checked(SOURCE, false);
    let ops: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.fields.is_some())
        .map(|(&id, op)| (id, op.clone()))
        .collect();
    assert_eq!(ops.len(), 3);
    for (id, op) in ops {
        let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.fields, Some(if op.owner == 0 { 1 } else { 2 }));
        for port in [
            Port::Projection { point: id, step: 0 },
            Port::Operation(id),
            Port::Normal(id),
        ] {
            if port == Port::Operation(id) && op.kind != CoercionKind::Convert {
                continue;
            }
            let stage = checker
                .coercion_effect_stage(&reports, op.owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.fields, op.fields);
            let mut effects = Effects::new();
            checker
                .record_coercion_effect(stage, &mut effects, 1, Span::default())
                .unwrap();
            let (_, Effect::Coercion(value)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(value.fields, op.fields);
            assert_eq!(value.projected, stage.kind == Kind::Projection);
            assert_eq!(value.operation, stage.kind == Kind::Operation);
            assert_eq!(value.result, stage.kind == Kind::Result);
        }
    }
}

#[test]
pub(crate) fn coercion_record_reports_reject_stale_counts_and_inconsistent_replay_atomically() {
    for fields in [None, Some(0), Some(2), Some(usize::MAX)] {
        let (mut checker, mut reports) = checked(SOURCE, false);
        let (&id, op) = checker
            .coercions
            .iter()
            .find(|(_, op)| op.fields == Some(1))
            .unwrap();
        let owner = op.owner;
        let stage = checker
            .coercion_effect_stage(&reports, owner, Port::Normal(id), Span::default())
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        checker
            .record_coercion_effect(stage, &mut effects, 1, Span::default())
            .unwrap();
        let before = effects.clone();
        assert!(
            checker
                .record_coercion_effect(Stage { fields, ..stage }, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        let (_, Effect::Coercion(observed)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        observed.fields = fields;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .primary_effect_inputs(
                    &reports,
                    id,
                    owner,
                    &reports.effects[&id].1,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(
            checker
                .forward_coercion_input(&reports, id, owner, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, reports) = checked(SOURCE, false);
    let id = *checker
        .coercions
        .iter()
        .rev()
        .find(|(_, op)| op.fields.is_some())
        .unwrap()
        .0;
    checker.coercions.get_mut(&id).unwrap().fields = Some(usize::MAX);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn coercion_record_reports_bound_exact_work_without_variable_payload() {
    let (mut checker, reports) = checked("n:1;p:&n;r:{->p;->tag:true};copy<&int32><null>:r", false);
    let (&id, op) = checker
        .coercions
        .iter()
        .find(|(_, op)| op.fields.is_some())
        .unwrap();
    let owner = op.owner;
    let start = checker.flow.work;
    let stage = checker
        .coercion_effect_stage(
            &reports,
            owner,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    let mut expected = Effects::new();
    checker
        .record_coercion_effect(stage, &mut expected, 1, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let stage = checker
            .coercion_effect_stage(
                &reports,
                owner,
                Port::Projection { point: id, step: 0 },
                Span::default(),
            )
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        let result = checker.record_coercion_effect(stage, &mut effects, 1, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(effects, expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(effects.is_empty());
        }
    }
    checker.flow = crate::flow::Flow::new();
    let mut limited = Effects::new();
    assert!(
        checker
            .record_coercion_effect(stage, &mut limited, 0, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(limited.is_empty());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
