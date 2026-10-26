use super::super::capture::tests::capture;
use super::*;

#[test]
pub(crate) fn scalar_slot_entries_create_separate_uninitialized_root_and_nested_slots() {
    let (checker, _, reports, snapshots) = capture("x:=7;{};{->true};f<uint8>:(){->1}");
    for (&block, (_, observed)) in &reports.blocks {
        let Some(View::Known(after)) = snapshots.after(Port::BlockEntry(block)) else {
            panic!()
        };
        assert_eq!(after.slot(block).unwrap().data, Data::Uninitialized);
        assert!(!after.slot(block).unwrap().control);
        let Some(View::Known(before)) = snapshots.before(Port::BlockEntry(block)) else {
            panic!()
        };
        assert_eq!(before.cells, after.cells);
        if observed.parent.is_some() {
            assert!(before.slot(block).is_none());
        }
        assert_eq!(after.owner(), checker.bodies[&block].owner);
    }
}

#[test]
pub(crate) fn scalar_slot_entries_preserve_unsupported_shape_boundaries() {
    for source in [
        "r:{->field:=7}",
        "v:{->1.5}",
        "v:{->\"cat\"}",
        "n:7;v:{->&n}",
    ] {
        let (checker, _, _, snapshots) = capture(source);
        for (&block, body) in &checker.bodies {
            if body.parent.is_some() {
                let Some(View::Known(after)) = snapshots.after(Port::BlockEntry(block)) else {
                    panic!()
                };
                assert!(after.slot(block).is_none(), "{source}");
            }
        }
    }
}

#[test]
pub(crate) fn scalar_slot_entries_do_not_restore_restart_or_bottom_state() {
    let (checker, _, _, snapshots) = capture("'out{|false|'out.restart();->7}");
    let block = checker.restart_inputs.values().next().unwrap().target;
    assert_eq!(
        snapshots.before(Port::BlockEntry(block)),
        Some(View::Missing)
    );
    assert_eq!(
        snapshots.after(Port::BlockEntry(block)),
        Some(View::Missing)
    );
    let (_, _, reports, snapshots) = capture("f<int32>:(n<never>){->7}");
    let block = reports.entries[&1].0;
    let Some(View::Known(after)) = snapshots.after(Port::BlockEntry(block)) else {
        panic!()
    };
    assert_eq!(after.reach, FALSE);
    assert!(after.slot(block).is_none());
}

#[test]
pub(crate) fn scalar_slot_entries_bound_publication_and_refuse_reentry() {
    let (mut checker, _, _, mut snapshots) = capture("{}");
    let block = *checker
        .bodies
        .iter()
        .find(|(_, body)| body.parent.is_some())
        .unwrap()
        .0;
    let before = snapshots.rows[&Port::BlockEntry(block)].before;
    let slot = observations::Slot {
        owner: 0,
        declared: Domain::NULL,
        control: None,
    };
    let size = snapshots.states.len();
    let room = snapshots.parts;
    let start = checker.flow.work;
    snapshots
        .enter_slot(block, before, &slot, &mut checker.flow, Span::default())
        .unwrap();
    let cost = checker.flow.work - start;
    let used = room - snapshots.parts;
    snapshots.states.truncate(size);
    for (room, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        snapshots.parts = room;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let saved = format!("{snapshots:?}");
        let result = snapshots.enter_slot(block, before, &slot, &mut checker.flow, Span::default());
        assert_eq!(result.is_ok(), pass);
        if !pass {
            assert_eq!(format!("{snapshots:?}"), saved);
        }
    }
    assert_eq!(snapshots.parts, 0);
    assert!(
        snapshots
            .states
            .last()
            .unwrap()
            .slot(block)
            .unwrap()
            .control
    );
    checker.flow.work = 0;
    checker.flow.full = false;
    snapshots.parts = MAX_EDGES;
    let saved = format!("{snapshots:?}");
    let entered = Value::Known(snapshots.states.len() - 1);
    assert_eq!(
        snapshots
            .enter_slot(block, entered, &slot, &mut checker.flow, Span::default())
            .unwrap(),
        Value::Missing
    );
    assert_eq!(
        snapshots
            .enter_slot(
                block,
                Value::Missing,
                &slot,
                &mut checker.flow,
                Span::default()
            )
            .unwrap(),
        Value::Missing
    );
    assert_eq!(format!("{snapshots:?}"), saved);
}
