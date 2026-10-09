use super::*;

pub(super) const SOURCE: &str = "n:1;p:&n;good:p.{->$;->tag:true}.{a<&int32>:$};bad:p.{->$;->tag:false}.{b<&int32><null>:((($))~<{-><&int32>;tag<boolean>}>)}";

#[test]
pub(crate) fn shared_receiver_coercion_links_reject_late_types_descriptors_scopes_and_cycles() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        match fault {
            0..=5 => {
                let source = match fault {
                    0 => None,
                    1 => Some(Shape::SharedScalar(ScalarKind::Bool)),
                    2 => Some(Shape::SharedScalar(ScalarKind::Int {
                        bits: 16,
                        signed: true,
                    })),
                    3 => Some(Shape::SharedScalar(ScalarKind::Int {
                        bits: 32,
                        signed: false,
                    })),
                    4 => Some(Shape::SharedScalar(ScalarKind::Int {
                        bits: 7,
                        signed: true,
                    })),
                    5 => Some(Shape::Never),
                    _ => unreachable!(),
                };
                checker.coercions.get_mut(&id).unwrap().source = source;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = source;
            }
            6 => reports.receivers.get_mut(&local).unwrap().2 = Some(ScalarKind::Bool),
            7 => {
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None
            }
            8 => {
                reports.index.operations.remove(&id);
            }
            9 => {
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
            10 => {
                let input = checker.typed_ops.values().last().unwrap().input;
                let mut group = checker.group_inputs[&input];
                group.input = input;
                checker.group_inputs.insert(input, group);
            }
            11 => reports.effects.get_mut(&id).unwrap().0 += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .slot_uses(&reports, Span::default())
            .expect_err(&format!("fault {fault}"));
        assert!(error.message.contains("identity"), "{error:?}");
        if fault == 9 {
            assert!(error.message.contains("receiver-scope"), "{error:?}");
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_coercion_links_keep_missing_evidence_opaque_and_earlier_links_intact()
{
    for fault in 0..7 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let (&local, &(_, receiver, _)) = reports
            .receivers
            .iter()
            .rev()
            .find(|(_, entry)| entry.2.is_some())
            .unwrap();
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
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
                reports.receivers.get_mut(&local).unwrap().2 = None;
                checker
                    .dispatch_ops
                    .get_mut(&receiver)
                    .unwrap()
                    .shared_primary = None;
            }
            5 => {
                let typed = *checker.typed_ops.keys().last().unwrap();
                reports.effects.remove(&typed);
            }
            6 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = false;
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
pub(crate) fn shared_receiver_coercion_links_keep_three_control_flags_and_exact_limits_independent()
{
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
                for (&id, op) in &mut checker.coercions {
                    op.control = consumer;
                    if let Some((_, Effect::Coercion(observed))) = reports.effects.get_mut(&id) {
                        observed.control = consumer;
                    }
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
