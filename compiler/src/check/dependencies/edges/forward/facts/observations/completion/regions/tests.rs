use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_stopped_regions_qualify_only_direct_leave_fallthrough() {
    for source in [
        "'out{|true|'out.leave();1}",
        "f:()'out{|false|'out.leave();1}",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let point = *checker.scope_exits.first_key_value().unwrap().0;
        let owner = checker.points[point].owner;
        let parent = checker.points[point].parent.unwrap();
        let definitions = definitions::Definitions::default();
        let (observations, _) = checker
            .scalar_completion_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        assert_eq!(observations.completion.absent[&Port::Normal(parent)], owner);
        let branch = checker.points[parent].parent.unwrap();
        assert!(
            !observations
                .completion
                .absent
                .contains_key(&Port::Normal(branch))
        );
        assert!(graph.nodes.contains_key(&Port::Normal(branch)));
    }
}

#[test]
pub(crate) fn scalar_stopped_regions_preserve_unknown_missing_and_nested_body_boundaries() {
    let (mut checker, reports, ports) = context("'out{|true|'out.leave();1}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let point = *checker.scope_exits.first_key_value().unwrap().0;
    let parent = checker.points[point].parent.unwrap();
    checker.region_edges.remove(&parent);
    assert_eq!(
        checker
            .scalar_stopped_region(&reports, &graph, point, 0, Span::default())
            .unwrap(),
        None
    );
    let (mut checker, reports, ports) = context("'out{|true|{'out.leave()};1}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let point = *checker.scope_exits.first_key_value().unwrap().0;
    assert_eq!(
        checker
            .scalar_stopped_region(&reports, &graph, point, 0, Span::default())
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn scalar_stopped_regions_reject_forged_transparent_edges_and_observations() {
    for fault in 0..2 {
        let (mut checker, mut reports, mut ports) = context("'out{|true|'out.leave();1}");
        let point = *checker.scope_exits.first_key_value().unwrap().0;
        let parent = checker.points[point].parent.unwrap();
        if fault == 0 {
            let position = reports.index.outgoing[&Port::Normal(point)].forward[0];
            reports.index.edges[position].0 = Family::Endpoint;
        } else {
            ports.insert(Port::Normal(parent), ports[&Port::Entry(parent)]);
        }
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        assert!(
            checker
                .scalar_stopped_region(&reports, &graph, point, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}
