use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_emissions_qualify_direct_primaries_without_aliases_or_composition() {
    let (mut checker, reports, ports) =
        context("v:'out{{'out->7}};f<boolean>:(){->true};r:{->n:1};s:{->{->x:1}}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, left) = checker
        .scalar_emission_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(observations.emissions.len(), 2);
    assert_eq!(MAX_EDGES - left, 2);
    for (&id, emission) in &observations.emissions {
        let original = &checker.emissions[&emission.statement];
        assert_eq!(emission.owner, original.owner);
        assert_eq!(emission.input, original.input);
        assert_eq!(emission.target, original.targets[0].block);
        assert_eq!(id, original.targets[0].id);
        assert!(emission.reached && emission.normal);
        assert_eq!(emission.control, Some(false));
    }
    assert!(observations.sources.is_empty());
}

#[test]
pub(crate) fn scalar_emissions_keep_initialization_and_statement_observations_independent() {
    for initialized in [false, true] {
        let (mut checker, mut reports, ports) = context("v:{->7}");
        let (&statement, op) = checker.emissions.first_key_value().unwrap();
        let id = op.targets[0].id;
        let Effect::Emission(observed) = &mut reports.effects.get_mut(&statement).unwrap().1 else {
            panic!()
        };
        observed.initialized[0] = initialized;
        observed.result = !initialized;
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let port = if initialized {
            Port::Normal(statement)
        } else {
            Port::Emission(id)
        };
        let node = graph.nodes.remove(&port).unwrap();
        graph.external.insert(port, node.site);
        if !initialized {
            graph
                .nodes
                .get_mut(&Port::Normal(statement))
                .unwrap()
                .incoming[0]
                .reached = false;
        }
        let definitions = definitions::Definitions::default();
        let (observations, _) = checker
            .scalar_emission_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        assert_eq!(observations.emissions.contains_key(&id), initialized);
        if initialized {
            assert!(!observations.emissions[&id].normal);
        }
    }
}

#[test]
pub(crate) fn scalar_emissions_reject_stale_sources_targets_and_predecessors() {
    for fault in 0..4 {
        let (mut checker, mut reports, ports) = context("v:{->7}");
        let (&statement, op) = checker.emissions.first_key_value().unwrap();
        let id = op.targets[0].id;
        let Effect::Emission(observed) = &mut reports.effects.get_mut(&statement).unwrap().1 else {
            panic!()
        };
        match fault {
            0 => observed.input = statement,
            1 => observed.targets[0].block = usize::MAX,
            2 => observed.initialized[0] = false,
            _ => (),
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if fault == 3 {
            graph.nodes.get_mut(&Port::Emission(id)).unwrap().incoming[0].edge = usize::MAX;
        }
        let definitions = definitions::Definitions::default();
        let before = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_emission_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{graph:?}"), before);
    }
}
