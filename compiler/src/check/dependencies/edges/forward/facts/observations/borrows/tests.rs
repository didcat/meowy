use super::*;

pub(super) fn context(
    source: &str,
) -> (
    Checker,
    hir::Program,
    Reports,
    ports::Ports,
    definitions::Definitions,
) {
    let (mut checker, program, reports) = versions::tests::context(source);
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports.clone(), Span::default(), parts)
        .unwrap();
    let (definitions, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    (checker, program, reports, ports, definitions)
}

#[test]
pub(crate) fn scalar_borrow_observations_keep_direct_stages_and_exclude_reference_paths() {
    let (mut checker, program, reports, ports, definitions) =
        context("a:1;&a;b:=2;&!b;row:{->n:=3};row.&!n;p:&a;&p");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (observations, _) = checker
        .scalar_borrow_observations(
            &program,
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    for (&point, borrow) in &checker.place_borrows {
        let direct = borrow.place.fields.is_empty()
            && matches!(program.locals[borrow.place.root], hir::Type::Int { .. });
        for port in [
            Port::Address { point, step: 0 },
            Port::Operation(point),
            Port::Normal(point),
        ] {
            assert_eq!(observations.sources.contains_key(&port), direct);
        }
    }
}

#[test]
pub(crate) fn scalar_borrow_observations_reject_stale_shapes_stages_and_edges() {
    for fault in 0..5 {
        let (mut checker, program, mut reports, ports, definitions) = context("n:=1;&!n");
        let point = *checker.place_borrows.keys().next().unwrap();
        if fault < 2 {
            let (_, Effect::Borrow(observed)) = reports.effects.get_mut(&point).unwrap() else {
                panic!()
            };
            if fault == 0 {
                observed.control = !observed.control;
            } else {
                observed.acquired = false;
            }
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            2 => {
                graph
                    .nodes
                    .get_mut(&Port::Operation(point))
                    .unwrap()
                    .incoming[0]
                    .edge = usize::MAX
            }
            3 => {
                graph
                    .nodes
                    .get_mut(&Port::Normal(point))
                    .unwrap()
                    .site
                    .owner += 1
            }
            4 => checker.place_borrows.get_mut(&point).unwrap().scalar = None,
            _ => (),
        }
        assert!(
            checker
                .scalar_borrow_observations(
                    &program,
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}

#[test]
pub(crate) fn scalar_borrow_observations_bound_payload_and_exact_work() {
    let (mut checker, program, reports, ports, definitions) = context("a:1;&a;b:=2;&!b");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_borrow_observations(
            &program,
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(used, 6);
    let before = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_borrow_observations(
            &program,
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((observations, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observations.sources, expected.sources);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), before);
    }
}
