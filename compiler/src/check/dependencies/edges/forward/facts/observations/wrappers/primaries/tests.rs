use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn primary_observations_retain_original_blocks_groups_and_independent_stages() {
    let (mut checker, reports, ports) =
        context("-(({->7;->tag:false}));f:(){!(({->true;->tag:7}))}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, _) = checker
        .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(obs.primaries.len(), 2);
    for (&port, primary) in &obs.primaries {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        let op = &checker.unaries[&point];
        assert!(op.primary);
        assert_eq!(primary.owner, op.owner);
        assert_eq!(primary.input, op.input);
        assert_eq!(primary.control, op.control);
        assert_eq!(reports.slot_uses[&port].1.block, primary.block);
        assert!(!obs.forwarded.contains_key(&Port::Normal(point)));
        assert!(!obs.sources.contains_key(&port));
    }
}

#[test]
pub(crate) fn primary_observations_keep_stored_dispatch_and_unsupported_record_sources_opaque() {
    for source in [
        "r:{->7;->tag:false};-r",
        "-7.{->$;->tag:false}",
        "-{->7;->tag:=false}",
        "n:7;-{->7;->tag:&n}",
        "-{->7;->tag:\"cat\"}",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (obs, _) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        assert!(obs.primaries.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn primary_observations_reject_stale_links_owners_stages_and_edges() {
    for fault in 0..7 {
        let (mut checker, mut reports, ports) = context("-(({->7;->tag:false}))");
        let point = *checker.unaries.iter().find(|(_, op)| op.primary).unwrap().0;
        let port = Port::Projection { point, step: 0 };
        match fault {
            0 => reports.slot_uses.get_mut(&port).unwrap().0 += 1,
            1 => reports.slot_uses.get_mut(&port).unwrap().1.index += 1,
            2 => {
                reports.slot_uses.remove(&port);
            }
            3 => {
                let Effect::Unary(op) = &mut reports.effects.get_mut(&point).unwrap().1 else {
                    panic!()
                };
                op.result = false;
            }
            4 => checker.unaries.get_mut(&point).unwrap().control = true,
            _ => (),
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            5 => graph.nodes.get_mut(&port).unwrap().incoming[0].edge = usize::MAX,
            6 => {
                graph.nodes.remove(&port);
            }
            _ => (),
        }
        let definitions = definitions::Definitions::default();
        let saved = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
                .is_err(),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}

#[test]
pub(crate) fn primary_observations_keep_binary_steps_and_stopped_right_operands_separate() {
    for (source, count) in [
        ("{->7;->tag:false}+{->9;->tag:true}", 2),
        ("d:@\"debug\";{->7;->tag:false}+{d.panic(\"stop\")}", 1),
    ] {
        let (mut checker, reports, ports) = context(source);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (obs, _) = checker
            .scalar_observations(&reports, &graph, &definitions, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(obs.primaries.len(), count);
        for (&port, primary) in &obs.primaries {
            let Port::Projection { point, step } = port else {
                panic!()
            };
            let Effect::Binary(op) = &reports.effects[&point].1 else {
                panic!()
            };
            assert_eq!(primary.input, op.inputs[step]);
            assert_eq!(reports.slot_uses[&port].1.block, primary.block);
            assert_eq!(op.operation, count == 2);
        }
    }
}
