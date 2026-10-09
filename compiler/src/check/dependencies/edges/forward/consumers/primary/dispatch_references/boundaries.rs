use super::*;

pub(super) const SOURCE: &str =
    "n:1;p:&n;good:p.{->$;->tag:true}==p;bad:p==((p.{->$;->tag:false}))";

#[test]
pub(crate) fn dispatch_reference_links_reject_late_identity_faults_and_group_cycles_atomically() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        assert_eq!(reports.slot_uses.len(), 2);
        let id = *checker.binaries.keys().last().unwrap();
        let (&dispatch, op) = checker.dispatch_ops.last_key_value().unwrap();
        let block = op.block;
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => {
                let types = &mut checker.binaries.get_mut(&id).unwrap().types;
                types.inputs = [BinaryClass::SharedScalar(ScalarKind::Bool); 2];
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.types = *types;
            }
            2 | 9 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                if fault == 2 {
                    slots[0].shape = Shape::SharedScalar(ScalarKind::Bool);
                } else {
                    slots[1].shape = Shape::SharedScalar(ScalarKind::Float { bits: 16 });
                }
            }
            3 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.input += 1;
            }
            4 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            5 => checker.points[checker.binaries[&id].inputs[1]].owner += 1,
            6 => {
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = [false; 2];
                op.operation = false;
                op.result = false;
            }
            7 => {
                let (&id, group) = checker.group_inputs.last_key_value().unwrap();
                let mut group = *group;
                group.input = id;
                checker.group_inputs.insert(id, group);
            }
            8 => {
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_reference_links_keep_missing_source_and_type_evidence_opaque() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.binaries.keys().last().unwrap();
        let (&dispatch, op) = checker.dispatch_ops.last_key_value().unwrap();
        let block = op.block;
        match fault {
            0 => {
                reports.results.remove(&block);
            }
            1 => {
                reports.effects.remove(&dispatch);
            }
            2 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            3 => {
                let types = &mut checker.binaries.get_mut(&id).unwrap().types;
                types.inputs = [BinaryClass::Reference(hir::ReferenceMode::Shared); 2];
                let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.types = *types;
            }
            4 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                slots[0].shape = Shape::Reference(hir::ReferenceMode::Shared);
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&Port::Projection { point: id, step: 1 });
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
pub(crate) fn dispatch_reference_links_preserve_independent_control_and_exact_limits() {
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.slot_uses.clone();
    for consumer in [false, true] {
        for source in [false, true] {
            for (&id, op) in &mut checker.binaries {
                op.control = consumer;
                let (_, Effect::Binary(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = consumer;
            }
            for (&id, op) in &mut checker.dispatch_ops {
                op.control = source;
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = source;
            }
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                expected
            );
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
