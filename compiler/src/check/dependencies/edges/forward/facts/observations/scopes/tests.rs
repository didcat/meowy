use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_scopes_qualify_nested_owners_and_preserve_missing_leave_histories() {
    let (mut checker, reports, ports) =
        context("v:{->7};f<boolean>:(){->{->true}};r:{->n:1};s:{->\"cat\"};n:{->1.5};v");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let before = format!("{reports:?}{graph:?}");
    let (observations, left) = checker
        .scalar_scope_observations(
            &reports,
            &graph,
            Observations::new(&definitions),
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(observations.scopes.len(), 5);
    assert_eq!(observations.forwarded.len(), 2);
    assert_eq!(MAX_EDGES - left, 5 + observations.forwarded.len());
    for (&id, scope) in &observations.scopes {
        assert_eq!(scope.control, Some(false));
        assert!(scope.normal && scope.result);
        assert_eq!(scope.owner, reports.blocks[&id].0);
        if let Some(parent) = scope.parent
            && checker.bodies[&id].scalar.is_some()
        {
            assert_eq!(
                observations.forwarded[&Port::Normal(parent)],
                Forward {
                    owner: scope.owner,
                    input: Port::BlockResult(id),
                    control: false,
                }
            );
        }
        assert!(
            graph.nodes[&Port::BlockNormal(id)]
                .incoming
                .iter()
                .any(|input| {
                    !input.reached && graph.edges[input.edge].1.from == Port::Leave(id)
                })
        );
    }
    assert_eq!(format!("{reports:?}{graph:?}"), before);
}

#[test]
pub(crate) fn scalar_scopes_keep_result_only_observations_without_recovering_normal_state() {
    let (mut checker, mut reports, ports) = context("v:{->7}");
    let id = reports
        .blocks
        .iter()
        .find(|(_, (_, block))| block.parent.is_some())
        .unwrap()
        .0
        .to_owned();
    reports.blocks.get_mut(&id).unwrap().1.normal = false;
    let (mut graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let port = Port::BlockNormal(id);
    let node = graph.nodes.remove(&port).unwrap();
    graph.external.insert(port, node.site);
    graph
        .nodes
        .get_mut(&Port::BlockResult(id))
        .unwrap()
        .incoming[0]
        .reached = false;
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
    assert!(!observations.scopes[&id].normal);
    assert!(observations.scopes[&id].result);
    assert!(observations.sources.is_empty());
    let parent = observations.scopes[&id].parent.unwrap();
    assert_eq!(
        observations.forwarded[&Port::Normal(parent)].input,
        Port::BlockResult(id)
    );
}

#[test]
pub(crate) fn scalar_block_consumers_reject_conflicting_and_foreign_routes() {
    for fault in 0..3 {
        let (mut checker, reports, ports) = context("v:{->7}");
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let (&block, &(owner, observed)) = reports
            .blocks
            .iter()
            .find(|(_, (_, block))| block.parent.is_some())
            .unwrap();
        let port = Port::Normal(observed.parent.unwrap());
        let definitions = definitions::Definitions::default();
        let mut observations = Observations::new(&definitions);
        match fault {
            0 => {
                observations.forwarded.insert(
                    port,
                    Forward {
                        owner,
                        input: Port::BlockResult(block),
                        control: false,
                    },
                );
            }
            1 => graph.nodes.get_mut(&port).unwrap().incoming[0].source.owner += 1,
            2 => graph.nodes.get_mut(&port).unwrap().incoming[0].edge = usize::MAX,
            _ => unreachable!(),
        }
        let saved = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_scope_observations(
                    &reports,
                    &graph,
                    observations,
                    Span::default(),
                    MAX_EDGES
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}

#[test]
pub(crate) fn scalar_scopes_reject_stale_completion_edges_and_duplicate_histories() {
    for fault in 0..5 {
        let (mut checker, mut reports, ports) = context("v:{->7}");
        let id = *reports
            .blocks
            .iter()
            .find(|(_, (_, block))| block.parent.is_some())
            .unwrap()
            .0;
        if fault == 0 {
            reports.blocks.get_mut(&id).unwrap().1.parent = None;
        }
        let (mut graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        match fault {
            1 => graph
                .nodes
                .get_mut(&Port::BlockNormal(id))
                .unwrap()
                .incoming
                .clear(),
            2 => {
                graph
                    .nodes
                    .get_mut(&Port::BlockResult(id))
                    .unwrap()
                    .incoming[0]
                    .edge = usize::MAX
            }
            3 => {
                graph
                    .nodes
                    .get_mut(&Port::BlockResult(id))
                    .unwrap()
                    .incoming[0]
                    .reached = false
            }
            _ => (),
        }
        let definitions = definitions::Definitions::default();
        let mut observations = Observations::new(&definitions);
        if fault == 4 {
            (observations, _) = checker
                .scalar_scope_observations(
                    &reports,
                    &graph,
                    observations,
                    Span::default(),
                    MAX_EDGES,
                )
                .unwrap();
        }
        let before = format!("{reports:?}{graph:?}");
        assert!(
            checker
                .scalar_scope_observations(
                    &reports,
                    &graph,
                    observations,
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
