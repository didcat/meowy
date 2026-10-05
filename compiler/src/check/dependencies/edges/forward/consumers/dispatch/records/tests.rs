use super::*;
use crate::check::dependencies::edges::forward::consumers::tests::checked;
use std::collections::BTreeSet;

#[test]
pub(crate) fn record_dispatch_bodies_keep_distinct_origins_wrappers_and_owners() {
    for source in [
        "a:3.{->n:$};x:a.n;f<int32>:(){r:4.{->n:$};->((r)).n}",
        "a<{n<int32>}>:((3.{->n:$})~<{n<int32>}>);x:a.n",
        "x:(3.{->{->n:$}}).n",
    ] {
        let (mut checker, reports) = checked(source);
        let inputs: Vec<_> = checker
            .fields
            .values()
            .map(|op| (op.input, op.owner))
            .collect();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut owners = BTreeSet::new();
        for (input, owner) in inputs {
            let point = checker
                .dispatch_consumer(&reports, input, owner, Span::default())
                .unwrap()
                .unwrap();
            let block = checker.dispatch_ops[&point].block;
            assert_eq!(
                checker
                    .record_dispatch_body(&reports, input, owner, Span::default())
                    .unwrap(),
                Some(block)
            );
            assert_eq!(
                checker
                    .slot_block(&reports, input, owner, Span::default())
                    .unwrap(),
                None
            );
            assert_eq!(
                checker
                    .scalar_dispatch_source(&reports, input, owner, Span::default())
                    .unwrap(),
                None
            );
            assert_eq!(reports.results[&block].1.dispatch, Some(point));
            assert!(
                !reports.blocks.contains_key(&block) && !reports.consumers.contains_key(&point)
            );
            owners.insert(owner);
        }
        assert!(owners.contains(&0));
        if source.contains("f<int32>") {
            assert!(owners.contains(&1));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn record_dispatch_bodies_require_results_independently_of_initialization() {
    let (mut checker, mut reports) = checked("v:3.{->n:$}");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let block = op.block;
    for initialized in [true, false] {
        for result in [true, false] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            assert_eq!(
                checker
                    .record_dispatch_body(&reports, id, 0, Span::default())
                    .unwrap(),
                result.then_some(block)
            );
        }
    }
    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.result = true;
    reports.results.remove(&block);
    assert_eq!(
        checker
            .record_dispatch_body(&reports, id, 0, Span::default())
            .unwrap(),
        None
    );
    for source in [
        "v:3.{}",
        "v:3.{->$}",
        "n:1;v:(&n).{->$}",
        "d:@\"debug\";v:3.{->n:$;d.panic(\"stop\")}",
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};v:stop().{->n:1}",
    ] {
        let (mut checker, reports) = checked(source);
        let id = *checker.dispatch_ops.keys().next().unwrap();
        assert_eq!(
            checker
                .record_dispatch_body(&reports, id, 0, Span::default())
                .unwrap(),
            None,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn record_dispatch_bodies_reject_corrupt_origins_owners_and_layouts() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("v:3.{->n:$}");
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let block = op.block;
        match fault {
            0 => reports.results.get_mut(&block).unwrap().0 += 1,
            1 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            2 => reports.results.get_mut(&block).unwrap().1.dispatch = Some(usize::MAX),
            3 => reports.results.get_mut(&block).unwrap().1.consumer = Some(id),
            4 => reports.results.get_mut(&block).unwrap().1.slots = None,
            5 => reports
                .results
                .get_mut(&block)
                .unwrap()
                .1
                .slots
                .as_mut()
                .unwrap()
                .clear(),
            6 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            7 => checker.bodies.get_mut(&block).unwrap().parent = None,
            8 => checker.bodies.get_mut(&block).unwrap().completion.normal = false,
            9 => checker.bodies.get_mut(&block).unwrap().layout = Layout::Unknown,
            10 => {
                reports
                    .blocks
                    .insert(block, *reports.blocks.values().next().unwrap());
            }
            11 => {
                reports.consumers.insert(id, (0, block));
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .record_dispatch_body(&reports, id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
