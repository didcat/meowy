use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_definitions_anchor_declared_entries_and_distinct_bind_write_sites() {
    let (mut checker, program, reports) =
        context("a:7;copy:a;x:=1;x=2;f:(n<int32>){v:=n;v=3};f(4)");
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let before = format!("{entries:?}{versions:?}{events:?}");
    let (definitions, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    assert_eq!(definitions.declarations.len(), versions.len() + 2);
    assert_eq!(definitions.writes.len(), 2);
    for (&local, location) in &definitions.declarations {
        assert_eq!(location.version.local, local);
        match location.version.definition {
            Definition::Parameter(_) => {
                assert_eq!(location.port, entries[&location.version.owner].0);
                assert_eq!(location.normal, None);
            }
            Definition::Bind(point) => {
                assert_eq!(location.port, Port::Operation(point));
                assert_eq!(location.normal, Some(Port::Normal(point)));
                assert!(matches!(
                    entries[&location.version.owner].1.get(local).unwrap().data,
                    state::Data::Uninitialized
                ));
            }
            _ => panic!(),
        }
    }
    for (&point, location) in &definitions.writes {
        assert_eq!(location.version.definition, Definition::Write(point));
        assert_eq!(location.port, Port::Operation(point));
        assert_ne!(
            definitions.declarations[&location.version.local].version,
            location.version
        );
    }
    assert_eq!(format!("{entries:?}{versions:?}{events:?}"), before);
}

#[test]
pub(crate) fn scalar_definitions_keep_operation_and_result_observations_independent() {
    let (mut checker, program, reports) = context("x:=1;x=2");
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (mut graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let point = *events.last_key_value().unwrap().0;
    let normal = graph.nodes.remove(&Port::Normal(point)).unwrap();
    let (definitions, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    assert_eq!(definitions.writes[&point].normal, None);
    graph.nodes.insert(Port::Normal(point), normal);
    graph.nodes.remove(&Port::Operation(point));
    assert!(
        checker
            .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
            .is_err()
    );
}

#[test]
pub(crate) fn scalar_definitions_reject_foreign_missing_and_mismatched_version_sources() {
    for fault in 0..4 {
        let (mut checker, program, reports) = context("a:1;x:=2;x=3;f:(n<int32>){n}");
        let (mut versions, mut events, entries, parts) = checker
            .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let (ports, parts) = checker
            .scalar_ports(&reports, &entries, Span::default(), parts)
            .unwrap();
        let (graph, parts) = checker
            .scalar_graph(&reports, ports, Span::default(), parts)
            .unwrap();
        let slot = versions
            .values_mut()
            .find(|slot| matches!(slot.version.definition, Definition::Bind(_)))
            .unwrap();
        match fault {
            0 => slot.version.owner += 1,
            1 => slot.source = None,
            2 => slot.version.local += 1,
            3 => {
                let (&point, event) = events.last_key_value().unwrap();
                let mut version = event.version.unwrap();
                version.definition = Definition::Bind(point);
                events.get_mut(&point).unwrap().version = Some(version);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{versions:?}{events:?}");
        assert_eq!(
            checker
                .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
                .unwrap_err()
                .code,
            "B001"
        );
        assert_eq!(format!("{reports:?}{versions:?}{events:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_definitions_ignore_candidate_values_and_bound_shared_allowances() {
    let (mut checker, program, reports) = context("a:7;copy:a;x:=1;x=2");
    let (mut versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let expected = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.1;
    assert!(versions.values().any(|slot| slot.value.is_some()));
    for slot in versions.values_mut() {
        slot.value = None;
    }
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_definitions(&reports, &versions, &events, &graph, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((definitions, left)) = result {
            assert_eq!(definitions, expected.0);
            assert_eq!(left, 0);
        }
    }
}
