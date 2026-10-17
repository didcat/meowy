use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::signatures::PARAM_PARTS;

#[test]
pub(crate) fn call_stage_limits_charge_arguments_once_and_keep_failed_updates_atomic() {
    for source in [
        "f<int32>:(){->1};x:f()",
        "f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)",
    ] {
        let (mut checker, reports) = checked(source, false);
        let call = checker.invocations.first_key_value().unwrap().1;
        let (id, count) = (call.point, call.args.len());
        let calls = checker.call_effect_index(Span::default()).unwrap();
        let result = checker
            .call_effect_stage(&reports, &calls, 0, Port::Normal(id), Span::default())
            .unwrap()
            .unwrap();
        let operation = checker
            .call_effect_stage(&reports, &calls, 0, Port::Operation(id), Span::default())
            .unwrap()
            .unwrap();
        let mut effects = Effects::new();
        let mut parts = count;
        assert!(
            checker
                .record_call_effect(result, &mut effects, 0, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(effects.is_empty());
        assert_eq!(parts, count);
        if count != 0 {
            let mut short = count - 1;
            assert!(
                checker
                    .record_call_effect(result, &mut effects, 1, &mut short, Span::default())
                    .unwrap_err()
                    .message
                    .contains("budget")
            );
            assert!(effects.is_empty());
            assert_eq!(short, count - 1);
        }
        checker
            .record_call_effect(result, &mut effects, 1, &mut parts, Span::default())
            .unwrap();
        assert_eq!(parts, 0);
        let partial = effects.clone();
        for stage in [result, operation, operation, result] {
            checker
                .record_call_effect(stage, &mut effects, 1, &mut parts, Span::default())
                .unwrap();
            assert_eq!(parts, 0);
        }
        assert_eq!(effects[&id], reports.effects[&id]);
        for initial in [Effects::new(), partial] {
            let room = if initial.is_empty() { count } else { 0 };
            let mut expected = initial.clone();
            let mut parts = room;
            let before = checker.flow.work;
            checker
                .record_call_effect(operation, &mut expected, 1, &mut parts, Span::default())
                .unwrap();
            let work = checker.flow.work - before;
            for short in [0, 1] {
                let mut effects = initial.clone();
                let mut parts = room;
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
                checker.flow.full = false;
                let result = checker.record_call_effect(
                    operation,
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default(),
                );
                assert_eq!(result.is_ok(), short == 0);
                assert_eq!(
                    effects,
                    if short == 0 { &expected } else { &initial }.clone()
                );
                assert_eq!(parts, if short == 0 { 0 } else { room });
            }
            checker.flow = crate::flow::Flow::new();
        }
        let call = checker.invocations.get_mut(&result.site).unwrap();
        call.args = vec![id; crate::check::dependencies::sequences::MAX_ITEMS + 1];
        let before = effects.clone();
        let mut parts = MAX_EDGES;
        assert!(
            checker
                .record_call_effect(result, &mut effects, MAX_EDGES, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(effects, before);
        assert_eq!(parts, MAX_EDGES);
    }
}

#[test]
pub(crate) fn call_collection_bounds_sparse_cross_owner_reports_and_retained_payload() {
    let (mut checker, mut reports) = checked(
        "f<int32>:(a<int32>,b<int32>){->a+b};g<int32>:(n<int32>){->f(n,1)};x:g(3)",
        false,
    );
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.clear();
    }
    for call in checker.invocations.values() {
        reports.entries.get_mut(&call.owner).unwrap().1.ports = vec![
            Port::Normal(call.point),
            Port::Operation(call.point),
            Port::Normal(call.point),
        ];
    }
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let (expected, parts) = checker
        .operation_effects_with_room(&reports, Span::default(), 2, 3, 0)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected.len(), 2);
    assert_eq!(parts, 0);
    for (limit, parts) in [(0, 3), (1, 3), (2, 2)] {
        assert!(
            checker
                .operation_effects_with_room(&reports, Span::default(), limit, parts, 0)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.operation_effects_with_room(&reports, Span::default(), 2, 3, 0);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok((effects, parts)) = result {
            assert_eq!(effects, expected);
            assert_eq!(parts, 0);
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_collection_discards_late_result_identity_failures() {
    let (mut checker, mut reports) = checked("f<int32>:(n<int32>){->n};x:f(1);y:f(2)", false);
    let calls: Vec<_> = checker
        .invocations
        .values()
        .map(|call| (call.site, call.point))
        .collect();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.clear();
    }
    reports.entries.get_mut(&0).unwrap().1.ports =
        vec![Port::Operation(calls[0].1), Port::Normal(calls[1].1)];
    checker
        .invocations
        .get_mut(&calls[1].0)
        .unwrap()
        .edges
        .pop();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let error = checker
        .operation_effects(&reports, Span::default())
        .unwrap_err();
    assert!(error.message.contains("call-effect identity"));
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn call_graph_charges_validation_work_for_result_only_reports() {
    let (mut checker, mut reports) = checked("f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)", false);
    for (_, effect) in reports.effects.values_mut() {
        if let Effect::Call { operation, .. } = effect {
            *operation = false;
        }
    }
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker.call_graph(&reports, Span::default()).unwrap();
    let work = checker.flow.work - start;
    assert!(expected.sites.is_empty());
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.call_graph(&reports, Span::default());
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(graph) = result {
            assert_eq!(graph, expected);
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn initializer_context_shares_parameter_payload_and_exact_consumer_work() {
    let (mut checker, program) = super::super::super::entries::tests::checked(
        "f:(a<int32>,b<boolean>){};f(1,true);n:1;p:&n;r:{->p;->tag:true};copy:r",
    );
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (_, parts) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    let (_, unreserved) = checker
        .operation_effects_with_room(&reports, Span::default(), MAX_EDGES, MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(reports.signatures.params.len(), 2);
    assert_eq!(parts, unreserved - 2 * PARAM_PARTS);
    let count = reports.shared_initializers.len();
    assert_eq!(count, 2);
    let before = format!("{program:?}{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .binding_initializers(&program, &reports, Span::default())
            .unwrap(),
        reports.initializers
    );
    let ordinary = checker.flow.work - start;
    let start = checker.flow.work;
    assert_eq!(
        checker
            .shared_initializers(&program, &reports, Span::default(), count, parts)
            .unwrap(),
        (reports.shared_initializers.clone(), parts - count)
    );
    let shared = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - ordinary + short;
        let result = checker.binding_initializers(&program, &reports, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), reports.initializers);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - shared + short;
        let result = checker.shared_initializers(&program, &reports, Span::default(), count, parts);
        if short == 0 {
            assert_eq!(
                result.unwrap(),
                (reports.shared_initializers.clone(), parts - count)
            );
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(
            format!("{program:?}{reports:?}{:?}", checker.edge_counts()),
            before
        );
    }
}
