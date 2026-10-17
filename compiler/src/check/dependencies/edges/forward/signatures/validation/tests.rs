use super::{super::super::entries::tests::checked, *};
use crate::check::dependencies::{ScalarKind, edges::forward::effects::Effect};

#[test]
pub(crate) fn signature_calls_reject_coherent_arity_and_result_shape_mismatches() {
    for fault in 0..4 {
        let (mut checker, program) = checked("f:(a<int32>,b<boolean>){};f(1,true)");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let function = call.function;
        match fault {
            0 => {
                checker.invocations.get_mut(&site).unwrap().shape = Shape::Scalar(ScalarKind::Bool)
            }
            1 => {
                let call = checker.invocations.get_mut(&site).unwrap();
                call.args.pop();
                call.edges = vec![
                    Edge::new(
                        Port::Entry(call.point),
                        Port::Entry(call.args[0]),
                        Route::Next,
                    ),
                    Edge::new(
                        Port::Normal(call.args[0]),
                        Port::Operation(call.point),
                        Route::Next,
                    ),
                    Edge::new(
                        Port::Operation(call.point),
                        Port::Normal(call.point),
                        Route::Returned,
                    ),
                ];
            }
            2 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&function)
                    .unwrap()
                    .shape = Shape::Scalar(ScalarKind::Bool)
            }
            3 => {
                let local = reports
                    .signatures
                    .functions
                    .get_mut(&function)
                    .unwrap()
                    .params
                    .pop()
                    .unwrap();
                reports.signatures.params.remove(&local);
                reports
                    .signatures
                    .functions
                    .get_mut(&function)
                    .unwrap()
                    .inputs
                    .pop();
                checker.parameters.remove(&local);
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.invocations,
            checker.edge_counts()
        );
        let error = checker
            .operation_effects(&reports, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("call-effect identity"),
            "fault {fault}: {error:?}"
        );
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
pub(crate) fn signature_calls_validate_result_only_reports_before_graph_filtering() {
    let (mut checker, program) = checked("f<int32>:(n<int32>){->n};f(1)");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let call = checker.invocations.first_key_value().unwrap().1;
    let (_, Effect::Call { operation, .. }) = reports.effects.get_mut(&call.point).unwrap() else {
        panic!()
    };
    *operation = false;
    reports
        .signatures
        .functions
        .get_mut(&call.function)
        .unwrap()
        .params
        .clear();
    reports.signatures.params.clear();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .call_graph(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn signature_context_rejects_stale_unused_parameter_positions() {
    let (mut checker, program) = checked("unused:(a<int32>,b<boolean>){}");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let local = program.functions[0].params[1];
    reports.signatures.params.get_mut(&local).unwrap().1 = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("signature identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
