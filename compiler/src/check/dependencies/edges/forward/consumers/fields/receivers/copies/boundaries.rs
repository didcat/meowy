use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true;->unused:1};a:good.{copy:$;value:{->copy.tag}};bad:p.{->$;->tag:false;->unused:2};alias:((bad));b:alias.{copy:(($));next:((copy)~<{-><&int32>;tag<boolean>;unused<int32>}>);value:{->next.tag}}";

pub(super) fn copy_read(checker: &Checker, reports: &Reports, id: PointId) -> PointId {
    *checker
        .local_reads
        .iter()
        .find(|(read, op)| {
            checker.points[**read].site == checker.points[id].site
                && reports.shared_initializers.contains_key(&op.local)
        })
        .unwrap()
        .0
}

#[test]
pub(crate) fn shared_receiver_copy_fields_reject_stale_hop_descriptors_and_mixed_cycles_atomically()
{
    for fault in 0..19 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let read = copy_read(&checker, &reports, id);
        let local = checker.local_reads[&read].local;
        let binding = reports.shared_initializers[&local].init;
        let copy = checker.local_reads[&copy_read(&checker, &reports, binding.statement)].local;
        let copied = reports.shared_initializers[&copy].init;
        let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
        match fault {
            0 => checker.fields.get_mut(&id).unwrap().count += 1,
            1 => {
                checker.fields.get_mut(&id).unwrap().shared_primary = Some(ScalarKind::Bool);
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = Some(ScalarKind::Bool);
            }
            2 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .fields += 1
            }
            3 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .record
                    .primary = ScalarKind::Bool
            }
            4 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .owner += 1
            }
            5 => {
                checker
                    .operations
                    .get_mut(&binding.statement)
                    .unwrap()
                    .shared = None
            }
            6 => {
                reports.index.operations.remove(&binding.statement);
            }
            7 => {
                reports
                    .shared_initializers
                    .get_mut(&copy)
                    .unwrap()
                    .record
                    .fields += 1
            }
            8 => {
                checker
                    .operations
                    .get_mut(&copied.statement)
                    .unwrap()
                    .shared = None
            }
            9 => {
                checker.dispatch_ops.get_mut(&receiver).unwrap().receiver =
                    Shape::Record { fields: 3 };
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.receiver = Shape::Record { fields: 3 };
            }
            10..=12 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    10 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    11 => slots.last_mut().unwrap().mutable = true,
                    12 => slots.last_mut().unwrap().shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            13 => {
                let group = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&group).unwrap().input = group;
            }
            14 | 15 => {
                let (read, local) = if fault == 14 {
                    let alias = checker.local_reads[&copy_read(&checker, &reports, receiver)].local;
                    (
                        copy_read(
                            &checker,
                            &reports,
                            reports.shared_initializers[&alias].init.statement,
                        ),
                        alias,
                    )
                } else {
                    (
                        *checker
                            .local_reads
                            .iter()
                            .find(|(id, op)| {
                                op.local == cell
                                    && checker.points[**id].site
                                        == checker.points[copied.statement].site
                            })
                            .unwrap()
                            .0,
                        copy,
                    )
                };
                let op = checker.local_reads.get_mut(&read).unwrap();
                op.local = local;
                op.storage = local;
                let (
                    _,
                    Effect::Read {
                        local: reported,
                        storage,
                        ..
                    },
                ) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *reported = local;
                *storage = local;
            }
            16 => reports.effects.get_mut(&id).unwrap().0 += 1,
            17 => {
                checker
                    .bodies
                    .get_mut(&slot.block)
                    .unwrap()
                    .completion
                    .result = Shape::Record { fields: 3 }
            }
            18 => {
                checker.local_reads.get_mut(&read).unwrap().storage = copy;
                let (_, Effect::Read { storage, .. }) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *storage = copy;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let errors = [
            checker
                .slot_uses(&reports, Span::default())
                .expect_err("slot uses"),
            checker
                .field_results(&reports, Span::default())
                .expect_err("field results"),
            checker
                .direct_walk_report(&reports, Span::default())
                .expect_err("expanded walk"),
        ];
        assert!(
            errors
                .iter()
                .all(|error| error.message.contains("identity")),
            "fault {fault}: {errors:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_copy_fields_validate_each_original_local_and_receiver_scope() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;r:{->p;->tag:true};a:r.{copy:$;value:{->copy.tag}};b:r.{copy:$;value:{->copy.tag}}",
        );
        let ids: Vec<_> = checker.fields.keys().copied().collect();
        let reads: Vec<_> = ids
            .iter()
            .map(|id| copy_read(&checker, &reports, *id))
            .collect();
        let (id, other) = if receiver {
            let (&cell, _) = reports.receivers.last_key_value().unwrap();
            (
                *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == cell)
                    .unwrap()
                    .0,
                *reports.receivers.keys().next().unwrap(),
            )
        } else {
            (reads[1], checker.local_reads[&reads[0]].local)
        };
        let op = checker.local_reads.get_mut(&id).unwrap();
        op.local = other;
        op.storage = other;
        let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        *local = other;
        *storage = other;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, mut reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;inner:3.{->copy.tag}}");
    assert_eq!(reports.slot_uses.len(), 1);
    let cells: Vec<_> = reports.receivers.keys().copied().collect();
    let (&id, op) = checker
        .local_reads
        .iter_mut()
        .find(|(_, op)| op.local == cells[0])
        .unwrap();
    op.local = cells[1];
    op.storage = cells[1];
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = cells[1];
    *storage = cells[1];
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("receiver-scope identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
