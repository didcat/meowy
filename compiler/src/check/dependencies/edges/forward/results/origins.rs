use super::{tests::checked, *};
use crate::check::dependencies::edges::forward::effects::Effect;

#[test]
pub(crate) fn result_origins_qualify_dispatches_without_ordinary_block_consumers() {
    let (mut checker, reports) =
        checked("v:3.{->$};empty:3.{};row:3.{->n:$};f<null>:(){v:4.{->$}}");
    let ids: Vec<_> = checker
        .dispatch_ops
        .iter()
        .map(|(&id, op)| (id, op.owner, op.block))
        .collect();
    assert!(ids.iter().any(|(_, owner, _)| *owner != 0));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (id, owner, block) in ids {
        let Layout::Slots(layout) = &checker.bodies[&block].layout else {
            panic!()
        };
        let row = Observed {
            consumer: None,
            dispatch: Some(id),
            slots: Some(vec![Sources::Unknown; layout.len()]),
        };
        checker
            .validate_result_report(&reports, block, owner, &row, Span::default())
            .unwrap();
        assert!(!reports.blocks.contains_key(&block));
        assert!(!reports.consumers.contains_key(&id));
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn result_origins_reject_ambiguous_wrong_and_unobserved_dispatches() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("a:3.{->$};b:4.{->$}");
        let (&id, op) = checker.dispatch_ops.last_key_value().unwrap();
        let block = op.block;
        let mut row = Observed {
            consumer: None,
            dispatch: Some(id),
            slots: Some(vec![Sources::Unknown]),
        };
        match fault {
            0 => row.consumer = Some(id),
            1 => row.dispatch = None,
            2 => row.dispatch = Some(usize::MAX),
            3 => row.dispatch = checker.dispatch_ops.keys().next().copied(),
            4 => row.slots = None,
            5 => row.slots.as_mut().unwrap().clear(),
            6 | 7 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                if fault == 6 {
                    op.result = false;
                } else {
                    op.normal = false;
                }
            }
            8 => {
                reports
                    .blocks
                    .insert(block, *reports.blocks.values().next().unwrap());
            }
            9 => reports.effects.get_mut(&id).unwrap().0 += 1,
            10 => {
                checker.proofs.dispatches.remove(&block);
            }
            11 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .validate_result_report(&reports, block, 0, &row, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
