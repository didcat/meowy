use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_slot_headers_keep_declared_domains_and_supported_entry_identities() {
    let (mut checker, reports, ports) =
        context("a:{};b:{->true};f<uint8>:(){->7};r:{->n:1};s:{->\"cat\"};n:{->1.5}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, _) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let (observations, left) = checker
        .scalar_slot_observations(&reports, &graph, observations, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(observations.slots.len(), 5);
    let mut used = 0;
    for (&block, slot) in &observations.slots {
        let scope = observations.scopes[&block];
        assert_eq!(slot.owner, scope.owner);
        assert_eq!(slot.control, scope.control);
        assert!(graph.nodes.contains_key(&Port::BlockEntry(block)));
        assert_eq!(
            slot.declared,
            scope.declared(&mut checker.flow, Span::default()).unwrap()
        );
        used += 1 + slot.declared.weight();
    }
    assert_eq!(MAX_EDGES - left, used);
}

#[test]
pub(crate) fn scalar_slot_headers_do_not_invent_unobserved_entries() {
    let (mut checker, reports, ports) = context("a:{->7}");
    let (mut graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, _) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let block = *observations
        .scopes
        .iter()
        .find(|(_, scope)| scope.parent.is_some())
        .unwrap()
        .0;
    let site = graph.nodes.remove(&Port::BlockEntry(block)).unwrap().site;
    graph.external.insert(Port::BlockEntry(block), site);
    let (observations, _) = checker
        .scalar_slot_observations(&reports, &graph, observations, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(!observations.slots.contains_key(&block));
    assert_eq!(observations.slots.len(), 1);
}

#[test]
pub(crate) fn scalar_slot_headers_reject_stale_owners_shapes_and_entry_sites() {
    for fault in 0..4 {
        let (mut checker, reports, ports) = context("a:{->7}");
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (mut observations, _) = checker
            .scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        let (&block, scope) = observations
            .scopes
            .iter_mut()
            .find(|(_, scope)| scope.parent.is_some())
            .unwrap();
        match fault {
            0 => scope.owner += 1,
            1 => scope.scalar.kind = crate::check::dependencies::ScalarKind::Null,
            2 => {
                graph
                    .nodes
                    .get_mut(&Port::BlockEntry(block))
                    .unwrap()
                    .site
                    .owner += 1
            }
            3 => scope.normal = false,
            _ => unreachable!(),
        }
        assert!(
            checker
                .scalar_slot_observations(
                    &reports,
                    &graph,
                    observations,
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}

#[test]
pub(crate) fn scalar_slot_headers_reserve_exact_work_and_payload() {
    let (mut checker, reports, ports) = context(
        "a:{};b:{->true};f<uint8><null>:(flag<boolean>){|flag|->7};g<boolean><null>:(){->false}",
    );
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, _) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let scopes = observations.scopes.clone();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_slot_observations(&reports, &graph, observations, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    let saved = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let observations = Observations {
            scopes: scopes.clone(),
            ..Observations::new(&definitions)
        };
        let result =
            checker.scalar_slot_observations(&reports, &graph, observations, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((observations, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(observations.slots, expected.slots);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}
