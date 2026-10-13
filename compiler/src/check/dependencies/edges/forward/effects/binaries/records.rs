use super::{super::tests::checked, *};

pub(super) const SOURCE: &str =
    "n:1;p:&n;a:{->p;->tag:true};b:{->p;->tag:false;->more:2};a==p;p!=b";

#[test]
pub(crate) fn binary_record_reports_preserve_counts_at_independent_operand_and_result_stages() {
    let (mut checker, reports) = checked(SOURCE, false);
    let ops: Vec<_> = checker
        .binaries
        .iter()
        .map(|(&id, op)| (id, op.clone()))
        .collect();
    assert_eq!(ops.len(), 2);
    for (id, op) in ops {
        let step = op.plan.primary.iter().position(|value| *value).unwrap();
        assert_eq!(op.fields[step], Some(step + 1));
        let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.fields, op.fields);
        for port in [
            Port::Projection { point: id, step },
            Port::Operation(id),
            Port::Normal(id),
        ] {
            let stage = checker
                .binary_effect_stage(&reports, op.owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.fields, op.fields);
            let mut effects = Effects::new();
            checker
                .record_binary_effect(stage, &mut effects, 1, Span::default())
                .unwrap();
            let (_, Effect::Binary(value)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(value.fields, op.fields);
            assert_eq!(value.projected[step], stage.kind == Kind::Projection(step));
            assert!(!value.projected[1 - step]);
            assert_eq!(value.operation, stage.kind == Kind::Operation);
            assert_eq!(value.result, stage.kind == Kind::Result);
        }
    }
}

#[test]
pub(crate) fn binary_record_reports_reject_stale_counts_and_wrong_operand_ownership_atomically() {
    for fields in [
        [None; 2],
        [Some(2), None],
        [None, Some(1)],
        [Some(usize::MAX), None],
    ] {
        let (mut checker, mut reports) = checked(SOURCE, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let owner = op.owner;
        let stage = checker
            .binary_effect_stage(&reports, owner, Port::Normal(id), Span::default())
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        checker
            .record_binary_effect(stage, &mut effects, 1, Span::default())
            .unwrap();
        let before = effects.clone();
        assert!(
            checker
                .record_binary_effect(Stage { fields, ..stage }, &mut effects, 1, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.fields = fields;
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
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, reports) = checked(SOURCE, false);
    let id = *checker.binaries.last_key_value().unwrap().0;
    checker.binaries.get_mut(&id).unwrap().fields = [Some(1), None];
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
pub(crate) fn binary_record_reports_bound_exact_stage_work_and_map_capacity() {
    let (mut checker, reports) = checked("n:1;p:&n;r:{->p;->tag:true};r==p", false);
    let id = *checker.binaries.first_key_value().unwrap().0;
    let port = Port::Projection { point: id, step: 0 };
    let start = checker.flow.work;
    let stage = checker
        .binary_effect_stage(&reports, 0, port, Span::default())
        .unwrap()
        .unwrap();
    let mut expected = Effects::new();
    checker
        .record_binary_effect(stage, &mut expected, 1, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let stage = checker
            .binary_effect_stage(&reports, 0, port, Span::default())
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        let result = checker.record_binary_effect(stage, &mut effects, 1, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(effects, expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(effects.is_empty());
        }
    }
    checker.flow = crate::flow::Flow::new();
    let mut effects = Effects::new();
    assert!(
        checker
            .record_binary_effect(stage, &mut effects, 0, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(effects.is_empty());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
