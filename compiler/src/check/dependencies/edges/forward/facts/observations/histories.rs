use super::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_histories_preserve_dependent_and_missing_ancestry() {
    for expected in [Some(false), Some(true), None] {
        let (mut checker, reports, ports) = context("|true|{->7}");
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        if let Some(control) = expected {
            checker.branch_inputs.values_mut().next().unwrap().control = control;
        } else {
            checker.branch_inputs.clear();
        }
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
        let scope = observations
            .scopes
            .values()
            .find(|scope| scope.parent.is_some())
            .unwrap();
        assert_eq!(scope.control, expected);
        let (observations, _) = checker
            .scalar_emission_observations(&reports, &graph, observations, Span::default(), parts)
            .unwrap();
        assert_eq!(
            observations.emissions.values().next().unwrap().control,
            expected
        );
    }
}

#[test]
pub(crate) fn scalar_histories_reserve_exact_work_and_payload_without_changing_sources() {
    for scope in [false, true] {
        let (mut checker, reports, ports) = context("a:{->7};b:{->false}");
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        let start = checker.flow.work;
        let (_, left) = if scope {
            checker.scalar_scope_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
        } else {
            checker.scalar_emission_observations(
                &reports,
                &graph,
                Observations::new(&definitions),
                Span::default(),
                MAX_EDGES,
            )
        }
        .unwrap();
        let cost = checker.flow.work - start;
        let used = MAX_EDGES - left;
        assert!(cost > 0 && used > 0);
        let before = format!("{reports:?}{graph:?}");
        for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
            checker.flow.full = false;
            let result = if scope {
                checker.scalar_scope_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    room,
                )
            } else {
                checker.scalar_emission_observations(
                    &reports,
                    &graph,
                    Observations::new(&definitions),
                    Span::default(),
                    room,
                )
            };
            assert_eq!(result.is_ok(), pass, "{scope} {room} {extra}");
            match result {
                Ok((_, left)) => assert_eq!(left, 0),
                Err(error) => assert!(error.message.contains("budget")),
            }
            assert_eq!(format!("{reports:?}{graph:?}"), before);
        }
    }
}

#[test]
pub(crate) fn scalar_histories_reject_shape_drift_before_classifying_unsupported_results() {
    use crate::check::dependencies::bodies::completion::Shape;
    for shape in [Shape::Never, Shape::Record { fields: 1 }, Shape::Other] {
        let (mut checker, mut reports, ports) = context("v:{->7}");
        let block = reports
            .blocks
            .values_mut()
            .find(|(_, block)| block.parent.is_some())
            .unwrap();
        block.1.completion.result = shape;
        let (graph, _) = checker
            .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
            .unwrap();
        let definitions = definitions::Definitions::default();
        assert!(
            checker
                .scalar_scope_observations(
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
