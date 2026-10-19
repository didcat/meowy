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
pub(crate) fn scalar_read_observations_require_actual_results_and_share_declared_domains() {
    let (mut checker, program, reports, ports, definitions) = context("f:(n<int32>){n;n}");
    let points = checker.local_reads.keys().copied().collect::<Vec<_>>();
    let (mut graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    graph.nodes.remove(&Port::Normal(points[0]));
    let (observed, parts) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let (observed, _) = checker
        .scalar_read_observations(&program, &reports, &graph, observed, Span::default(), parts)
        .unwrap();
    assert_eq!(observed.declared.len(), 1);
    assert!(observed.sources.contains_key(&Port::Operation(points[0])));
    assert!(!observed.sources.contains_key(&Port::Normal(points[0])));
    assert!(observed.sources.contains_key(&Port::Normal(points[1])));
    let local = program.functions[0].params[0];
    assert_eq!(
        Some(&observed.declared[&local]),
        Domain::declared(&program.locals[local], &mut checker.flow, Span::default())
            .unwrap()
            .as_ref()
    );
}

#[test]
pub(crate) fn scalar_read_observations_preserve_mutable_slots_and_exclude_aliases_and_references() {
    let (mut checker, program, reports, ports, definitions) = context(
        "n:=1;n;copy:n;copy;row:{->x:2;x};f:(p<&int32>,text<string>){p;text};g<never>:(v<never>){->v}",
    );
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (observed, parts) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    let (observed, _) = checker
        .scalar_read_observations(&program, &reports, &graph, observed, Span::default(), parts)
        .unwrap();
    let mut mutable = 0;
    for (&point, read) in &checker.local_reads {
        let ordinary = read.local == read.storage
            && !checker.proofs.aliases.contains_key(&read.local)
            && Domain::declared(
                &program.locals[read.local],
                &mut checker.flow,
                Span::default(),
            )
            .unwrap()
            .is_some();
        assert_eq!(
            observed.sources.contains_key(&Port::Operation(point)),
            ordinary
        );
        assert_eq!(
            observed.sources.contains_key(&Port::Normal(point)),
            ordinary && read.normal
        );
        if ordinary && checker.proofs.mutable.contains(&read.local) {
            mutable += 1;
        }
    }
    assert_eq!(mutable, 2);
}

#[test]
pub(crate) fn scalar_read_observations_reject_stale_reports_definitions_and_ports() {
    for fault in 0..6 {
        let (mut checker, mut program, mut reports, ports, mut definitions) =
            context("f:(n<int32>){n}");
        let point = *checker.local_reads.first_key_value().unwrap().0;
        let local = program.functions[0].params[0];
        match fault {
            0 => {
                program.locals.pop();
            }
            1 => {
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            2 => {
                definitions
                    .declarations
                    .get_mut(&local)
                    .unwrap()
                    .version
                    .owner += 1
            }
            3 => {
                definitions
                    .declarations
                    .get_mut(&local)
                    .unwrap()
                    .version
                    .local += 1
            }
            _ => {}
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if fault == 4 {
            graph.nodes.remove(&Port::Operation(point));
        }
        if fault == 5 {
            graph
                .nodes
                .get_mut(&Port::Normal(point))
                .unwrap()
                .site
                .owner += 1;
        }
        let (observed, parts) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        let before = format!("{reports:?}{definitions:?}{graph:?}");
        assert!(
            checker
                .scalar_read_observations(
                    &program,
                    &reports,
                    &graph,
                    observed,
                    Span::default(),
                    parts
                )
                .is_err(),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{definitions:?}{graph:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_read_observations_bound_shared_payload_and_exact_work() {
    let (mut checker, program, reports, ports, definitions) = context("f:(n<uint8>){n;n}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let make = || Observations {
        definitions: &definitions,
        sources: HashMap::new(),
        declared: HashMap::new(),
    };
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_read_observations(
            &program,
            &reports,
            &graph,
            make(),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(
        used,
        expected.sources.len()
            + expected
                .declared
                .values()
                .map(|domain| 1 + domain.weight())
                .sum::<usize>()
    );
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_read_observations(
            &program,
            &reports,
            &graph,
            make(),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((observed, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observed.sources, expected.sources);
            assert_eq!(observed.declared, expected.declared);
        }
    }
}
