use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_completion_qualifies_absent_and_observed_leaves_without_using_guards() {
    let (mut checker, reports, ports) = context("a:{};'out{->7;'out.leave()};f:(){}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (observations, parts) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    let (&point, exit) = checker.scope_exits.first_key_value().unwrap();
    let Port::Leave(target) = exit.edge.to else {
        panic!()
    };
    let (observations, _) = checker
        .scalar_completion_observations(&reports, &graph, observations, Span::default(), parts)
        .unwrap();
    assert_eq!(observations.completion.leaves[&target], Some(false));
    assert_eq!(observations.completion.absent[&Port::Normal(point)], 0);
    for (&id, scope) in &observations.scopes {
        assert_eq!(
            observations.completion.absent.get(&Port::Leave(id)),
            (id != target).then_some(&scope.owner)
        );
    }
}

#[test]
pub(crate) fn scalar_completion_keeps_unobserved_and_missing_source_leaves_unavailable() {
    for missing in [false, true] {
        let (mut checker, reports, ports) = context("'out{'out.leave();'out.leave()}");
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (observations, parts) = checker
            .scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        let (&point, exit) = checker.scope_exits.first_key_value().unwrap();
        let Port::Leave(target) = exit.edge.to else {
            panic!()
        };
        if missing {
            let block = checker.points[point].block.unwrap();
            let body = checker.bodies.get_mut(&block).unwrap();
            let i = body
                .sources
                .iter()
                .position(|source| *source == Some(point))
                .unwrap();
            body.sources[i] = None;
        }
        let (observations, _) = checker
            .scalar_completion_observations(&reports, &graph, observations, Span::default(), parts)
            .unwrap();
        assert!(
            !observations
                .completion
                .absent
                .contains_key(&Port::Leave(target))
        );
        assert_eq!(
            observations.completion.leaves[&target],
            (!missing).then_some(false)
        );
        assert!(
            graph.nodes[&Port::Leave(target)]
                .incoming
                .iter()
                .any(|input| !input.reached)
        );
    }
}

#[test]
pub(crate) fn scalar_completion_rejects_forged_leave_edges_and_normal_observations() {
    for fault in 0..3 {
        let (mut checker, mut reports, mut ports) = context("'out{'out.leave()}");
        let point = *checker.scope_exits.first_key_value().unwrap().0;
        match fault {
            0 => {
                reports
                    .index
                    .outgoing
                    .get_mut(&Port::Entry(point))
                    .unwrap()
                    .forward
                    .clear();
            }
            1 => {
                let position = reports.index.outgoing[&Port::Entry(point)].forward[0];
                reports.index.edges[position].0 = Family::Region;
            }
            2 => {
                ports.insert(Port::Normal(point), ports[&Port::Entry(point)]);
            }
            _ => unreachable!(),
        }
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        assert!(
            checker
                .scalar_completion_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}
