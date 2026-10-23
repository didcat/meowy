use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_absence_boundaries_reserve_exact_nested_work_and_payload() {
    let (mut checker, reports, ports) = context("v:'out{->7;{'out.leave();1};2}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_completion_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    let saved = format!(
        "{:?}{:?}{reports:?}{graph:?}",
        checker.bodies, checker.endpoints
    );
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_completion_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        match result {
            Ok((obs, left)) => {
                assert_eq!(obs.completion, expected.completion);
                assert_eq!(left, 0);
                assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
            }
            Err(error) => assert!(error.message.contains("budget")),
        }
        assert_eq!(
            format!(
                "{:?}{:?}{reports:?}{graph:?}",
                checker.bodies, checker.endpoints
            ),
            saved
        );
    }
}

#[test]
pub(crate) fn scalar_absence_boundaries_reject_stale_inventory_sites_and_owners() {
    for fault in 0..3 {
        let (mut checker, reports, ports) = context("'out{{'out.leave();1};2}");
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if fault == 0 {
            checker.region_edges.values_mut().next().unwrap()[0].route = Route::Join;
        } else {
            let (_, site) = graph
                .external
                .iter_mut()
                .find(|(port, _)| matches!(port, Port::Normal(_)))
                .unwrap();
            if fault == 1 {
                site.span.end += 1;
            } else {
                site.owner += 1;
            }
        }
        let definitions = definitions::Definitions::default();
        let saved = format!("{:?}{reports:?}{graph:?}", checker.region_edges);
        assert!(
            checker
                .scalar_completion_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES
                )
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(
            format!("{:?}{reports:?}{graph:?}", checker.region_edges),
            saved
        );
    }
}

#[test]
pub(crate) fn scalar_absence_boundaries_keep_unknown_leave_sources_explicit_after_dead_entry() {
    let (mut checker, reports, ports) = context("'out{'out.leave();'inner{'inner.leave()}}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let point = *checker.scope_exits.keys().next_back().unwrap();
    let block = checker.points[point].block.unwrap();
    let body = checker.bodies.get_mut(&block).unwrap();
    let index = body
        .sources
        .iter()
        .position(|source| *source == Some(point))
        .unwrap();
    body.sources[index] = None;
    let definitions = definitions::Definitions::default();
    let (obs, _) = checker
        .scalar_completion_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert!(obs.completion.absent.contains_key(&Port::Entry(point)));
    assert!(!obs.completion.absent.contains_key(&Port::Leave(block)));
    assert!(
        !obs.completion
            .absent
            .contains_key(&Port::BlockNormal(block))
    );
}

#[test]
pub(crate) fn scalar_absence_boundaries_never_solve_restart_cycles_from_a_dead_parent() {
    let (mut checker, reports, ports) = context("'out{'out.leave();'loop{'loop.restart()}}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let restart = checker.restart_edges.values().next().unwrap();
    let Port::BlockEntry(block) = restart.to else {
        panic!()
    };
    let parent = checker.bodies[&block].parent.unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, _) = checker
        .scalar_completion_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert!(obs.completion.absent.contains_key(&Port::Entry(parent)));
    assert!(!obs.completion.absent.contains_key(&Port::BlockEntry(block)));
    assert!(
        !obs.completion
            .absent
            .contains_key(&Port::BlockResult(block))
    );
}
