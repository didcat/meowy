use super::tests::setup;
use super::*;

#[test]
pub(crate) fn record_storage_reserves_exact_work_and_shared_state_payload_atomically() {
    let (mut checker, mut snapshots, definitions, _) = setup("x:=7;r:(({->x;->tag:false;x=9}))");
    let point = *definitions.records.keys().next().unwrap();
    let before = snapshots.rows[&Port::Operation(point)].before;
    let start = checker.flow.work;
    let room = snapshots.parts;
    let count = snapshots.states.len();
    let Some(Value::Known(id)) = snapshots
        .storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(id, count);
    let cost = checker.flow.work - start;
    let used = room - snapshots.parts;
    let expected = snapshots.states.pop().unwrap();
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let saved = format!("{snapshots:?}");
        let result = snapshots.storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        );
        assert_eq!(result.is_ok(), pass);
        if pass {
            assert_eq!(result.unwrap(), Some(Value::Known(count)));
            assert_eq!(snapshots.states[count], expected);
            assert_eq!(snapshots.parts, 0);
        } else {
            assert_eq!(format!("{snapshots:?}"), saved);
        }
    }
}

#[test]
pub(crate) fn record_storage_rejects_foreign_sources_and_incompatible_values_without_mutation() {
    for fault in 0..3 {
        let (mut checker, mut snapshots, mut definitions, _) = setup("x:=7;r:{->x;->tag:false}");
        let point = *definitions.records.keys().next().unwrap();
        let storage = &definitions.records[&point].storage;
        let before = snapshots.rows[&Port::Operation(point)].before;
        match fault {
            0 => snapshots.records.get_mut(&storage.input).unwrap().0 += 1,
            1 => {
                snapshots
                    .records
                    .get_mut(&storage.input)
                    .unwrap()
                    .1
                    .value
                    .as_mut()
                    .unwrap()
                    .domain = Domain::boolean(true)
            }
            2 => {
                definitions
                    .records
                    .get_mut(&point)
                    .unwrap()
                    .storage
                    .version
                    .owner += 1
            }
            _ => unreachable!(),
        }
        let saved = format!("{snapshots:?}");
        assert!(
            snapshots
                .storage(
                    point,
                    before,
                    &definitions,
                    &mut checker.flow,
                    Span::default()
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{snapshots:?}"), saved);
    }
}
