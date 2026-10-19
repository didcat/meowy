use super::super::super::super::versions::tests::context;
use super::*;

pub(in super::super) fn capture(source: &str) -> (Checker, hir::Program, Reports, Snapshots) {
    let (mut checker, program, reports) = context(source);
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let (orders, parts) = checker
        .scalar_schedule(&graph, &entries, Span::default(), parts)
        .unwrap();
    let (definitions, parts) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let (observations, parts) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), parts)
        .unwrap();
    let (observations, parts) = checker
        .scalar_read_observations(
            &program,
            &reports,
            &graph,
            observations,
            Span::default(),
            parts,
        )
        .unwrap();
    let snapshots = checker
        .scalar_snapshots(
            &graph,
            &orders,
            &entries,
            &observations,
            Span::default(),
            parts,
        )
        .unwrap();
    (checker, program, reports, snapshots)
}

#[test]
pub(crate) fn scalar_capture_preserves_storage_across_literal_stages() {
    let (checker, _, reports, snapshots) = capture("7;false;null;1.5;\"cat\"");
    let entry = snapshots
        .seed(Port::BlockEntry(reports.entries[&0].0))
        .unwrap();
    for point in checker.scalar_leaves.keys() {
        for port in [Port::Operation(*point), Port::Normal(*point)] {
            assert_eq!(snapshots.before(port), Some(View::Known(entry)));
            assert_eq!(snapshots.after(port), Some(View::Known(entry)));
        }
    }
}

#[test]
pub(crate) fn scalar_capture_preserves_parameter_reads_and_stops_at_unqualified_operations() {
    let (mut checker, program, reports, snapshots) = capture(
        "d:@\"debug\";f:(n<int32>){n};stop<never>:(){d.panic(\"stop\")};f(7);stop();late:9",
    );
    let function = &program.functions[0];
    let local = function.params[0];
    let point = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.owner == function.id + 1 && read.local == local)
        .unwrap()
        .0;
    let Some(View::Known(before)) = snapshots.before(Port::Operation(point)) else {
        panic!()
    };
    let Data::Initialized {
        domain, version, ..
    } = &before.get(local).unwrap().data
    else {
        panic!()
    };
    assert_eq!(
        *domain,
        Domain::declared(&program.locals[local], &mut checker.flow, Span::default()).unwrap()
    );
    assert_eq!(version.unwrap().definition, Definition::Parameter(0));
    assert_eq!(
        snapshots.after(Port::Operation(point)),
        Some(View::Known(before))
    );
    let stop = checker
        .invocations
        .values()
        .find(|call| call.owner == 0 && call.function == program.functions[1].id)
        .unwrap();
    assert_eq!(
        snapshots.after(Port::Operation(stop.point)),
        Some(View::Missing)
    );
    assert_eq!(snapshots.before(Port::Normal(stop.point)), None);
    assert_eq!(
        snapshots.before(Port::Operation(
            *checker.operations.last_key_value().unwrap().0
        )),
        None
    );
    assert_eq!(
        snapshots.rows.len(),
        reports
            .entries
            .values()
            .map(|(_, walk)| walk.ports.len())
            .sum()
    );
}

#[test]
pub(crate) fn scalar_capture_preserves_empty_never_and_restart_entry_boundaries() {
    let (_, _, reports, snapshots) = capture("");
    let block = reports.entries[&0].0;
    assert!(matches!(
        snapshots.before(Port::BlockEntry(block)),
        Some(View::Known(_))
    ));
    assert_eq!(
        snapshots.before(Port::BlockNormal(block)),
        Some(View::Missing)
    );
    assert_eq!(
        snapshots.after(Port::BlockResult(block)),
        Some(View::Missing)
    );
    let (_, _, _, snapshots) = capture("impossible<never>:(n<never>){->n}");
    for row in snapshots.rows.values().filter(|row| row.owner == 1) {
        for value in [row.before, row.after] {
            let Some(View::Known(state)) = snapshots.view(value) else {
                panic!()
            };
            assert_eq!(state.reach, FALSE);
        }
    }
    let (_, _, reports, snapshots) = capture("forever<never>:() 'loop {'loop.restart()}");
    let entry = Port::BlockEntry(reports.entries[&1].0);
    assert_eq!(snapshots.seed(entry).unwrap().reach, crate::flow::TRUE);
    assert_eq!(snapshots.before(entry), Some(View::Missing));
    assert_eq!(snapshots.after(entry), Some(View::Missing));
}

#[test]
pub(crate) fn scalar_capture_does_not_publish_candidate_bind_or_write_values() {
    let (checker, _, reports, snapshots) =
        capture("x:=1;x=2;stable:3;copy:stable;f:(n<int32>){v:=n;v=2}");
    for (&point, operation) in &checker.operations {
        if !reports.effects.contains_key(&point) {
            continue;
        }
        let entry = Port::BlockEntry(reports.entries[&operation.owner].0);
        assert!(matches!(
            snapshots
                .seed(entry)
                .unwrap()
                .get(operation.storage)
                .unwrap()
                .data,
            Data::Uninitialized
        ));
        assert_eq!(snapshots.after(Port::Operation(point)), Some(View::Missing));
    }
}

#[test]
pub(crate) fn scalar_capture_rejects_partial_schedules_and_late_definition_or_budget_failures() {
    let (mut checker, program, reports) = context("flag:false;|flag|{a:1;b:2};x:=3;x=4");
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let (orders, parts) = checker
        .scalar_schedule(&graph, &entries, Span::default(), parts)
        .unwrap();
    let (mut definitions, parts) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let (observations, parts) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let expected = checker
        .scalar_snapshots(
            &graph,
            &orders,
            &entries,
            &observations,
            Span::default(),
            parts,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.room();
    let sources = observations.sources;
    for fault in 0..3 {
        let mut order = orders.clone();
        match fault {
            0 => {
                let ports = order.get_mut(&0).unwrap();
                ports.insert(1, ports[0]);
            }
            1 => {
                order.get_mut(&0).unwrap().pop();
            }
            2 => {
                definitions
                    .writes
                    .values_mut()
                    .next()
                    .unwrap()
                    .version
                    .owner += 1
            }
            _ => unreachable!(),
        }
        let observations = observations::Observations {
            definitions: &definitions,
            sources: sources.clone(),
            declared: HashMap::new(),
        };
        assert!(
            checker
                .scalar_snapshots(
                    &graph,
                    &order,
                    &entries,
                    &observations,
                    Span::default(),
                    parts
                )
                .is_err()
        );
        if fault == 2 {
            definitions
                .writes
                .values_mut()
                .next()
                .unwrap()
                .version
                .owner -= 1;
        }
    }
    let observations = observations::Observations {
        definitions: &definitions,
        sources,
        declared: HashMap::new(),
    };
    let before = format!("{reports:?}{entries:?}{definitions:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_snapshots(
            &graph,
            &orders,
            &entries,
            &observations,
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok(snapshots) = result {
            assert_eq!(snapshots.room(), 0);
            assert_eq!(snapshots.rows, expected.rows);
            assert_eq!(snapshots.states, expected.states);
        }
        assert_eq!(format!("{reports:?}{entries:?}{definitions:?}"), before);
    }
}
