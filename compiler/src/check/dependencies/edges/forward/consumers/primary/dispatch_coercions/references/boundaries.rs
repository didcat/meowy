use super::*;
use crate::check::dependencies::ScalarKind;

pub(super) const SOURCE: &str = "n:1;p:&n;good<&int32>:(p.{->$;->tag:true}~<{-><&int32>;tag<boolean>}>);bad<&int32><null>:(((p.{->$;->tag:false}))~<{-><&int32>;tag<boolean>}>)";

#[test]
pub(crate) fn shared_coercion_links_reject_late_type_owner_and_cycle_faults_atomically() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked(SOURCE);
        assert_eq!(reports.slot_uses.len(), 2);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let block = reports.slot_uses[&Port::Projection { point: id, step: 0 }]
            .1
            .block;
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
            6 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            7 => reports.effects.get_mut(&id).unwrap().0 += 1,
            8 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.input = usize::MAX;
            }
            9 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            10 => {
                let id = checker.typed_ops.values().last().unwrap().input;
                let mut group = checker.group_inputs[&id];
                group.input = id;
                checker.group_inputs.insert(id, group);
            }
            11 => {
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .expect_err(&format!("fault {fault}"))
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_coercion_links_keep_missing_evidence_opaque_without_dropping_earlier_links() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
        let dispatch = reports.results[&block].1.dispatch.unwrap();
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
                let source = Some(Shape::Reference(hir::ReferenceMode::Shared));
                checker.coercions.get_mut(&id).unwrap().source = source;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.source = source;
            }
            4 => {
                let id = *checker.typed_ops.keys().last().unwrap();
                reports.effects.remove(&id);
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&port);
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
pub(crate) fn shared_coercion_links_keep_control_and_exact_map_work_limits_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.slot_uses.clone();
    for consumer in [false, true] {
        for producer in [false, true] {
            for (&id, op) in &mut checker.coercions {
                op.control = consumer;
                if let Some((_, Effect::Coercion(observed))) = reports.effects.get_mut(&id) {
                    observed.control = consumer;
                }
            }
            for (&id, op) in &mut checker.dispatch_ops {
                op.control = producer;
                let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                observed.control = producer;
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
