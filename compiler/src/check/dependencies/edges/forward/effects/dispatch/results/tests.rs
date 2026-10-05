use super::*;
use crate::check::dependencies::edges::forward::effects::tests::checked;

#[test]
pub(crate) fn dispatch_result_bodies_preserve_owners_layouts_and_independent_observations() {
    let (mut checker, mut reports) = checked(
        "v:3.{->$};empty:3.{};row:3.{->x:$};f<int32>:(){->4.{->$}}",
        false,
    );
    let ids: Vec<_> = checker
        .dispatch_ops
        .iter()
        .map(|(&id, op)| (id, op.owner, op.block))
        .collect();
    assert!(ids.iter().any(|(_, owner, _)| *owner != 0));
    for (id, owner, block) in ids {
        for result in [true, false] {
            for initialized in [true, false] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.result = result;
                op.initialized = initialized;
                let before = format!("{reports:?}{:?}", checker.edge_counts());
                assert_eq!(
                    checker
                        .dispatch_result_body(&reports, id, owner, Span::default())
                        .unwrap(),
                    result.then_some(block)
                );
                assert_eq!(
                    checker.sequences[&SequenceSource::Block(block)].items[0],
                    None
                );
                assert!(!reports.blocks.contains_key(&block));
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
        }
        reports.effects.remove(&id);
        assert_eq!(
            checker
                .dispatch_result_body(&reports, id, owner, Span::default())
                .unwrap(),
            None
        );
    }
    for tail in ["stop().{}", "3.{stop()}"] {
        let (mut checker, reports) = checked(
            &format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail}"),
            false,
        );
        let id = *checker.dispatch_ops.keys().next().unwrap();
        assert_eq!(
            checker
                .dispatch_result_body(&reports, id, 0, Span::default())
                .unwrap(),
            None
        );
    }
}

#[test]
pub(crate) fn dispatch_result_bodies_reject_corrupt_reports_middle_statements_and_layouts() {
    for fault in 0..19 {
        let (mut checker, mut reports) = checked("v:3.{x:1;y:2;->$}", false);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let block = op.block;
        let key = SequenceSource::Block(block);
        let middle = checker.sequences[&key].items[2].unwrap();
        let (owner, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.input = id,
            2 => op.local = usize::MAX,
            3 => op.block = usize::MAX,
            4 => op.normal = false,
            5 => op.control = !op.control,
            6 => checker.bodies.get_mut(&block).unwrap().completion.normal = false,
            7 => checker.bodies.get_mut(&block).unwrap().span.end = 0,
            8 => checker.points[middle].parent = None,
            9 => checker.points[middle].site = None,
            10 => checker.points[middle].complete = false,
            11 => checker.sequences.get_mut(&key).unwrap().items.swap(1, 2),
            12 => checker.sequences.get_mut(&key).unwrap().edges.clear(),
            13 => {
                checker.bodies.get_mut(&block).unwrap().layout =
                    crate::check::dependencies::bodies::Layout::Unknown
            }
            14 => {
                checker.dispatch_ops.remove(&id);
            }
            15 => {
                reports.index.operations.remove(&id);
            }
            16 => {
                checker.points[id].block = None;
                checker.points[op.input].block = None;
            }
            17 => {
                checker.points[id].span.start = usize::MAX;
                checker.dispatch_ops.get_mut(&id).unwrap().span = checker.points[id].span;
            }
            18 => {
                let sequence = checker.sequences.get_mut(&key).unwrap();
                sequence.items[2] = sequence.items[1];
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .dispatch_result_body(&reports, id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
