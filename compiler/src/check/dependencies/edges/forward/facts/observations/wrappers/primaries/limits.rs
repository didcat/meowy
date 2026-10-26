use super::super::super::tests::context;
use super::*;

#[test]
pub(crate) fn primary_observations_share_exact_field_scope_and_projection_limits() {
    let (mut checker, reports, ports) = context("(({->7;->tag:false}))+{->9;->tag:true}");
    let (graph, _) = checker
        .scalar_graph(&reports, ports, Span::default(), MAX_EDGES)
        .unwrap();
    let definitions = definitions::Definitions::default();
    let observe = |checker: &mut Checker, parts| -> Result<_> {
        let (obs, parts) =
            checker.scalar_observations(&reports, &graph, &definitions, Span::default(), parts)?;
        let (obs, parts) =
            checker.scalar_scope_observations(&reports, &graph, obs, Span::default(), parts)?;
        let (obs, parts) =
            checker.scalar_slot_observations(&reports, &graph, obs, Span::default(), parts)?;
        checker.scalar_emission_observations(&reports, &graph, obs, Span::default(), parts)
    };
    let start = checker.flow.work;
    let (expected, left) = observe(&mut checker, MAX_EDGES).unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    assert_eq!(expected.primaries.len(), 2);
    assert_eq!(
        expected
            .forwarded
            .keys()
            .filter(|port| matches!(port, Port::Emission(_)))
            .count(),
        2
    );
    let saved = format!("{reports:?}{graph:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = observe(&mut checker, room);
        assert_eq!(result.is_ok(), pass);
        if let Ok((obs, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(obs.primaries, expected.primaries);
            assert_eq!(obs.forwarded, expected.forwarded);
            assert_eq!(obs.scopes, expected.scopes);
            assert_eq!(obs.slots, expected.slots);
        }
        assert_eq!(format!("{reports:?}{graph:?}"), saved);
    }
}
