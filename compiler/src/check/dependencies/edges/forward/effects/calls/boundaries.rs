use super::{super::tests::checked, *};

#[test]
pub(crate) fn call_result_stages_reject_invalid_arguments_callees_and_return_edges() {
    for fault in 0..16 {
        let (mut checker, mut reports) =
            checked("f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)", false);
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let (id, arg) = (call.point, call.args[0]);
        let body = reports.entries[&(call.function + 1)].0;
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.clear();
        }
        reports
            .entries
            .get_mut(&0)
            .unwrap()
            .1
            .ports
            .push(Port::Normal(id));
        match fault {
            0 => checker.invocations.get_mut(&site).unwrap().function = usize::MAX,
            1 => {
                reports.entries.remove(&1);
            }
            2 => checker.bodies.get_mut(&body).unwrap().owner = 0,
            3 => checker.bodies.get_mut(&body).unwrap().parent = Some(0),
            4 => checker.invocations.get_mut(&site).unwrap().args[1] = arg,
            5 => checker.points[arg].complete = false,
            6 => checker.points[arg].parent = None,
            7 => checker.points[arg].owner = 1,
            8 => checker.points[arg].kind = PointKind::Stmt,
            9 => checker.points[arg].block = Some(usize::MAX),
            10 => checker.invocations.get_mut(&site).unwrap().args.swap(0, 1),
            11 => checker.invocations.get_mut(&site).unwrap().edges[0].route = Route::Returned,
            12 => {
                checker.invocations.get_mut(&site).unwrap().edges.pop();
            }
            13 => {
                checker
                    .invocations
                    .get_mut(&site)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .route = Route::Next
            }
            14 => {
                checker
                    .invocations
                    .get_mut(&site)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .from = Port::Normal(arg)
            }
            15 => {
                reports.index.operations.remove(&id);
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
        assert_eq!(error.code, "B001");
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
pub(crate) fn call_reports_reject_conflicting_partial_headers_in_aggregation_and_graphs() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked(
            "flag:false;f<int32>:(a<int32>,b<int32>){->a+b};|flag|x:f(1,2)",
            true,
        );
        let id = checker.invocations.first_key_value().unwrap().1.point;
        let calls = checker.call_effect_index(Span::default()).unwrap();
        let stage = checker
            .call_effect_stage(&reports, &calls, 0, Port::Operation(id), Span::default())
            .unwrap()
            .unwrap();
        let (owner, effect) = reports.effects.get_mut(&id).unwrap();
        let Effect::Call {
            site,
            function,
            args,
            may_return,
            control,
            operation,
            result,
        } = effect
        else {
            panic!()
        };
        *operation = false;
        assert!(*result && *control);
        match fault {
            0 => *owner += 1,
            1 => *site = usize::MAX,
            2 => *function = usize::MAX,
            3 => args.swap(0, 1),
            4 => {
                args.pop();
            }
            5 => *may_return = false,
            6 => *control = false,
            7 => *result = false,
            8 => *effect = Effect::Unknown,
            _ => unreachable!(),
        }
        let mut effects = reports.effects.clone();
        let mut parts = 0;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .record_call_effect(stage, &mut effects, MAX_EDGES, &mut parts, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("call-effect identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(effects, reports.effects);
        assert_eq!(parts, 0);
        if fault != 8 {
            let error = checker.call_graph(&reports, Span::default()).unwrap_err();
            assert!(
                error.message.contains("call-effect identity"),
                "fault {fault}: {error:?}"
            );
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_stages_reject_changed_headers_before_retaining_arguments() {
    let (mut checker, reports) = checked("f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)", false);
    let id = checker.invocations.first_key_value().unwrap().1.point;
    let calls = checker.call_effect_index(Span::default()).unwrap();
    let stage = checker
        .call_effect_stage(&reports, &calls, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    for fault in 0..7 {
        let mut stage = stage;
        match fault {
            0 => stage.point = usize::MAX,
            1 => stage.owner += 1,
            2 => stage.site = usize::MAX,
            3 => stage.function += 1,
            4 => stage.count += 1,
            5 => stage.may_return = false,
            6 => stage.control = !stage.control,
            _ => unreachable!(),
        }
        let mut effects = Effects::new();
        let mut parts = 2;
        let error = checker
            .record_call_effect(stage, &mut effects, 1, &mut parts, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("call-effect identity"),
            "fault {fault}: {error:?}"
        );
        assert!(effects.is_empty());
        assert_eq!(parts, 2);
    }
}

#[test]
pub(crate) fn call_report_validation_requires_original_sites_spans_and_operation_owners() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked("f<int32>:(){->1};x:f()", false);
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let id = call.point;
        let Effect::Call { operation, .. } = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        *operation = false;
        match fault {
            0 => checker.calls = site,
            1 => {
                checker.proofs.calls.remove(&site);
            }
            2 => checker.invocations.get_mut(&site).unwrap().span = Span::default(),
            3 => checker.invocations.get_mut(&site).unwrap().site = usize::MAX,
            4 => {
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker.call_graph(&reports, Span::default()).unwrap_err();
        assert!(
            error.message.contains("call-effect identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
