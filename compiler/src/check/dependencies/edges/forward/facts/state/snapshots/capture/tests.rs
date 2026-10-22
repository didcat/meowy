use super::super::super::super::versions::tests::context;
use super::*;

pub(in super::super) fn capture(source: &str) -> (Checker, hir::Program, Reports, Snapshots) {
    let (mut checker, program, reports) = context(source);
    checker.proofs.conditions = std::mem::take(&mut checker.guards);
    checker.proofs.tags = std::mem::take(&mut checker.tags);
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let calls = checker
        .qualify_borrow_calls(&program, &reports, &facts, Span::default())
        .unwrap();
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
    let (observations, parts) = checker
        .scalar_borrow_observations(
            &program,
            &reports,
            &graph,
            observations,
            Span::default(),
            parts,
        )
        .unwrap();
    let (observations, parts) = checker
        .scalar_call_observations(
            &reports,
            &graph,
            &calls,
            observations,
            Span::default(),
            parts,
        )
        .unwrap();
    let (observations, parts) = checker
        .scalar_call_result_observations(&reports, &graph, observations, Span::default(), parts)
        .unwrap();
    let (observations, parts) = checker
        .scalar_scope_observations(&reports, &graph, observations, Span::default(), parts)
        .unwrap();
    let (observations, parts) = checker
        .scalar_emission_observations(&reports, &graph, observations, Span::default(), parts)
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
pub(crate) fn scalar_capture_keeps_direct_borrows_storage_only() {
    for source in ["n:7;&n;n", "n:=7;&!n;n"] {
        let (checker, _, _, snapshots) = capture(source);
        let point = *checker.place_borrows.keys().next().unwrap();
        let entry = snapshots.before(Port::Entry(point)).unwrap();
        for port in [
            Port::Address { point, step: 0 },
            Port::Operation(point),
            Port::Normal(point),
        ] {
            assert_eq!(snapshots.after(port), Some(entry));
        }
        assert!(!snapshots.expressions.contains_key(&point));
        let read = *checker.local_reads.keys().next_back().unwrap();
        assert!(snapshots.expressions[&read].value.is_some());
    }
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
pub(crate) fn scalar_capture_preserves_parameter_reads_and_nonreturning_call_boundaries() {
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
    assert!(matches!(
        snapshots.after(Port::Operation(stop.point)),
        Some(View::Known(_))
    ));
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
pub(crate) fn scalar_capture_keeps_bindings_uninitialized_in_entry_states() {
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
        if snapshots.before(Port::Operation(point)) == Some(View::Missing) {
            assert_eq!(snapshots.after(Port::Operation(point)), Some(View::Missing));
        }
    }
}

#[test]
pub(crate) fn scalar_capture_initializes_bindings_and_copies_with_distinct_storage_versions() {
    let (checker, _, _, snapshots) =
        capture("x:=7;x;copy:x;copy;stable:copy;stable;f:(n<uint8>){v:((n)~<uint8>);v}");
    let mut origins = Vec::new();
    let mut versions = Vec::new();
    for (&point, read) in &checker.local_reads {
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        let version = value.version.unwrap();
        assert_eq!(version.owner, read.owner);
        assert_eq!(version.local, read.local);
        let definition = match version.definition {
            Definition::Bind(point) => Port::Operation(point),
            Definition::Parameter(_) => continue,
            Definition::Write(_) => panic!(),
        };
        let Some(View::Known(before)) = snapshots.before(definition) else {
            panic!()
        };
        let Some(View::Known(after)) = snapshots.after(definition) else {
            panic!()
        };
        assert_eq!(before.get(read.local).unwrap().data, Data::Uninitialized);
        assert_eq!(
            after.get(read.local).unwrap().data,
            Data::Initialized {
                version: Some(version),
                domain: Some(value.domain.clone()),
                origin: value.origin,
            }
        );
        if read.owner == 0 {
            origins.push(value.origin.unwrap());
            versions.push(version);
        } else {
            assert_eq!(value.origin, Some(Origin::Parameter(read.owner - 1, 0)));
        }
    }
    assert!(origins.len() >= 3);
    assert!(origins.iter().all(|origin| *origin == origins[0]));
    versions.sort_by_key(|version| version.local);
    versions.dedup();
    assert_eq!(versions.len(), 3);
}

#[test]
pub(crate) fn scalar_capture_rejects_partial_schedules_and_late_definition_or_budget_failures() {
    let (mut checker, program, reports) =
        context("flag:true&&false;|flag|{a:1;b:2};x:=3;x=4;f:(n<uint8>){v<uint8>:((n)~<uint8>)}");
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
    let forwarded = observations.forwarded;
    let declared = observations.declared;
    let guards = observations.guards;
    let logical = observations.logical;
    for fault in 0..4 {
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
            3 => {}
            _ => unreachable!(),
        }
        let mut observations = observations::Observations {
            sources: sources.clone(),
            forwarded: forwarded.clone(),
            declared: declared.clone(),
            guards: guards.clone(),
            logical: logical.clone(),
            ..observations::Observations::new(&definitions)
        };
        if fault == 3 {
            let input = *observations.guards.values().next().unwrap();
            observations.guards.insert(Port::Entry(input.point), input);
        }
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
        sources,
        forwarded,
        declared,
        guards,
        logical,
        ..observations::Observations::new(&definitions)
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
            assert_eq!(snapshots.expressions, expected.expressions);
            assert_eq!(snapshots.guards, expected.guards);
        }
        assert_eq!(format!("{reports:?}{entries:?}{definitions:?}"), before);
    }
}
