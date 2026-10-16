use super::{super::entries::tests::checked, *};

#[test]
pub(crate) fn signature_headers_retain_order_unused_parameters_and_declared_results() {
    let source = "d:@\"debug\";f<uint8>:(a<uint8>,unused<boolean>,b<int32>){->a};empty:(){};stop<{tag<boolean>}>:(unused<int32>){d.panic(\"stop\")};x:f(7,true,2)";
    let (mut checker, mut program) = checked(source);
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert!(checker.functions.is_empty() && checker.locals.is_empty());
    assert_eq!(reports.signatures.functions.len(), 3);
    assert_eq!(reports.signatures.params.len(), 4);
    for function in &program.functions {
        let header = &reports.signatures.functions[&function.id];
        assert_eq!(header.body, function.body.id);
        assert_eq!(header.params, function.params);
        assert_eq!(header.shape, Invocation::result_shape(&function.result));
        for (position, local) in function.params.iter().enumerate() {
            assert_eq!(reports.signatures.params[local], (function.id, position));
            assert!(!reports.initializers.contains_key(local));
        }
        if function.name == "stop" {
            assert_eq!(header.shape, Shape::Record { fields: 1 });
            assert_eq!(checker.bodies[&header.body].completion.result, Shape::Never);
        }
    }
    program.functions.reverse();
    assert_eq!(
        checker
            .function_signatures(&program, &reports, Span::default())
            .unwrap(),
        reports.signatures
    );
    let (mut checker, program) = checked("");
    assert_eq!(
        checker
            .entry_reports(&program, Span::default())
            .unwrap()
            .signatures,
        Index::default()
    );
}

#[test]
pub(crate) fn signature_headers_bound_header_parameter_and_work_storage_atomically() {
    let (mut checker, program) = checked("f:(a<int32>,b<boolean>){};g:(){}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .function_signatures_limited(&program, &reports, Span::default(), 2, 4)
        .unwrap();
    let work = checker.flow.work - start;
    for (limit, parts) in [(1, 4), (2, 3)] {
        assert!(
            checker
                .function_signatures_limited(&program, &reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.function_signatures_limited(&program, &reports, Span::default(), 2, 4);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn signature_parameter_payload_is_reserved_once_before_call_arguments() {
    let (mut checker, program) = checked("f:(a<int32>,b<boolean>){};f(1,true)");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (expected, parts) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    assert_eq!(parts, MAX_EDGES - 6);
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    let (effects, remaining) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(remaining, parts);
    assert_eq!(reports.signatures.params.len(), 2);
}
