use super::super::super::versions::tests::context;
use super::super::tests::value;
use super::*;

#[test]
pub(crate) fn scalar_snapshots_keep_seed_and_before_after_histories_independent() {
    let (mut checker, program, reports) = context("x:=1;x=2");
    let (_, _, mut entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let mut snapshots = checker
        .scalar_entry_snapshots(&graph, &entries, Span::default(), parts)
        .unwrap();
    let entry = entries[&0].0;
    let point = *checker.operations.last_key_value().unwrap().0;
    let mut current = snapshots
        .seed(entry)
        .unwrap()
        .snapshot(&mut checker.flow, Span::default(), snapshots.room())
        .unwrap();
    let old = value(0, point, 7, &mut checker.flow);
    current
        .put(
            0,
            &old,
            &mut checker.flow,
            Span::default(),
            snapshots.room(),
        )
        .unwrap();
    let before = snapshots
        .copy(&current, &mut checker.flow, Span::default())
        .unwrap();
    let new = value(0, point, 9, &mut checker.flow);
    current
        .put(
            0,
            &new,
            &mut checker.flow,
            Span::default(),
            snapshots.room(),
        )
        .unwrap();
    let after = snapshots
        .copy(&current, &mut checker.flow, Span::default())
        .unwrap();
    let port = Port::Operation(point);
    snapshots
        .record(
            port,
            Row {
                control: None,
                owner: 0,
                before,
                after,
            },
            &mut checker.flow,
            Span::default(),
        )
        .unwrap();
    entries
        .get_mut(&0)
        .unwrap()
        .1
        .put(0, &new, &mut checker.flow, Span::default(), MAX_EDGES)
        .unwrap();
    let Some(View::Known(before)) = snapshots.before(port) else {
        panic!()
    };
    let Some(View::Known(after)) = snapshots.after(port) else {
        panic!()
    };
    assert_eq!(before.get(0), Some(&old));
    assert_eq!(after.get(0), Some(&new));
    assert!(matches!(
        snapshots.seed(entry).unwrap().get(0).unwrap().data,
        Data::Uninitialized
    ));
    assert!(
        snapshots
            .record(
                port,
                Row {
                    control: None,
                    owner: 0,
                    before: Value::Missing,
                    after: Value::Missing
                },
                &mut checker.flow,
                Span::default()
            )
            .is_err()
    );
    assert!(matches!(snapshots.before(port), Some(View::Known(_))));
}

#[test]
pub(crate) fn scalar_snapshots_distinguish_unvisited_missing_bottom_and_foreign_frames() {
    let (mut checker, program, reports) =
        context("impossible<never>:(n<never>){->n};forever<never>:() 'loop {'loop.restart()}");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let mut snapshots = checker
        .scalar_entry_snapshots(&graph, &entries, Span::default(), parts)
        .unwrap();
    assert_eq!(snapshots.before(Port::Entry(usize::MAX)), None);
    let never = entries[&1].0;
    let Some(View::Known(bottom)) = snapshots.before(never) else {
        panic!()
    };
    assert_eq!(bottom.reach, FALSE);
    let repeat = entries[&2].0;
    assert_eq!(snapshots.before(repeat), Some(View::Missing));
    assert_eq!(snapshots.after(repeat), Some(View::Missing));
    assert_eq!(snapshots.seed(repeat).unwrap().reach, crate::flow::TRUE);
    let foreign = Value::Known(snapshots.seeds[&never]);
    let port = Port::BlockResult(reports.entries[&0].0);
    assert!(
        snapshots
            .record(
                port,
                Row {
                    control: None,
                    owner: 0,
                    before: foreign,
                    after: foreign
                },
                &mut checker.flow,
                Span::default()
            )
            .is_err()
    );
    assert_eq!(snapshots.before(port), None);
}

#[test]
pub(crate) fn scalar_snapshots_bound_seed_copies_and_row_payload_without_mutating_entries() {
    let (mut checker, program, reports) = context("a:7;f:(n<int32>){}");
    let (_, _, entries, parts) = checker
        .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (ports, parts) = checker
        .scalar_ports(&reports, &entries, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .scalar_graph(&reports, ports, Span::default(), parts)
        .unwrap();
    let start = checker.flow.work;
    let expected = checker
        .scalar_entry_snapshots(&graph, &entries, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - expected.room();
    let before = format!("{entries:?}{reports:?}");
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_entry_snapshots(&graph, &entries, Span::default(), room);
        assert_eq!(result.is_ok(), pass);
        if let Ok(snapshots) = result {
            assert_eq!(snapshots.room(), 0);
            assert_eq!(snapshots.states, expected.states);
            assert_eq!(snapshots.rows, expected.rows);
        }
        assert_eq!(format!("{entries:?}{reports:?}"), before);
    }
}

#[test]
pub(crate) fn scalar_snapshot_rows_reserve_completion_control_with_their_state_references() {
    let (mut checker, _, _, mut snapshots) = super::capture::tests::capture("{}");
    let port = Port::Normal(usize::MAX);
    for control in [None, Some(false), Some(true)] {
        for (room, pass) in [(2, false), (3, true)] {
            snapshots.parts = room;
            let saved = format!("{snapshots:?}");
            let result = snapshots.record(
                port,
                Row {
                    owner: 0,
                    before: Value::Missing,
                    after: Value::Missing,
                    control,
                },
                &mut checker.flow,
                Span::default(),
            );
            assert_eq!(result.is_ok(), pass);
            if pass {
                assert_eq!(snapshots.rows.remove(&port).unwrap().control, control);
                assert_eq!(snapshots.parts, 0);
            } else {
                assert_eq!(format!("{snapshots:?}"), saved);
            }
        }
    }
}
