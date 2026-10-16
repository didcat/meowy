use super::{super::entries::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn signature_context_rejects_missing_foreign_and_duplicate_header_metadata() {
    for fault in 0..15 {
        let (mut checker, program) = checked("root:1;f:(a<int32>,b<boolean>){};g:(c<int32>){}");
        let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
        let f = &program.functions[0];
        let g = &program.functions[1];
        let local = f.params[0];
        match fault {
            0 => {
                reports.signatures.functions.remove(&f.id);
            }
            1 => {
                reports.signatures.params.remove(&local);
            }
            2 => {
                reports.signatures.params.insert(0, (f.id, 2));
            }
            3 => reports
                .signatures
                .functions
                .get_mut(&f.id)
                .unwrap()
                .params
                .swap(0, 1),
            4 => reports.signatures.functions.get_mut(&f.id).unwrap().params[0] = g.params[0],
            5 => reports.signatures.functions.get_mut(&f.id).unwrap().body = g.body.id,
            6 => {
                reports.signatures.functions.get_mut(&f.id).unwrap().shape =
                    Shape::Scalar(ScalarKind::Int {
                        bits: 7,
                        signed: true,
                    })
            }
            7 => {
                reports.signatures.functions.get_mut(&f.id).unwrap().shape = Shape::Record {
                    fields: crate::check::dependencies::bodies::completion::MAX_BODY_RESULTS + 1,
                }
            }
            8 => reports
                .signatures
                .functions
                .get_mut(&f.id)
                .unwrap()
                .params
                .push(local),
            9 => reports.signatures.functions.get_mut(&f.id).unwrap().params[0] = reports.locals,
            10 => {
                checker.proofs.bindings.remove(&g.params[0]);
            }
            11 => {
                reports.entries.remove(&0);
            }
            12 => reports.signatures.params.get_mut(&local).unwrap().0 = g.id,
            13 => reports.signatures.params.get_mut(&local).unwrap().1 = 1,
            14 => {
                let header = reports.signatures.functions.remove(&f.id).unwrap();
                reports.signatures.functions.insert(usize::MAX, header);
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.proofs.bindings,
            checker.edge_counts()
        );
        let error = checker
            .operation_effects(&reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("function-signature identity"),
            "fault {fault}: {error:?}"
        );
        if fault == 10 {
            assert!(
                checker
                    .function_signatures(&program, &reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("signature identity")
            );
        }
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.proofs.bindings,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn signature_context_rejects_oversized_parameter_vectors_before_copy_or_scan() {
    let (mut checker, mut program) = checked("f:(a<int32>){}");
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let local = program.functions[0].params[0];
    program.functions[0].params = vec![local; MAX_ITEMS + 1];
    let before = reports.signatures.clone();
    assert!(
        checker
            .function_signatures(&program, &reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.signatures, before);
    reports.signatures.functions.get_mut(&0).unwrap().params = program.functions[0].params.clone();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn signature_calls_reject_mismatches_on_unobserved_calls_after_never() {
    let (mut checker, program) = checked(
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(n<int32>){->n};stop();f(1)",
    );
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let (&site, call) = checker
        .invocations
        .iter()
        .find(|(_, call)| call.may_return)
        .unwrap();
    assert!(!reports.effects.contains_key(&call.point));
    checker.invocations.get_mut(&site).unwrap().shape = Shape::Scalar(ScalarKind::Bool);
    assert!(checker.invocations[&site].valid_result());
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("call-effect identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
