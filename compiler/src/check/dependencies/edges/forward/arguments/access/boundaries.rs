use super::{super::tests::context, *};

#[test]
pub(crate) fn call_forest_access_rejects_late_missing_foreign_and_non_normal_markers() {
    for fault in 0..6 {
        let (mut checker, reports) = context(
            "d:@\"debug\";f:(a<int32>,b<int32>,c<int32>,e<int32>){};r:{->n:1};f(r.n,2,d.panic(\"stop\"),3)",
        );
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        let (mut forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let first = forest.index[&call.args[0]];
        match fault {
            0 => {
                forest.index.remove(&call.args[1]);
            }
            1 => {
                forest.index.insert(call.args[1], usize::MAX);
            }
            2 => {
                forest.index.insert(call.args[1], first);
            }
            3 => {
                forest.index.insert(call.args[1], first + 1);
            }
            4 => {
                forest.index.insert(call.args[2], first);
            }
            5 => {
                forest.index.insert(call.args[3], first);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}{:?}", forest.index, checker.edge_counts());
        let Err(error) = checker.call_argument_forest(&forest, call.site, Span::default()) else {
            panic!("fault {fault}")
        };
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!("{reports:?}{:?}{:?}", forest.index, checker.edge_counts()),
            before
        );
    }
}

#[test]
pub(crate) fn call_forest_access_revalidates_current_call_identity_before_exposure() {
    for fault in 0..5 {
        let (mut checker, reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
        let site = *checker.invocations.first_key_value().unwrap().0;
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let call = checker.invocations.get_mut(&site).unwrap();
        match fault {
            0 => call.owner += 1,
            1 => call.args.swap(0, 1),
            2 => call.function = usize::MAX,
            3 => call.site += 1,
            4 => {}
            _ => unreachable!(),
        }
        let site = if fault == 4 { usize::MAX } else { site };
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.invocations,
            checker.edge_counts()
        );
        let Err(error) = checker.call_argument_forest(&forest, site, Span::default()) else {
            panic!("fault {fault}")
        };
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
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
pub(crate) fn call_forest_access_preserves_conservative_rows_and_missing_source_evidence() {
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
        reports.argument_walk = checker
            .argument_walk_report(&reports, Span::default(), MAX_EDGES)
            .unwrap()
            .0;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let (_, inputs) = checker
            .call_argument_forest(&forest, call.site, Span::default())
            .unwrap();
        for (position, (_, point, row, marker)) in inputs.enumerate() {
            assert_eq!(row, reports.arguments.get(&point));
            assert_eq!(marker.is_some(), position == 0 || matches!(mode, 0 | 3));
            if let Some(marker) = marker {
                assert_eq!(
                    forest.walk.visits[marker],
                    Visit::Argument(call.owner, reports.argument_sources[&point].1)
                );
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
