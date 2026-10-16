use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn call_parameter_pairs_reject_stale_positions_owners_and_headers_atomically() {
    for fault in 0..18 {
        let (mut checker, mut reports) = checked(
            "f<int32>:(a<int32>,b<int32>){->a+b};g<int32>:(p<int32>){->p};x:f(1,2)",
            false,
        );
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let param = reports.signatures.functions[&call.function].params[0];
        let other = reports
            .signatures
            .functions
            .values()
            .find(|header| header.params.len() == 1)
            .unwrap()
            .params[0];
        match fault {
            0 => reports.signatures.params.get_mut(&param).unwrap().1 = 1,
            1 => reports.signatures.params.get_mut(&param).unwrap().0 = usize::MAX,
            2 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&call.function)
                    .unwrap()
                    .params[0] = other
            }
            3 => reports
                .signatures
                .functions
                .get_mut(&call.function)
                .unwrap()
                .params
                .swap(0, 1),
            4 => {
                let last = reports
                    .signatures
                    .functions
                    .get_mut(&call.function)
                    .unwrap()
                    .params
                    .pop()
                    .unwrap();
                reports.signatures.params.remove(&last);
            }
            5 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&call.function)
                    .unwrap()
                    .params
                    .push(other);
                reports.signatures.params.insert(other, (call.function, 2));
            }
            6 => checker.points[call.args[0]].owner = call.function + 1,
            7 => checker.points[call.args[0]].parent = None,
            8 => checker.points[call.args[0]].block = Some(usize::MAX),
            9 => checker
                .invocations
                .get_mut(&call.site)
                .unwrap()
                .args
                .swap(0, 1),
            10 => {
                checker.invocations.get_mut(&call.site).unwrap().edges[0].to =
                    Port::Entry(call.args[1])
            }
            11 => {
                let body = reports.signatures.functions[&call.function].body;
                checker.bodies.get_mut(&body).unwrap().owner = 0;
            }
            12 => checker.points[call.point].owner += 1,
            13 => {
                checker.proofs.bindings.remove(&param);
            }
            14 => {
                reports.signatures.functions.remove(&call.function);
            }
            15 => checker.invocations.get_mut(&call.site).unwrap().args[1] = usize::MAX,
            16 => {
                reports.entries.remove(&(call.function + 1));
            }
            17 => {
                reports
                    .signatures
                    .functions
                    .get_mut(&call.function)
                    .unwrap()
                    .shape = Shape::Never
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}{:?}{:?}",
            checker.invocations,
            checker.points,
            checker.proofs.bindings,
            checker.edge_counts()
        );
        let error = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity mismatch"),
            "fault {fault}: {error:?}"
        );
        let effect = &reports.effects[&call.point].1;
        assert!(
            checker
                .validate_call_report(&reports, call.point, call.owner, effect, Span::default())
                .unwrap_err()
                .message
                .contains("identity mismatch")
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}{:?}{:?}",
                checker.invocations,
                checker.points,
                checker.proofs.bindings,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn call_parameter_pairs_bound_exact_work_without_reserving_payload() {
    let (mut checker, reports) = checked("f:(a<int32>,b<int32>){};f(1,2)", false);
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let (_, parts) = checker
        .operation_effects_budgeted(&reports, Span::default())
        .unwrap();
    assert_eq!(parts, MAX_EDGES - 6);
    let before = format!(
        "{reports:?}{:?}{:?}",
        checker.invocations,
        checker.edge_counts()
    );
    let start = checker.flow.work;
    let expected = checker
        .checked_call_inputs(&reports, call.site, Span::default())
        .unwrap()
        .pairs
        .collect::<Vec<_>>();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.checked_call_inputs(&reports, call.site, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap().pairs.collect::<Vec<_>>(), expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("call-effect budget"));
            assert!(checker.flow.exceeded());
        }
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
pub(crate) fn call_parameter_pairs_reject_oversized_arguments_before_exposure() {
    for captured in [false, true] {
        let (mut checker, mut reports) = checked("f<int32>:(a<int32>){->a};x:f(1)", false);
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let limit = crate::check::dependencies::sequences::MAX_ITEMS;
        if captured {
            checker.invocations.get_mut(&call.site).unwrap().args = vec![call.args[0]; limit + 1];
        } else {
            let params = &mut reports
                .signatures
                .functions
                .get_mut(&call.function)
                .unwrap()
                .params;
            *params = vec![params[0]; limit + 1];
        }
        let parts = reports.parts;
        let counts = checker.edge_counts();
        let start = checker.flow.work;
        let error = checker
            .checked_call_inputs(&reports, call.site, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("budget"));
        assert!(checker.flow.work - start < 100);
        assert_eq!(reports.parts, parts);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn call_parameter_pairs_reject_extra_or_missing_report_arguments() {
    let (mut checker, reports) = checked("f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)", false);
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    for count in [0, 1, 3] {
        let mut effect = reports.effects[&call.point].1.clone();
        let Effect::Call {
            args, operation, ..
        } = &mut effect
        else {
            panic!()
        };
        args.resize(count, call.args[0]);
        *operation = false;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .validate_call_report(&reports, call.point, call.owner, &effect, Span::default())
                .unwrap_err()
                .message
                .contains("call-effect identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
