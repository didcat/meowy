use super::{super::tests::context, *};

#[test]
pub(crate) fn argument_sources_reject_stale_rows_and_producer_links_atomically() {
    for fault in 0..7 {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<int32>){};r:{->n:1};s:{->n:2};f(r.n,s.n)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let point = call.args[1];
        let field = reports.argument_sources[&point].1.source.unwrap().field;
        match fault {
            0 => reports.arguments.get_mut(&point).unwrap().owner += 1,
            1 => reports.arguments.get_mut(&point).unwrap().position = 0,
            2 => reports
                .entries
                .get_mut(&call.owner)
                .unwrap()
                .1
                .ports
                .retain(|port| *port != Port::Normal(point)),
            3 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(field))
                    .unwrap()
                    .0 += 1
            }
            4 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(field))
                    .unwrap()
                    .1
                    .index = usize::MAX
            }
            5 => reports.effects.get_mut(&field).unwrap().0 += 1,
            6 => {
                checker.coercions.get_mut(&point).unwrap().edges.pop();
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.coercions,
            checker.edge_counts()
        );
        let error = checker
            .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.coercions,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn argument_sources_share_cached_fields_and_bound_rows_payload_and_work() {
    let (mut checker, reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,copy)");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    let first = checker
        .direct_source(&mut ctx, call.args[0], call.owner, Span::default())
        .unwrap();
    let parts = ctx.parts;
    let second = checker
        .direct_source(&mut ctx, call.args[1], call.owner, Span::default())
        .unwrap();
    assert_eq!(first.source.unwrap().field, second.source.unwrap().field);
    assert_eq!(ctx.parts, parts);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .argument_sources(&reports, Span::default(), 2, MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected.0, reports.argument_sources);
    assert_eq!(expected.1, parts - 2);
    let used = MAX_EDGES - expected.1;
    for (limit, parts) in [(1, used), (2, used - 1), (2, used)] {
        let result = checker.argument_sources(&reports, Span::default(), limit, parts);
        if limit == 2 && parts == used {
            assert_eq!(result.unwrap(), (expected.0.clone(), 0));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.argument_sources(&reports, Span::default(), 2, MAX_EDGES);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_sources_keep_reference_aggregate_and_missing_field_sources_unknown() {
    for source in [
        "f:(p<&int32>){};n:1;f(&n)",
        "f:(r<{n<int32>}>){};r:{->n:1};f(r)",
        "f:(a<int32>){};g:(p<int32>){f(p)}",
    ] {
        let (_, reports) = context(source);
        assert_eq!(reports.argument_sources.len(), 1);
        let direct = reports.argument_sources.first_key_value().unwrap().1.1;
        assert!(direct.source.is_none() && direct.block.is_none() && direct.dispatch.is_none());
    }
    let (mut checker, mut reports) = context("f:(a<int32>){};r:{->n:1};f(r.n)");
    reports.field_results.clear();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (sources, parts) = checker
        .argument_sources(&reports, Span::default(), 1, 1)
        .unwrap();
    assert_eq!(parts, 0);
    let direct = sources.first_key_value().unwrap().1.1;
    assert!(direct.source.is_none() && direct.block.is_none() && direct.dispatch.is_none());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_sources_reject_oversized_retained_rows_before_lookup() {
    let (mut checker, mut reports) = context("f:(a<int32>){};f(1)");
    let arg = *reports.arguments.first_key_value().unwrap().1;
    reports.arguments = (0..=MAX_EDGES).map(|point| (point, arg)).collect();
    let sources = reports.argument_sources.clone();
    let parts = reports.parts;
    let work = checker.flow.work;
    let edges = checker.edge_counts();
    assert!(
        checker
            .argument_sources(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err()
            .message
            .contains("call-argument report budget")
    );
    assert_eq!(checker.flow.work, work);
    assert_eq!(reports.argument_sources, sources);
    assert_eq!(reports.parts, parts);
    assert_eq!(checker.edge_counts(), edges);
}
