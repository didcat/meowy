use super::*;

#[test]
pub(crate) fn record_read_observations_share_exact_work_and_payload_limits() {
    let (mut checker, program, reports) =
        versions::tests::context("r:{->7;->tag:false};copy:((r));r;copy");
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let (defs, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_read_observations(
            &program,
            &reports,
            &graph,
            Observations::new(&defs),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(expected.records.len(), 3);
    let saved = format!("{reports:?}{graph:?}{defs:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_read_observations(
            &program,
            &reports,
            &graph,
            Observations::new(&defs),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((obs, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(obs.records, expected.records);
            assert_eq!(obs.declared, expected.declared);
            assert_eq!(obs.sources, expected.sources);
        }
        assert_eq!(format!("{reports:?}{graph:?}{defs:?}"), saved);
    }
}
