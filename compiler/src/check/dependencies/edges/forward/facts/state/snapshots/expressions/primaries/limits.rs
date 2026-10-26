use super::boundaries::setup;
use super::*;

#[test]
pub(crate) fn primary_publication_reserves_exact_work_and_payload_without_partial_changes() {
    for source in [
        "-(({->7;->tag:false}))",
        "!({->true;->tag:7})",
        "-{->7;->tag:false;1+2}",
    ] {
        setup(source, |checker, snapshots, port, primary, obs| {
            let start = checker.flow.work;
            snapshots
                .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                .unwrap();
            let cost = checker.flow.work - start;
            let fact = snapshots.projections.remove(&port).unwrap();
            for (room, extra, pass) in [
                (fact.weight() - 1, 0, false),
                (fact.weight(), 1, false),
                (fact.weight(), 0, true),
            ] {
                snapshots.parts = room;
                checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
                checker.flow.full = false;
                let saved = format!("{snapshots:?}");
                let result = snapshots.primary_expression(
                    port,
                    &primary,
                    obs,
                    &mut checker.flow,
                    Span::default(),
                );
                assert_eq!(result.is_ok(), pass);
                if pass {
                    assert_eq!(snapshots.projections[&port], fact);
                    assert_eq!(snapshots.parts, 0);
                } else {
                    assert_eq!(format!("{snapshots:?}"), saved);
                }
            }
            checker.flow.work = 0;
            checker.flow.full = false;
            snapshots.parts = MAX_EDGES;
            let saved = format!("{snapshots:?}");
            assert!(
                snapshots
                    .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
            assert_eq!(format!("{snapshots:?}"), saved);
        });
    }
}
