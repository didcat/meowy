use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn dispatch_unary_primaries_keep_exact_ports_types_and_independent_owners() {
    for source in [
        "x:-((3.{->$;->tag:true}));f<null>:(){-((4.{->$;->tag:false}))}",
        "n<int8>:3;x:-(n.{->$;->tag:true})",
        "n<float64>:1.5;x:-(n.{->$;->tag:true})",
        "x:!(true.{->$;->tag:true})",
    ] {
        let (mut checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), checker.unaries.len());
        for (&id, op) in &checker.unaries {
            assert!(op.primary);
            let (owner, slot) = reports.slot_uses[&Port::Projection { point: id, step: 0 }];
            assert_eq!((owner, slot.index), (op.owner, 0));
            let (_, Effect::Unary(unary)) = &reports.effects[&id] else {
                panic!()
            };
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(slots[0].shape, Shape::Scalar(unary.ty));
            let row = &reports.results[&slot.block].1;
            assert!(row.dispatch.is_some() && row.consumer.is_none());
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_unary_primaries_require_projection_and_dispatch_result_independently() {
    let (mut checker, mut reports) = checked("x:-(3.{->$;->tag:true})");
    let id = *checker.unaries.keys().next().unwrap();
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    for initialized in [false, true] {
        for completed in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = completed;
            for (projected, operation, result) in [
                (true, false, false),
                (true, true, true),
                (false, true, false),
                (false, false, true),
            ] {
                let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = projected;
                op.operation = operation;
                op.result = result;
                let uses = checker.slot_uses(&reports, Span::default()).unwrap();
                assert_eq!(
                    uses,
                    if projected && completed {
                        expected.clone()
                    } else {
                        Uses::new()
                    }
                );
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn dispatch_unary_primaries_keep_other_producers_and_arithmetic_results_opaque() {
    for source in [
        "r:3.{->$;->tag:true};p:&r;x:-(*p)",
        "f<{-><int32>;tag<boolean>}>:(){->3;->tag:true};x:-f()",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (_, reports) = checked("x:-(3.{->$;->tag:true});out:{->x}");
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .direct_sources
            .values()
            .all(|(_, direct)| direct.source.is_none()
                && direct.block.is_none()
                && direct.dispatch.is_none())
    );
}

#[test]
pub(crate) fn dispatch_unary_primaries_reject_consumer_and_source_conflicts_atomically() {
    for fault in 0..6 {
        let (mut checker, mut reports) = checked("x:-(3.{->$;->tag:true})");
        let id = *checker.unaries.keys().next().unwrap();
        let block = checker.dispatch_ops.values().next().unwrap().block;
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => {
                let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.input = usize::MAX;
            }
            2 => {
                checker.unaries.get_mut(&id).unwrap().ty = hir::Type::Int {
                    bits: 8,
                    signed: true,
                };
                let (_, Effect::Unary(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.ty = ScalarKind::Int {
                    bits: 8,
                    signed: true,
                };
            }
            3 => checker.unaries.get_mut(&id).unwrap().edges.clear(),
            4 => reports.results.get_mut(&block).unwrap().1.consumer = Some(id),
            5 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                slots[0].shape = Shape::Scalar(ScalarKind::Bool);
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
pub(crate) fn dispatch_unary_primaries_share_exact_map_and_work_limits_without_payload() {
    let (mut checker, mut reports) = checked("a:-(3.{->$;->tag:true});b:-(4.{->$;->tag:false})");
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + expected.len();
    reports.parts = 0;
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
