use super::super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_completion_boundaries_preserve_dependent_and_unknown_leave_control() {
    for expected in [Some(false), Some(true), None] {
        let (mut checker, reports, ports) = context("f:(flag<boolean>)'out{|flag|'out.leave();1}");
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if let Some(control) = expected {
            checker.branch_inputs.values_mut().next().unwrap().control = control;
        } else {
            checker.branch_inputs.clear();
        }
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
        let exit = checker.scope_exits.values().next().unwrap();
        let Port::Leave(target) = exit.edge.to else {
            panic!()
        };
        assert_eq!(observations.completion.leaves[&target], expected);
        assert!(!observations.completion.absent.contains_key(&exit.edge.to));
    }
}

#[test]
pub(crate) fn scalar_completion_boundaries_reserve_exact_shared_work_and_payload() {
    let (mut checker, reports, ports) = context("a:{};'out{|true|'out.leave();1};f:(){}");
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
        .scalar_completion_observations(&reports, &graph, observations, Span::default(), MAX_EDGES)
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
        let result = checker.scalar_completion_observations(
            &reports,
            &graph,
            observations,
            Span::default(),
            room,
        );
        assert_eq!(result.is_ok(), pass);
        match result {
            Ok((observations, left)) => {
                assert_eq!(left, 0);
                assert_eq!(observations.completion, expected.completion);
            }
            Err(error) => assert!(error.message.contains("budget")),
        }
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}
