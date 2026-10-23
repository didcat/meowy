use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_absent_paths_follow_dead_tails_and_nested_leave_continuations() {
    for source in [
        "x:=1;'out{'out.leave();x=2};x",
        "x:=1;'out{|true|{'out.leave()};x=2};x",
    ] {
        let (mut checker, reports, ports) = context(source);
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let (sources, mut parts) = checker
            .scalar_exit_sources(Span::default(), MAX_EDGES)
            .unwrap();
        let mut obs = Observations::new(&definitions);
        for (&point, leave) in &sources.leaves {
            checker
                .scalar_completion_edge(
                    &reports,
                    Family::ScopeExit,
                    Edge::new(Port::Entry(point), Port::Leave(leave.target), Route::Exit),
                    Span::default(),
                )
                .unwrap();
            parts -= 1;
            obs.completion
                .absent
                .insert(Port::Normal(point), leave.owner);
        }
        let (obs, parts) = checker
            .scalar_completion_bodies(&reports, &graph, &sources, obs, Span::default(), parts)
            .unwrap();
        let (obs, _) = checker
            .scalar_absent_paths(&reports, &graph, &sources, obs, Span::default(), parts)
            .unwrap();
        for (&port, &owner) in &obs.completion.absent {
            assert!(!graph.nodes.contains_key(&port));
            assert_eq!(
                checker
                    .scalar_port_site(port, owner, Span::default())
                    .unwrap()
                    .owner,
                owner
            );
        }
        for (&block, body) in &checker.bodies {
            if !body.completion.normal {
                assert_eq!(
                    obs.completion.absent.get(&Port::BlockNormal(block)),
                    Some(&body.owner)
                );
                assert_eq!(
                    obs.completion.absent.get(&Port::BlockResult(block)),
                    Some(&body.owner)
                );
                assert!(
                    obs.completion
                        .absent
                        .contains_key(&Port::Normal(body.parent.unwrap()))
                );
            }
        }
        for (&point, op) in &checker.operations {
            if !graph.nodes.contains_key(&Port::Operation(point)) {
                assert_eq!(
                    obs.completion.absent.get(&Port::Normal(point)),
                    Some(&op.owner)
                );
            }
        }
    }
}

#[test]
pub(crate) fn scalar_absent_paths_require_proven_predecessors_for_never_results() {
    let (mut checker, reports, ports) =
        context("d:@\"debug\";'out{|true|{d.panic(\"stop\")};'out.leave()}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let (sources, parts) = checker
        .scalar_exit_sources(Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let (obs, parts) = checker
        .scalar_completion_bodies(
            &reports,
            &graph,
            &sources,
            Observations::new(&definitions),
            Span::default(),
            parts,
        )
        .unwrap();
    let (obs, _) = checker
        .scalar_absent_paths(&reports, &graph, &sources, obs, Span::default(), parts)
        .unwrap();
    let (&block, body) = checker
        .bodies
        .iter()
        .find(|(id, body)| {
            !body.completion.normal && body.parent.is_some() && !sources.targets.contains(*id)
        })
        .unwrap();
    assert!(obs.completion.bodies.contains_key(&block));
    assert!(obs.completion.absent.contains_key(&Port::Leave(block)));
    for port in [
        Port::BlockNormal(block),
        Port::BlockResult(block),
        Port::Normal(body.parent.unwrap()),
    ] {
        assert!(!obs.completion.absent.contains_key(&port));
    }
}
