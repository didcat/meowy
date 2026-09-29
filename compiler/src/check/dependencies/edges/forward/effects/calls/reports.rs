use super::{super::tests::checked, *};

#[test]
pub(crate) fn call_effects_keep_exact_alias_recursion_control_and_return_metadata() {
    let source = "flag:false;f<int32>:(a<int32>,b<int32>){->a+b};alias:f;|flag|x:1.(alias,2);recur<int32>:(n<int32>){->recur(n)};stop<never>:(){'loop{'loop.restart()}};stop();tail:f(3,4)";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    let mut seen = 0;
    for call in checker.invocations.values() {
        let Some((owner, effect)) = reports.effects.get(&call.point) else {
            assert_eq!(&source[call.span.start..call.span.end], "f(3,4)");
            continue;
        };
        seen += 1;
        assert_eq!(*owner, call.owner);
        assert_eq!(
            *effect,
            Effect::Call {
                site: call.site,
                function: call.function,
                args: call.args.clone(),
                may_return: call.may_return,
                control: call.control
            }
        );
        let callee = reports.entries[&(call.function + 1)].0;
        if call.owner == 0 {
            assert!(
                !reports.entries[&0]
                    .1
                    .ports
                    .contains(&Port::BlockEntry(callee))
            );
        }
        assert_eq!(call.control, call.args.len() == 2);
    }
    assert_eq!(seen, 3);
    assert!(reports.effects.values().any(|(_, effect)| matches!(
        effect,
        Effect::Call {
            may_return: false,
            ..
        }
    )));
}

#[test]
pub(crate) fn call_effects_exclude_calls_and_later_inputs_after_stopped_arguments() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};f<int32>:(a<int32>,b<int32>){->a+b};x:f(stop(),{d.print(\"later\");->2})";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let outer = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap();
    let stop = checker
        .invocations
        .values()
        .find(|call| call.args.is_empty())
        .unwrap();
    assert!(!reports.effects.contains_key(&outer.point));
    assert!(
        !reports.entries[&0]
            .1
            .ports
            .contains(&Port::Entry(outer.args[1]))
    );
    assert!(matches!(
        reports.effects[&stop.point].1,
        Effect::Call {
            may_return: false,
            ..
        }
    ));
}

#[test]
pub(crate) fn call_effects_reject_invalid_callee_arguments_and_return_edges() {
    for fault in 0..12 {
        let (mut checker, mut reports) =
            checked("f<int32>:(a<int32>,b<int32>){->a+b};x:f(1,2)", false);
        let (&site, call) = checker.invocations.first_key_value().unwrap();
        let arg = call.args[0];
        let body = reports.entries[&(call.function + 1)].0;
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
            9 => checker.invocations.get_mut(&site).unwrap().args.swap(0, 1),
            10 => checker.invocations.get_mut(&site).unwrap().may_return = false,
            11 => {
                checker
                    .invocations
                    .get_mut(&site)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .route = Route::Next
            }
            _ => unreachable!(),
        }
        let before = reports.effects.clone();
        let error = checker
            .operation_effects(&reports, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001", "fault {fault}");
        assert!(error.message.contains("identity mismatch"), "fault {fault}");
        assert_eq!(reports.effects, before);
    }
}

#[test]
pub(crate) fn call_effects_bound_shared_path_argument_storage_and_charge_copies_once() {
    let (mut checker, mut reports) = checked(
        "f<int32>:(a<int32>,b<int32>){->a+b};r:{->n:=1};r.n=f(1,2)",
        false,
    );
    let call = checker.invocations.first_key_value().unwrap().1.point;
    reports
        .entries
        .get_mut(&0)
        .unwrap()
        .1
        .ports
        .extend([Port::Operation(call); 3]);
    let expected = reports.effects.clone();
    for parts in [0, 2, 3] {
        let result = checker.operation_effects_limited(
            &reports,
            Span::default(),
            MAX_EDGES,
            parts,
            MAX_EDGES,
        );
        assert_eq!(result.is_ok(), parts == 3);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
    }
    let before = checker.flow.work;
    checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        assert_eq!(
            checker.operation_effects(&reports, Span::default()).is_ok(),
            spare == 0
        );
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
    checker.flow = crate::flow::Flow::new();
    let (&site, call) = checker.invocations.first_key_value().unwrap();
    checker.invocations.get_mut(&site).unwrap().args =
        vec![call.args[0]; crate::check::dependencies::sequences::MAX_ITEMS + 1];
    let mut parts = MAX_EDGES;
    assert!(
        checker
            .call_effect(&reports, site, &mut parts, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(parts, MAX_EDGES);
    assert_eq!(reports.effects, expected);
}
