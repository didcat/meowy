use super::*;

#[test]
pub(crate) fn scalar_storage_definitions_keep_rhs_domains_versions_and_opaque_boundaries() {
    let (mut checker, program, reports) = versions::tests::context(
        "a:7;x:=1;x=2;copy:x;text:\"cat\";r:{->n:=1;n=2};f:(n<uint8>){v<uint8>:n}",
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
    let (definitions, _) = checker
        .scalar_definitions(&reports, &versions, &events, &graph, Span::default(), parts)
        .unwrap();
    assert_eq!(definitions.storage.len(), 5);
    for (&point, storage) in &definitions.storage {
        let (
            owner,
            Effect::Storage {
                local,
                storage: root,
                input,
                control,
                ..
            },
        ) = &reports.effects[&point]
        else {
            panic!()
        };
        assert_eq!(*owner, storage.version.owner);
        assert_eq!(*local, storage.version.local);
        assert_eq!(local, root);
        assert_eq!(*input, Some(storage.input));
        assert_eq!(*control, storage.control);
        assert_eq!(
            Some(&storage.declared),
            Domain::declared(&program.locals[*local], &mut checker.flow, Span::default())
                .unwrap()
                .as_ref()
        );
    }
    assert!(
        definitions
            .storage
            .values()
            .any(|storage| matches!(storage.version.definition, Definition::Write(_)))
    );
}

#[test]
pub(crate) fn scalar_storage_definitions_validate_type_domains_and_shared_limits_atomically() {
    let (mut checker, program, reports) = versions::tests::context("x:=1;x=2;copy:x");
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
    checker.flow.work = 0;
    checker.flow.full = false;
    let slot = versions.values_mut().next().unwrap();
    slot.declared = Some(Domain::boolean(true));
    assert!(
        checker
            .scalar_definitions(
                &reports,
                &versions,
                &events,
                &graph,
                Span::default(),
                MAX_EDGES
            )
            .is_err()
    );
    versions.values_mut().next().unwrap().declared = None;
    let (unknown, _) = checker
        .scalar_definitions(
            &reports,
            &versions,
            &events,
            &graph,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(unknown.storage.len() + 1, expected.storage.len());
}
