use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true;->unused:1};a:good.{copy:{->$.tag}};bad:p.{->$;->tag:false;->unused:2};copy:((bad));b:copy.{copy:{->(($)~<{-><&int32>;tag<boolean>;unused<int32>}>).tag}}";

#[test]
pub(crate) fn shared_local_receiver_fields_reject_late_descriptors_layouts_and_cycles_atomically() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let (&local, row) = reports.shared_initializers.last_key_value().unwrap();
        let statement = row.init.statement;
        let receiver = reports.receivers.values().last().unwrap().1;
        match fault {
            0 => checker.fields.get_mut(&id).unwrap().count += 1,
            1 => {
                let op = checker.dispatch_ops.get_mut(&receiver).unwrap();
                op.receiver = Shape::Record { fields: 3 };
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&receiver).unwrap()
                else {
                    panic!()
                };
                observed.receiver = op.receiver;
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
            5 => checker.operations.get_mut(&statement).unwrap().shared = None,
            6 => {
                reports.index.operations.remove(&statement);
            }
            7..=9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    7 => slots.last_mut().unwrap().mutable = true,
                    8 => slots.last_mut().unwrap().shape = Shape::SharedScalar(ScalarKind::Bool),
                    9 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            10 => {
                checker
                    .bodies
                    .get_mut(&slot.block)
                    .unwrap()
                    .completion
                    .result = Shape::Record { fields: 3 }
            }
            11 => reports.effects.get_mut(&receiver).unwrap().0 += 1,
            12 => {
                let group = *checker.group_inputs.keys().last().unwrap();
                checker.group_inputs.get_mut(&group).unwrap().input = group;
            }
            13 => {
                let read = *checker
                    .local_reads
                    .iter()
                    .find(|(id, op)| {
                        op.local != local
                            && reports.shared_initializers.contains_key(&op.local)
                            && checker.points[**id].site == checker.points[statement].site
                    })
                    .unwrap()
                    .0;
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
            14 => {
                checker.fields.get_mut(&id).unwrap().shared_primary = Some(ScalarKind::Bool);
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = Some(ScalarKind::Bool);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
        assert!(
            checker
                .field_results(&reports, Span::default())
                .expect_err(&format!("field results, fault {fault}"))
                .message
                .contains("identity")
        );
        assert!(
            checker
                .direct_sources(&reports, Span::default())
                .expect_err(&format!("direct sources, fault {fault}"))
                .message
                .contains("identity")
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .expect_err(&format!("expanded walk, fault {fault}"))
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_receiver_fields_revalidate_receiver_and_initializer_scopes() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;a:{r:{->p;->tag:true};->r.{->$.tag}};b:{r:{->p;->tag:false};->r.{->$.tag}}",
        );
        let (other, local) = if receiver {
            (
                *reports.receivers.keys().next().unwrap(),
                *reports.receivers.keys().last().unwrap(),
            )
        } else {
            (
                *reports.shared_initializers.keys().next().unwrap(),
                *reports.shared_initializers.keys().last().unwrap(),
            )
        };
        let id = *checker
            .local_reads
            .iter()
            .find(|(_, op)| op.local == local)
            .unwrap()
            .0;
        let op = checker.local_reads.get_mut(&id).unwrap();
        op.local = other;
        op.storage = other;
        let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        *local = other;
        *storage = other;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker.slot_uses(&reports, Span::default()).unwrap_err();
        assert!(error.message.contains("scope identity"), "{error:?}");
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("scope identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
