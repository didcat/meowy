use super::*;
use crate::check::dependencies::edges::forward::consumers::tests::checked;

pub(super) const INT: ScalarKind = ScalarKind::Int {
    bits: 32,
    signed: true,
};

#[test]
pub(crate) fn dispatch_primary_slots_keep_exact_scalar_layouts_and_distinct_owners() {
    for source in [
        "r:3.{->$;->tag:true};f<int32>:(){r:4.{->$;->tag:true};->r}",
        "n<int8>:3;r:n.{->$;->tag:true}",
        "n<uint16>:3;r:n.{->$;->tag:true}",
        "n<float32>:1.5;r:n.{->$;->tag:true}",
        "r:true.{->$;->tag:true}",
        "r:\"cat\".{->$;->tag:true}",
        "r:3.{->tag:true}",
    ] {
        let (mut checker, reports) = checked(source);
        let ops: Vec<_> = checker
            .dispatch_ops
            .iter()
            .map(|(&id, op)| (id, op.owner, op.block))
            .collect();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for (id, owner, block) in ops {
            let Layout::Slots(slots) = &checker.bodies[&block].layout else {
                panic!()
            };
            let Shape::Scalar(ty) = slots[0].shape else {
                panic!()
            };
            assert_eq!(
                checker
                    .dispatch_primary_slot(&reports, id, owner, ty, Span::default())
                    .unwrap(),
                Some(Slot { block, index: 0 })
            );
            assert_eq!(
                checker
                    .primary_slot(&reports, id, owner, Span::default())
                    .unwrap(),
                None
            );
            assert_eq!(reports.results[&block].1.dispatch, Some(id));
            assert!(reports.results[&block].1.consumer.is_none());
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_primary_slots_require_results_and_keep_nonscalars_opaque() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true}");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let block = op.block;
    for initialized in [false, true] {
        for result in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            assert_eq!(
                checker
                    .dispatch_primary_slot(&reports, id, 0, INT, Span::default())
                    .unwrap(),
                result.then_some(Slot { block, index: 0 })
            );
        }
    }
    reports.results.remove(&block);
    assert_eq!(
        checker
            .dispatch_primary_slot(&reports, id, 0, INT, Span::default())
            .unwrap(),
        None
    );
    for source in [
        "r:3.{->$}",
        "r:3.{->[1,2];->tag:true}",
        "n:1;r:3.{->&n;->tag:true}",
        "n<int32><null>:1;r:3.{->n;->tag:true}",
        "d:@\"debug\";r:3.{->$;->tag:true;d.panic(\"stop\")}",
    ] {
        let (mut checker, reports) = checked(source);
        let id = *checker.dispatch_ops.keys().next().unwrap();
        assert_eq!(
            checker
                .dispatch_primary_slot(&reports, id, 0, INT, Span::default())
                .unwrap(),
            None,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn dispatch_primary_slots_reject_wrong_kinds_owners_origins_and_slot_headers() {
    for fault in 0..7 {
        let (mut checker, mut reports) = checked("r:3.{->$;->tag:true}");
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let block = op.block;
        let mut ty = INT;
        match fault {
            0 => {
                ty = ScalarKind::Int {
                    bits: 8,
                    signed: true,
                }
            }
            1 => {
                ty = ScalarKind::Int {
                    bits: 32,
                    signed: false,
                }
            }
            2 => ty = ScalarKind::Bool,
            3 => reports.results.get_mut(&block).unwrap().0 += 1,
            4 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            5 | 6 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 5 {
                    slots[0].mutable = true;
                } else {
                    slots[0].field = Some("wrong".into());
                }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .dispatch_primary_slot(&reports, id, 0, ty, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_primary_slots_share_exact_work_without_payload_or_row_copies() {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true}");
    let id = *checker.dispatch_ops.keys().next().unwrap();
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let expected = checker
        .dispatch_primary_slot(&reports, id, 0, INT, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.dispatch_primary_slot(&reports, id, 0, INT, Span::default());
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
