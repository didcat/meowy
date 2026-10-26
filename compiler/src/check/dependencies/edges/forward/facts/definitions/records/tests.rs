use super::*;

#[test]
pub(crate) fn record_definitions_keep_exact_bind_versions_layouts_and_original_blocks() {
    let (mut checker, program, reports) = versions::tests::context(
        "x:=7;r:{->x;->tag:false;x=9};copy:((r));f:(n<uint8>){v:{->n;->tag:true}}",
    );
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
    assert_eq!(defs.records.len(), 3);
    for (&point, record) in &defs.records {
        let storage = &record.storage;
        assert_eq!(storage.version.definition, Definition::Bind(point));
        assert_eq!(storage.version.owner, checker.bodies[&record.block].owner);
        assert!(
            checker.bodies[&record.block]
                .layout
                .matches(&program.locals[storage.version.local])
        );
        assert_eq!(
            defs.declarations[&storage.version.local].version,
            storage.version
        );
        assert!(!defs.storage.contains_key(&point));
    }
    let blocks = defs
        .records
        .values()
        .filter(|record| record.storage.version.owner == 0)
        .map(|record| record.block)
        .collect::<Vec<_>>();
    assert_eq!(blocks[0], blocks[1]);
}

#[test]
pub(crate) fn record_definitions_exclude_mutable_reference_opaque_and_parameter_records() {
    for source in [
        "r:={->7;->tag:false};copy:r",
        "r:{->7;->tag:=false};copy:r",
        "n:7;r:{->7;->tag:&n}",
        "r:{->7;->tag:\"cat\"}",
        "f:(r<{-><int32>;tag<boolean>}>){copy:r}",
    ] {
        let (mut checker, program, reports) = versions::tests::context(source);
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
        assert!(defs.records.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn record_definitions_reject_type_and_source_identity_drift() {
    for fault in 0..3 {
        let (mut checker, program, reports) =
            versions::tests::context("r:{->7;->tag:false};other:{->true;->tag:7}");
        let (mut versions, events, entries, parts) = checker
            .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let (ports, parts) = checker
            .scalar_ports(&reports, &entries, Span::default(), parts)
            .unwrap();
        let (graph, parts) = checker
            .scalar_graph(&reports, ports, Span::default(), parts)
            .unwrap();
        let local = *versions.keys().next().unwrap();
        match fault {
            0 => versions.get_mut(&local).unwrap().ty = versions.values().next_back().unwrap().ty,
            1 => versions.get_mut(&local).unwrap().version.owner += 1,
            2 => versions.get_mut(&local).unwrap().source = None,
            _ => unreachable!(),
        }
        let saved = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
                .is_err()
        );
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}

#[test]
pub(crate) fn record_definitions_share_exact_scalar_work_and_payload_limits() {
    let (mut checker, program, reports) = versions::tests::context("r:{->7;->tag:false};copy:r");
    let (versions, events, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - left;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.scalar_definitions(&reports, &versions, &events, &graph, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((actual, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(actual, expected);
        }
    }
}
