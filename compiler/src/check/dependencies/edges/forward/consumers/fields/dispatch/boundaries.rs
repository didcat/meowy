use super::*;
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn record_dispatch_fields_reject_corrupt_selections_and_retained_links() {
    for fault in 0..13 {
        let (mut checker, mut reports) =
            checked("a:3.{->n:1};b:4.{->n:2;->z:3};out:{->a.n;->again:b.n}");
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
        match fault {
            0 => checker.fields.get_mut(&id).unwrap().count += 1,
            1 => {
                checker.fields.get_mut(&id).unwrap().index = usize::MAX;
                let (_, Effect::Field { index, .. }) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                *index = usize::MAX;
            }
            2..=5 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    2 => slots[slot.index].field = None,
                    3 => slots[2].field = slots[1].field.clone(),
                    4 => {
                        slots[slot.index].shape = Shape::Scalar(ScalarKind::Int {
                            bits: 7,
                            signed: true,
                        })
                    }
                    5 => slots[slot.index].shape = Shape::Record { fields: 0 },
                    _ => unreachable!(),
                }
            }
            6 => reports.slot_uses.get_mut(&Port::Operation(id)).unwrap().0 += 1,
            7 => {
                reports
                    .slot_uses
                    .get_mut(&Port::Operation(id))
                    .unwrap()
                    .1
                    .index = 0
            }
            8 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(id))
                    .unwrap()
                    .1
                    .index = 0
            }
            9 => reports.results.get_mut(&slot.block).unwrap().1.consumer = Some(dispatch),
            10 => {
                reports.effects.remove(&id);
            }
            11 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            12 => checker.points[id].complete = false,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn record_dispatch_fields_reject_overlapping_producers_along_input_wrappers() {
    for fault in 0..3 {
        let (mut checker, reports) = checked("r:3.{->n:1};copy:((r));out:{->copy.n}");
        let (&field, op) = checker.fields.first_key_value().unwrap();
        let input = op.input;
        let dispatch = *checker.dispatch_ops.keys().next().unwrap();
        let target = match fault {
            0 => dispatch,
            1 => *checker.local_reads.keys().last().unwrap(),
            _ => *checker.group_inputs.keys().next().unwrap(),
        };
        checker
            .fields
            .insert(target, checker.fields[&field].clone());
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .record_dispatch_body(&reports, input, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_record_dispatch_fields_reject_cycles_across_local_initializers() {
    let (mut checker, mut reports) = checked("r:3.{->n:1};copy:((r));out:{->copy.n}");
    let (&id, _) = checker.local_reads.first_key_value().unwrap();
    let copy = checker.local_reads.last_key_value().unwrap().1.local;
    let read = checker.local_reads.get_mut(&id).unwrap();
    read.local = copy;
    read.storage = copy;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = copy;
    *storage = copy;
    let field = *checker.fields.keys().next().unwrap();
    let input = checker.fields[&field].input;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .record_dispatch_body(&reports, input, 0, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .field_slot(
                &reports,
                field,
                0,
                &reports.effects[&field].1,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
