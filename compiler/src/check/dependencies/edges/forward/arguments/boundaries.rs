use super::{tests::context, *};

#[test]
pub(crate) fn argument_observations_reject_stale_and_foreign_identities_atomically() {
    for fault in 0..12 {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<boolean>){};g:(){f(3,false)};f(1,true)");
        let call = checker
            .invocations
            .values()
            .find(|call| call.owner == 0)
            .unwrap()
            .clone();
        let other = checker
            .invocations
            .values()
            .find(|call| call.owner != 0)
            .unwrap()
            .owner;
        let param = reports.signatures.functions[&call.function].params[0];
        match fault {
            0 => {
                reports.entries.remove(&call.owner);
            }
            1 => reports.entries.get_mut(&call.owner).unwrap().0 = reports.entries[&other].0,
            2 => checker.points[call.args[0]].parent = None,
            3 => checker.points[call.args[0]].owner = other,
            4 => checker
                .invocations
                .get_mut(&call.site)
                .unwrap()
                .args
                .swap(0, 1),
            5 => reports
                .signatures
                .functions
                .get_mut(&call.function)
                .unwrap()
                .params
                .swap(0, 1),
            6 => reports.signatures.params.get_mut(&param).unwrap().1 = 1,
            7 => reports
                .entries
                .get_mut(&other)
                .unwrap()
                .1
                .ports
                .push(Port::Entry(call.args[0])),
            8 => checker.points[call.args[0]].complete = false,
            9 => {
                reports.index.operations.remove(&call.point);
            }
            10 => checker.invocations.get_mut(&call.site).unwrap().args[1] = call.args[0],
            11 => {
                checker.proofs.bindings.remove(&param);
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}{:?}",
            checker.points,
            checker.invocations,
            checker.edge_counts()
        );
        let error = checker
            .call_arguments(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity mismatch"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}{:?}",
                checker.points,
                checker.invocations,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn argument_observations_share_row_payload_and_exact_work_limits() {
    let (mut checker, mut reports) =
        context("f:(a<int32>,b<boolean>){};g:(){f(3,false)};f(1,true)");
    let count = reports.arguments.len();
    assert_eq!(count, 4);
    let indexed = checker
        .call_argument_index(&reports, Span::default(), count)
        .unwrap();
    assert_eq!(indexed.len(), count);
    assert!(indexed.values().all(|arg| !arg.entry && !arg.normal));
    assert!(
        checker
            .call_argument_index(&reports, Span::default(), count - 1)
            .unwrap_err()
            .message
            .contains("index budget")
    );
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .call_arguments(&reports, Span::default(), count, count)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected, (reports.arguments.clone(), 0));
    for (limit, parts) in [(count - 1, count), (count, count - 1)] {
        assert!(
            checker
                .call_arguments(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("call-argument budget")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.call_arguments(&reports, Span::default(), count, count);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("call-argument budget"));
            assert!(checker.flow.exceeded());
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_observations_bound_captured_inputs_and_walks_before_scanning() {
    for captured in [false, true] {
        let (mut checker, mut reports) = context("f:(a<int32>){};f(1)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        if captured {
            checker.invocations.get_mut(&call.site).unwrap().args =
                vec![call.args[0]; crate::check::dependencies::sequences::MAX_ITEMS + 1];
        } else {
            reports.entries.get_mut(&call.owner).unwrap().1.ports =
                vec![Port::Entry(call.args[0]); MAX_REPORT_ITEMS + 1];
        }
        let expected = reports.arguments.clone();
        let parts = reports.parts;
        let edges = checker.edge_counts();
        let start = checker.flow.work;
        assert!(
            checker
                .call_arguments(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert!(checker.flow.work - start < 1000);
        assert_eq!(reports.arguments, expected);
        assert_eq!(reports.parts, parts);
        assert_eq!(checker.edge_counts(), edges);
    }
}

#[test]
pub(crate) fn argument_observations_preserve_declared_returns_and_wrapped_roots() {
    let source = "d:@\"debug\";fail<int32>:(){d.panic(\"stop\")};f:(a<int32>,b<int32>){};r:{->2;->tag:true};f(fail(),((r)))";
    let (checker, reports) = context(source);
    let call = checker
        .invocations
        .values()
        .find(|call| call.args.len() == 2)
        .unwrap();
    let fail = checker
        .invocations
        .values()
        .find(|call| call.args.is_empty())
        .unwrap();
    let body = reports.signatures.functions[&fail.function].body;
    assert!(!checker.bodies[&body].completion.normal);
    for (&point, text) in call.args.iter().zip(["fail()", "((r))"]) {
        let arg = reports.arguments[&point];
        assert!(arg.entry && arg.normal);
        let span = checker.points[point].span;
        assert_eq!(&source[span.start..span.end], text);
    }
}
