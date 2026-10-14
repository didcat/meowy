use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:{->p;->tag:true};a:good.{copy:$;joined:{->copy}};bad:p.{->$;->z:9;->a:true};root:((bad));b:root.{copy:(($));alias:((copy)~<{-><&int32>;a<boolean>;z<int32>}>);joined:{->alias}}";

#[test]
pub(crate) fn receiver_copy_compositions_reject_stale_hops_layouts_targets_and_mixed_cycles() {
    for fault in 0..21 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
        let read = copy_read(&checker, &reports, id);
        let local = checker.local_reads[&read].local;
        let binding = reports.shared_initializers[&local].init;
        let copy = checker.local_reads[&copy_read(&checker, &reports, binding.statement)].local;
        let copied = reports.shared_initializers[&copy].init;
        let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => op.composed.as_mut().unwrap().shared_primary = Some(ScalarKind::Bool),
            1 => {
                op.composed.as_mut().unwrap().count += 1;
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .count += 1;
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
                reports
                    .shared_initializers
                    .get_mut(&copy)
                    .unwrap()
                    .record
                    .fields += 1
            }
            7 => {
                reports.index.operations.remove(&copied.statement);
            }
            8 => {
                checker.dispatch_ops.get_mut(&receiver).unwrap().receiver =
                    Shape::Record { fields: 3 };
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.receiver = Shape::Record { fields: 3 };
            }
            9..=12 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    9 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    10 => slots[2].field = Some("wrong".into()),
                    11 => slots[2].mutable = true,
                    12 => slots[2].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            13..=15 => {
                let target = &mut op.targets[2];
                match fault {
                    13 => target.field = Some("wrong".into()),
                    14 => target.projection = Projection::Field(0),
                    15 => {
                        target.alias = Some(0);
                        target.storage = Some(0);
                    }
                    _ => unreachable!(),
                }
                checker.emissions.get_mut(&id).unwrap().targets[2] = target.clone();
            }
            16 => {
                let group = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&group).unwrap().input = group;
            }
            17 => {
                let original = *checker
                    .local_reads
                    .iter()
                    .find(|(id, op)| {
                        op.local == cell
                            && checker.points[**id].site == checker.points[copied.statement].site
                    })
                    .unwrap()
                    .0;
                let op = checker.local_reads.get_mut(&original).unwrap();
                op.local = copy;
                op.storage = copy;
                let (_, Effect::Read { local, storage, .. }) =
                    reports.effects.get_mut(&original).unwrap()
                else {
                    panic!()
                };
                *local = copy;
                *storage = copy;
            }
            18 => {
                checker.local_reads.get_mut(&read).unwrap().storage = copy;
                let (_, Effect::Read { storage, .. }) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *storage = copy;
            }
            19 => {
                checker
                    .bodies
                    .get_mut(&slot.block)
                    .unwrap()
                    .completion
                    .result = Shape::Record { fields: 3 }
            }
            20 => reports.effects.get_mut(&id).unwrap().0 += 1,
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.emission_sources,
            checker.edge_counts()
        );
        let errors = [
            checker
                .slot_uses(&reports, Span::default())
                .expect_err("slot uses"),
            checker
                .candidate_inputs(&reports, Span::default())
                .expect_err("candidate inputs"),
            checker
                .candidate_walk_report(&reports, Span::default())
                .expect_err("candidate walk"),
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
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.emission_sources,
                checker.edge_counts()
            ),
            before
        );
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .expect_err("unobserved suffix")
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_copy_compositions_validate_original_copy_and_receiver_scopes() {
    for receiver in [false, true] {
        let (mut checker, mut reports) = checked(
            "n:1;p:&n;r:{->p;->tag:true};a:r.{copy:$;joined:{->copy}};b:r.{copy:$;joined:{->copy}}",
        );
        let ids: Vec<_> = checker
            .emissions
            .iter()
            .filter(|(_, op)| op.composed.is_some())
            .map(|(&id, _)| id)
            .collect();
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
        for error in [
            checker.slot_uses(&reports, Span::default()).unwrap_err(),
            checker
                .candidate_inputs(&reports, Span::default())
                .unwrap_err(),
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err(),
        ] {
            assert!(error.message.contains("scope identity"), "{error:?}");
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, mut reports) =
        checked("n:1;p:&n;r:{->p;->tag:true};out:r.{copy:$;inner:3.{joined:{->copy}}}");
    assert_eq!(reports.slot_uses.len(), 2);
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
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("receiver-scope identity")
    );
}
