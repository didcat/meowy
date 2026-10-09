use super::*;

pub(super) const SOURCE: &str =
    "n:1;p:&n;good:p.{->$;->tag:true}.{->$==p};bad:p.{->$;->tag:false}.{inner:{->(($))==p}}";

#[test]
pub(crate) fn shared_receiver_links_reject_late_identities_cycles_and_crossed_dispatch_scopes() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let id = *checker.binaries.keys().last().unwrap();
        let block = reports.slot_uses[&Port::Projection { point: id, step: 0 }]
            .1
            .block;
        match fault {
            0 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            1 => reports.receivers.get_mut(&local).unwrap().1 = usize::MAX,
            2 => reports.receivers.get_mut(&local).unwrap().2 = Some(ScalarKind::Bool),
            3 => {
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None
            }
            4 => {
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = Some(ScalarKind::Bool);
                reports.receivers.get_mut(&local).unwrap().2 = Some(ScalarKind::Bool);
            }
            5 | 8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 5 {
                    slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
                } else {
                    slots[0].mutable = true;
                }
            }
            6 => {
                let other = *reports
                    .receivers
                    .iter()
                    .find(|(id, entry)| **id != local && entry.2.is_some())
                    .unwrap()
                    .0;
                let id = *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == local)
                    .unwrap()
                    .0;
                let op = checker.local_reads.get_mut(&id).unwrap();
                op.local = other;
                op.storage = other;
                let (_, Effect::Read { local, storage, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *local = other;
                *storage = other;
            }
            7 => {
                reports.index.operations.remove(&id);
            }
            9 => {
                let (&id, group) = checker.group_inputs.last_key_value().unwrap();
                let mut group = *group;
                group.input = id;
                checker.group_inputs.insert(id, group);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        if fault == 6 {
            assert!(error.message.contains("receiver-scope"), "{error:?}");
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_links_keep_missing_sources_and_eligibility_opaque() {
    for fault in 0..6 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let id = *checker.binaries.keys().last().unwrap();
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
        let source = reports.results[&block].1.dispatch.unwrap();
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
                reports.results.remove(&block);
            }
            4 => {
                reports.effects.remove(&source);
            }
            5 => {
                reports.receivers.get_mut(&local).unwrap().2 = None;
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None;
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&port);
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_links_keep_three_control_flags_and_exact_limits_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    for source in [false, true] {
        for receiver in [false, true] {
            for consumer in [false, true] {
                for (&id, op) in &mut checker.dispatch_ops {
                    op.control = if op.shared_primary.is_some() {
                        receiver
                    } else {
                        source
                    };
                    let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = op.control;
                }
                for (&id, op) in &mut checker.binaries {
                    op.control = consumer;
                    let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    observed.control = consumer;
                }
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    expected
                );
            }
        }
    }
    reports.parts = 0;
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + expected.len();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - start;
    assert!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
