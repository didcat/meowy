use super::super::boundaries::setup;
use super::*;

#[test]
pub(crate) fn stored_projection_publication_rejects_foreign_record_histories_atomically() {
    for fault in 0..4 {
        setup(
            "r:{->7;->tag:false};v<int32>:r",
            |checker, snapshots, port, primary, obs| {
                assert!(primary.stored);
                match fault {
                    0 => snapshots.records.get_mut(&primary.input).unwrap().0 += 1,
                    1 => snapshots.records.get_mut(&primary.input).unwrap().1.owner += 1,
                    2 => {
                        snapshots
                            .rows
                            .get_mut(&Port::Normal(primary.input))
                            .unwrap()
                            .after = Value::Missing
                    }
                    3 => obs.scopes.get_mut(&primary.block).unwrap().owner += 1,
                    _ => unreachable!(),
                }
                let saved = format!("{snapshots:?}");
                assert!(
                    snapshots
                        .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                        .unwrap_err()
                        .message
                        .contains("identity")
                );
                assert_eq!(format!("{snapshots:?}"), saved);
            },
        );
    }
}

#[test]
pub(crate) fn stored_projection_publication_keeps_missing_control_facts_and_incompatible_values_unavailable()
 {
    for fault in 0..4 {
        setup(
            "r:{->7;->tag:false};v<int32>:r",
            |checker, snapshots, port, primary, obs| {
                match fault {
                    0 => {
                        snapshots.records.remove(&primary.input);
                    }
                    1 => snapshots.records.get_mut(&primary.input).unwrap().1.value = None,
                    2 => {
                        snapshots
                            .records
                            .get_mut(&primary.input)
                            .unwrap()
                            .1
                            .value
                            .as_mut()
                            .unwrap()
                            .domain = Domain::boolean(true)
                    }
                    3 => {
                        for port in [port, Port::Normal(primary.input)] {
                            snapshots.rows.get_mut(&port).unwrap().control = None;
                        }
                    }
                    _ => unreachable!(),
                }
                snapshots
                    .primary_expression(port, &primary, obs, &mut checker.flow, Span::default())
                    .unwrap();
                assert!(snapshots.projections[&port].value.is_none());
            },
        );
    }
}
