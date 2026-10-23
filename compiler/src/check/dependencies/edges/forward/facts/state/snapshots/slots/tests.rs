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
        "r:{->field:7}",
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
