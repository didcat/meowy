use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:p.{->$;->tag:true}.{copy:{->$.tag}};bad:p.{->$;->tag:false}.{copy:{->((($))~<{-><&int32>;tag<boolean>}>).tag}}";

#[test]
pub(crate) fn shared_field_links_reject_late_type_layout_scope_and_cycle_faults_atomically() {
    for fault in 0..13 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        match fault {
            0..=3 => {
                let primary = Some(match fault {
                    0 | 1 => ScalarKind::Bool,
                    2 => ScalarKind::Int {
                        bits: 16,
                        signed: true,
                    },
                    3 => ScalarKind::Int {
                        bits: 32,
                        signed: false,
                    },
                    _ => unreachable!(),
                });
                if fault != 0 {
                    checker.fields.get_mut(&id).unwrap().shared_primary = primary;
                }
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = primary;
            }
            4 => reports.receivers.get_mut(&local).unwrap().2 = None,
            5 => {
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None
            }
            6 | 7 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 6 {
                    slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
                } else {
                    slots[slot.index].mutable = true;
                }
            }
            8 => {
                reports.index.operations.remove(&id);
            }
            9 => reports.effects.get_mut(&id).unwrap().0 += 1,
            10 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                let mut group = checker.group_inputs[&input];
                group.input = input;
                checker.group_inputs.insert(input, group);
            }
            11 => {
                let other = *reports
                    .receivers
                    .iter()
                    .find(|(id, entry)| **id != local && entry.2.is_some())
                    .unwrap()
                    .0;
                let read = *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == local)
                    .unwrap()
                    .0;
                let op = checker.local_reads.get_mut(&read).unwrap();
                op.local = other;
                op.storage = other;
                let (_, Effect::Read { local, storage, .. }) =
                    reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *local = other;
                *storage = other;
            }
            12 => checker.fields.get_mut(&id).unwrap().count += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
        if fault == 11 {
            assert!(error.message.contains("receiver-scope"), "{error:?}");
        }
        assert!(
            checker
                .field_results(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_field_links_keep_missing_evidence_opaque_without_erasing_earlier_links() {
    for fault in 0..7 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        match fault {
            0 => {
                reports.receivers.remove(&local);
            }
            1 => {
                reports.effects.remove(&receiver);
            }
            2 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = false;
            }
            3 => {
                reports.results.remove(&slot.block);
            }
            4 => {
                let typed = *checker.typed_ops.keys().last().unwrap();
                reports.effects.remove(&typed);
            }
            5 => {
                checker.fields.get_mut(&id).unwrap().shared_primary = None;
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = None;
            }
            6 => {
                let (_, Effect::Field { operation, .. }) = reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *operation = false;
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&Port::Operation(id));
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let actual = checker.slot_uses(&reports, Span::default()).unwrap();
        assert_eq!(actual, expected, "fault {fault}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        reports.slot_uses = actual;
        assert_eq!(
            checker
                .field_result_slot(&reports, id, 0, &reports.effects[&id].1, Span::default())
                .unwrap(),
            None
        );
        let (&Port::Operation(first), &(owner, slot)) = expected.first_key_value().unwrap() else {
            panic!()
        };
        assert_eq!(
            checker
                .field_result_slot(
                    &reports,
                    first,
                    owner,
                    &reports.effects[&first].1,
                    Span::default()
                )
                .unwrap(),
            Some(slot)
        );
    }
}
