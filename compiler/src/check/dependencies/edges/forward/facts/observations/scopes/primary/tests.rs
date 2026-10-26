use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn primary_scope_headers_keep_exact_scalar_types_without_whole_record_values() {
    let types = [
        "boolean", "int8", "uint8", "int16", "uint16", "int32", "uint32", "int64", "uint64",
    ];
    for ty in types {
        let source = format!("f:(n<{ty}>){{v:{{->n;->tag:false}}}}");
        let (mut checker, reports, ports) = context(&source);
        let block = *checker
            .bodies
            .iter()
            .find(|(_, body)| matches!(body.completion.result, Shape::Record { .. }))
            .unwrap()
            .0;
        let parent = checker.bodies[&block].parent.unwrap();
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (obs, parts) = checker
            .scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        let (obs, _) = checker
            .scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)
            .unwrap();
        let Layout::Slots(slots) = &checker.bodies[&block].layout else {
            panic!()
        };
        assert_eq!(obs.scopes[&block].scalar.shape(), Some(slots[0].shape));
        assert!(!obs.scopes[&block].scalar.nullable);
        assert!(!obs.forwarded.contains_key(&Port::Normal(parent)));
        assert_eq!(
            obs.slots[&block].declared,
            obs.scopes[&block]
                .declared(&mut checker.flow, Span::default())
                .unwrap()
        );
    }
}

#[test]
pub(crate) fn primary_scope_headers_exclude_mutable_reference_and_opaque_suffixes() {
    for source in [
        "v:{->7;->tag:=false}",
        "n:7;v:{->7;->tag:&n}",
        "v:{->7;->tag:\"cat\"}",
        "v:{->1.5;->tag:false}",
        "f:(flag<boolean>){v:{|flag|->7;->tag:false}}",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (obs, _) = checker
            .scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        for (&block, body) in &checker.bodies {
            if matches!(body.completion.result, Shape::Record { .. }) {
                assert!(!obs.scopes.contains_key(&block), "{source}");
            }
        }
    }
}

#[test]
pub(crate) fn primary_scope_headers_reject_layout_owner_and_stage_drift() {
    for fault in 0..5 {
        let (mut checker, mut reports, ports) = context("v:{->7;->tag:false}");
        let block = *checker
            .bodies
            .iter()
            .find(|(_, body)| matches!(body.completion.result, Shape::Record { .. }))
            .unwrap()
            .0;
        match fault {
            0 => reports.blocks.get_mut(&block).unwrap().0 += 1,
            1 => reports.blocks.get_mut(&block).unwrap().1.result = false,
            2 => checker.bodies.get_mut(&block).unwrap().layout = Layout::Unknown,
            3 | 4 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 3 {
                    slots.pop();
                } else {
                    slots[1].field = None;
                }
            }
            _ => unreachable!(),
        }
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let saved = format!("{reports:?}{graph:?}{:?}", checker.bodies);
        assert!(
            checker
                .scalar_scope_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES
                )
                .is_err()
        );
        assert_eq!(format!("{reports:?}{graph:?}{:?}", checker.bodies), saved);
    }
}

#[test]
pub(crate) fn primary_scope_headers_share_exact_work_and_payload_limits() {
    let (mut checker, reports, ports) = context("v:{->7;->tag:false}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let start = checker.flow.work;
    let (expected, left) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        if let Ok((obs, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(obs.scopes, expected.scopes);
            assert_eq!(obs.forwarded, expected.forwarded);
        }
    }
}
