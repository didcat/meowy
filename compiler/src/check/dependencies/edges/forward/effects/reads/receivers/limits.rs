use super::*;
use crate::check::dependencies::{edges::forward::effects::tests::checked, points::Point};

#[test]
pub(crate) fn receiver_inputs_reject_crossed_dispatch_bindings_and_scope_owners() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked("v:3.{inner:4.{nested:{->$}};->$}", false);
        let (&outer, op) = checker.dispatch_ops.first_key_value().unwrap();
        let outer_local = op.local;
        let (&inner, op) = checker.dispatch_ops.last_key_value().unwrap();
        let inner_block = op.block;
        let (&id, _) = checker
            .local_reads
            .iter()
            .find(|(_, read)| read.local == op.local)
            .unwrap();
        let block = checker.points[id].block.unwrap();
        let parent = checker.bodies[&block].parent.unwrap();
        match fault {
            0 => {
                let read = checker.local_reads.get_mut(&id).unwrap();
                read.local = outer_local;
                read.storage = outer_local;
                let (_, Effect::Read { local, storage, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *local = outer_local;
                *storage = outer_local;
            }
            1 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            2 => checker.points[parent].complete = false,
            3 => checker.points[parent].parent = Some(outer),
            4 => checker.points[parent].block = Some(block),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .read_receiver_input(&reports, id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}, {inner}, {inner_block}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_inputs_share_exact_scope_hop_and_work_limits() {
    for (source, hops) in [("v:3.{->$}", 3), ("v:3.{nested:{->$}}", 5)] {
        let (mut checker, reports) = checked(source, false);
        let (&id, read) = checker.local_reads.first_key_value().unwrap();
        let dispatch = reports.receivers[&read.local].1;
        let op = &checker.dispatch_ops[&dispatch];
        let (input, block) = (op.input, op.block);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        checker
            .receiver_scope(id, block, 0, Span::default(), hops)
            .unwrap();
        assert!(
            checker
                .receiver_scope(id, block, 0, Span::default(), hops - 1)
                .unwrap_err()
                .message
                .contains("budget")
        );
        let start = checker.flow.work;
        assert_eq!(
            checker
                .read_receiver_input(&reports, id, 0, Span::default())
                .unwrap(),
            Some(input)
        );
        let work = checker.flow.work - start;
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.read_receiver_input(&reports, id, 0, Span::default());
            assert_eq!(result.is_ok(), short == 0);
            if short == 1 {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_receiver_scope_is_iterative_and_rejects_long_cycles() {
    let (mut checker, reports) = checked("v:3.{->$}", false);
    let (&id, read) = checker.local_reads.first_key_value().unwrap();
    let dispatch = reports.receivers[&read.local].1;
    let block = checker.dispatch_ops[&dispatch].block;
    let (span, site) = (checker.points[id].span, checker.points[id].site);
    let mut parent = checker.points[id].parent;
    let start = checker.points.len();
    for _ in 0..2048 {
        let next = checker.points.len();
        checker.points.push(Point {
            kind: PointKind::Expr,
            owner: 0,
            block: Some(block),
            site,
            parent,
            span,
            complete: true,
        });
        parent = Some(next);
    }
    checker.points[id].parent = parent;
    checker
        .receiver_scope(id, block, 0, Span::default(), 2051)
        .unwrap();
    assert!(
        checker
            .receiver_scope(id, block, 0, Span::default(), 2050)
            .unwrap_err()
            .message
            .contains("budget")
    );
    checker.points[start].parent = parent;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .receiver_scope(id, block, 0, Span::default(), MAX_GROUPS)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
