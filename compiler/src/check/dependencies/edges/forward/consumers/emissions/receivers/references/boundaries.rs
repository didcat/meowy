use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good:p.{->$;->tag:true}.{copy:{->$}};bad:p.{->$;->z:9;->a:true}.{copy:{->((($))~<{-><&int32>;a<boolean>;z<int32>}>)}}";

#[test]
pub(crate) fn shared_emission_links_reject_late_referent_suffix_scope_and_cycle_conflicts() {
    for fault in 0..16 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap()
            .0;
        let last = checker.emissions[&id].targets[2].id;
        let slot = reports.slot_uses[&Port::Emission(last)].1;
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        op.initialized = vec![true, false, false];
        op.result = false;
        match fault {
            0..=2 => {
                let primary = Some(match fault {
                    0 => ScalarKind::Bool,
                    1 => ScalarKind::Int {
                        bits: 16,
                        signed: true,
                    },
                    2 => ScalarKind::Int {
                        bits: 32,
                        signed: false,
                    },
                    _ => unreachable!(),
                });
                op.composed.as_mut().unwrap().shared_primary = primary;
                if fault != 0 {
                    checker
                        .emissions
                        .get_mut(&id)
                        .unwrap()
                        .composed
                        .as_mut()
                        .unwrap()
                        .shared_primary = primary;
                }
            }
            3 => reports.receivers.get_mut(&local).unwrap().2 = None,
            4 => {
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None
            }
            5 | 6 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&slot.block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 5 {
                    slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
                } else {
                    slots[2].field = Some("wrong".into());
                }
            }
            7..=9 => {
                let target = &mut op.targets[2];
                match fault {
                    7 => target.field = Some("wrong".into()),
                    8 => target.projection = Projection::Field(0),
                    9 => {
                        target.alias = Some(0);
                        target.storage = Some(0);
                    }
                    _ => unreachable!(),
                }
                checker.emissions.get_mut(&id).unwrap().targets[2] = target.clone();
            }
            10 => {
                checker.emission_sources.insert(last, (id, 0));
            }
            11 => reports.effects.get_mut(&id).unwrap().0 += 1,
            12 => checker.bodies.get_mut(&slot.block).unwrap().owner += 1,
            13 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                checker.group_inputs.get_mut(&input).unwrap().input = input;
            }
            14 => {
                let other = *reports
                    .receivers
                    .iter()
                    .find(|(id, entry)| **id != local && entry.2.is_some())
                    .unwrap()
                    .0;
                let (&read, op) = checker
                    .local_reads
                    .iter_mut()
                    .find(|(_, op)| op.local == local)
                    .unwrap();
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
            15 => {
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
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.emission_sources,
            checker.edge_counts()
        );
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
        if fault == 14 {
            assert!(error.message.contains("receiver-scope"));
        }
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.emission_sources,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn shared_emission_links_keep_missing_evidence_opaque_and_preserve_earlier_links() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let ports: BTreeSet<_> = op
            .targets
            .iter()
            .map(|target| Port::Emission(target.id))
            .collect();
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
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
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            5 | 6 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                if fault == 5 {
                    op.composed.as_mut().unwrap().shared_primary = None;
                    checker
                        .emissions
                        .get_mut(&id)
                        .unwrap()
                        .composed
                        .as_mut()
                        .unwrap()
                        .shared_primary = None;
                } else {
                    op.initialized.fill(false);
                }
            }
            7 => {
                checker.proofs.mutable.insert(local);
            }
            _ => unreachable!(),
        }
        let expected: Uses = reports
            .slot_uses
            .iter()
            .filter(|(port, _)| !ports.contains(port))
            .map(|(&port, &slot)| (port, slot))
            .collect();
        assert_eq!(expected.len(), 2);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
