use super::{super::entries::tests::checked, *};

#[test]
pub(crate) fn signature_context_validation_obeys_exact_work_limits_without_republication() {
    let (mut checker, program) = checked("f:(a<int32>,b<boolean>){};g:(c<int32>){};f(1,true)");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    checker
        .validate_function_signatures(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.validate_function_signatures(&reports, Span::default());
        if short == 0 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    checker.flow = crate::flow::Flow::new();
    let start = checker.flow.work;
    checker
        .validate_signature_calls(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.validate_signature_calls(&reports, Span::default());
        if short == 0 {
            result.unwrap();
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn signature_context_counts_unused_parameters_and_duplicate_calls_once() {
    let (mut checker, program) =
        checked("f:(a<int32>,b<boolean>){};g:(unused<int32>){};f(1,true);f(2,false)");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (expected, parts) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    assert_eq!(parts, MAX_EDGES - 10);
    let before = reports.signatures.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    let (effects, remaining) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(remaining, parts);
    assert_eq!(reports.signatures, before);
}
