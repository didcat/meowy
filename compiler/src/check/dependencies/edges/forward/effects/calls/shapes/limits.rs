use super::*;

#[test]
pub(crate) fn call_result_shapes_obey_exact_stage_map_payload_and_work_limits() {
    let (mut checker, reports) =
        checked("f<int32[3]>:(a<int32>,b<int32>){->[a,b]};x:f(1,2)", false);
    let id = checker.invocations.first_key_value().unwrap().1.point;
    let calls = checker.call_effect_index(Span::default()).unwrap();
    let start = checker.flow.work;
    let operation = checker
        .call_effect_stage(&reports, &calls, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result =
            checker.call_effect_stage(&reports, &calls, 0, Port::Operation(id), Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), Some(operation));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    checker.flow = crate::flow::Flow::new();
    let result = checker
        .call_effect_stage(&reports, &calls, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    let mut effects = Effects::new();
    for (limit, mut parts) in [(0, 2), (1, 1)] {
        let before = parts;
        assert!(
            checker
                .record_call_effect(result, &mut effects, limit, &mut parts, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(parts, before);
        assert!(effects.is_empty());
    }
    let mut parts = 2;
    checker
        .record_call_effect(result, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    let partial = effects.clone();
    assert_eq!(parts, 0);
    for stage in [result, operation, result, operation] {
        checker
            .record_call_effect(stage, &mut effects, 1, &mut parts, Span::default())
            .unwrap();
        assert_eq!(parts, 0);
    }
    assert_eq!(effects[&id], reports.effects[&id]);
    for initial in [Effects::new(), partial] {
        let room = if initial.is_empty() { 2 } else { 0 };
        let mut expected = initial.clone();
        let mut parts = room;
        let start = checker.flow.work;
        checker
            .record_call_effect(operation, &mut expected, 1, &mut parts, Span::default())
            .unwrap();
        let work = checker.flow.work - start;
        for short in [0, 1] {
            let mut effects = initial.clone();
            let mut parts = room;
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result =
                checker.record_call_effect(operation, &mut effects, 1, &mut parts, Span::default());
            if short == 0 {
                result.unwrap();
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
            assert_eq!(&effects, if short == 0 { &expected } else { &initial });
            assert_eq!(parts, if short == 0 { 0 } else { room });
        }
        checker.flow = crate::flow::Flow::new();
    }
}
