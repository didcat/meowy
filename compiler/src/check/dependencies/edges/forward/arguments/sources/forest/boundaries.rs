use super::*;

#[test]
pub(crate) fn argument_forest_report_shares_exact_qualification_and_traversal_budgets() {
    let (mut checker, reports) =
        context("f:(a<int32>,b<int32>,c<int32>){};r:{->n:1};copy:r.n;f(copy,copy,7)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .argument_walk_report(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    assert_eq!(expected.0, reports.argument_walk);
    let used = MAX_EDGES - expected.1;
    let start = checker.flow.work;
    let (graph, sources, parts) = checker
        .argument_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        graph
            .argument_forest(sources, &mut checker.flow, Span::default(), parts)
            .unwrap(),
        expected
    );
    assert_eq!(checker.flow.work - start, work);
    for parts in [used - 1, used] {
        let result = checker.argument_walk_report(&reports, Span::default(), parts);
        if parts == used {
            let (walk, left) = result.unwrap();
            assert_eq!(walk, expected.0);
            assert_eq!(left, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.argument_walk_report(&reports, Span::default(), used);
        if short == 0 {
            assert_eq!(result.unwrap().0, expected.0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_forest_report_charges_unknown_visits_and_validates_empty_maps() {
    let (mut checker, reports) = context("f:(){};f()");
    let start = checker.flow.work;
    let (walk, parts) = checker
        .argument_walk_report(&reports, Span::default(), 0)
        .unwrap();
    let work = checker.flow.work - start;
    assert!(walk.visits.is_empty());
    assert_eq!(parts, 0);
    let start = checker.flow.work;
    checker
        .argument_source_view(&reports, Span::default(), 0)
        .unwrap();
    assert_eq!(checker.flow.work - start, work);
    let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (_, _, parts) = checker
        .argument_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (walk, left) = checker
        .argument_walk_report(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(walk, reports.argument_walk);
    assert_eq!(left, parts - 2);
    assert!(
        checker
            .argument_walk_report(&reports, Span::default(), 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    reports.argument_sources.clear();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .argument_walk_report(&reports, Span::default(), 0)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_forest_report_keeps_call_argument_and_source_observations_independent() {
    for mode in 0..4 {
        let (mut checker, mut reports) = context("f:(a<int32>,b<int32>){};r:{->n:1};f(r.n,2)");
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        reports.effects.remove(&call.point);
        let point = call.args[1];
        match mode {
            0 => reports.arguments.get_mut(&point).unwrap().entry = false,
            1 => {
                reports.arguments.get_mut(&point).unwrap().normal = false;
                reports.argument_sources.remove(&point);
            }
            2 => {
                reports.arguments.remove(&point);
                reports.argument_sources.remove(&point);
            }
            3 => {
                let direct = &mut reports.argument_sources.get_mut(&call.args[0]).unwrap().1;
                reports
                    .field_results
                    .remove(&Port::Normal(direct.source.take().unwrap().field));
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (walk, _) = checker
            .argument_walk_report(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        let markers = walk
            .visits
            .iter()
            .filter_map(|visit| match visit {
                Visit::Argument(owner, direct) => Some((direct.point, (*owner, *direct))),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(markers, reports.argument_sources);
        assert_eq!(markers.contains_key(&point), matches!(mode, 0 | 3));
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Root(_)))
                .count(),
            usize::from(mode != 3)
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_forest_report_rejects_late_foreign_and_stale_metadata_atomically() {
    for fault in 0..7 {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<int32>){};r:{->n:1};f(2,({->r.n}))");
        let point = checker.invocations.first_key_value().unwrap().1.args[1];
        let source = reports.argument_sources[&point].1.block.unwrap();
        let key = *reports
            .candidate_inputs
            .keys()
            .find(|key| key.0 == source.slot.block)
            .unwrap();
        match fault {
            0 => reports.argument_sources.get_mut(&point).unwrap().0 += 1,
            1 => {
                reports.argument_sources.remove(&point);
            }
            2 => reports.arguments.get_mut(&point).unwrap().owner += 1,
            3 => {
                reports.direct_sources.remove(&key);
            }
            4 => reports.candidate_inputs.get_mut(&key).unwrap().1.point = usize::MAX,
            5 => reports.results.get_mut(&source.slot.block).unwrap().0 += 1,
            6 => {
                reports
                    .argument_sources
                    .insert(usize::MAX, reports.argument_sources[&point]);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .argument_walk_report(&reports, Span::default(), MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
