use super::*;
use crate::check::dependencies::edges::forward::effects::tests::checked;

#[test]
pub(crate) fn receiver_inputs_keep_nested_scopes_owners_and_ordinary_initializer_exclusion() {
    for source in [
        "v:3.{nested:{copy:$;->$};inner:$.{->$};->$}",
        "f<int32>:(n<int32>){->n.{inner:{->$};->$}}",
        "d:@\"debug\";v:3.{copy:$;d.panic(\"stop\")}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let reads: Vec<_> = checker
            .local_reads
            .iter()
            .filter_map(|(&id, read)| {
                reports
                    .receivers
                    .get(&read.local)
                    .map(|&(owner, dispatch)| (id, owner, dispatch))
            })
            .collect();
        assert!(!reads.is_empty());
        for (id, owner, dispatch) in reads {
            let input = checker.dispatch_ops[&dispatch].input;
            assert_eq!(
                checker
                    .read_initializer_input(&reports, id, owner, Span::default())
                    .unwrap(),
                None
            );
            assert_eq!(
                checker
                    .read_receiver_input(&reports, id, owner, Span::default())
                    .unwrap(),
                Some(input)
            );
        }
    }
}

#[test]
pub(crate) fn receiver_inputs_require_initialization_independently_of_result_observation() {
    let (mut checker, mut reports) = checked("v:3.{->$}", false);
    let (&id, read) = checker.local_reads.first_key_value().unwrap();
    let dispatch = reports.receivers[&read.local].1;
    let input = checker.dispatch_ops[&dispatch].input;
    for initialized in [true, false] {
        for result in [true, false] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            assert_eq!(
                checker
                    .read_receiver_input(&reports, id, 0, Span::default())
                    .unwrap(),
                initialized.then_some(input)
            );
        }
    }
    reports.effects.remove(&dispatch);
    assert_eq!(
        checker
            .read_receiver_input(&reports, id, 0, Span::default())
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn receiver_inputs_preserve_reference_aggregate_and_unobserved_leaves() {
    for source in [
        "n:1;v:(&n).{copy:$;->*$}",
        "v:{->n:1}.{copy:$;->$.n}",
        "v:[1,2].{copy:$;->3}",
        "n<int32><null>:1;v:n.{copy:$;->3}",
        "d:@\"debug\";v:3.{d.panic(\"stop\");copy:$}",
    ] {
        let (mut checker, reports) = checked(source, false);
        let reads: Vec<_> = checker
            .local_reads
            .iter()
            .filter_map(|(&id, read)| {
                reports
                    .receivers
                    .contains_key(&read.local)
                    .then_some((id, read.owner))
            })
            .collect();
        assert!(!reads.is_empty());
        for (id, owner) in reads {
            assert_eq!(
                checker
                    .read_receiver_input(&reports, id, owner, Span::default())
                    .unwrap(),
                None,
                "{source}"
            );
        }
    }
}

#[test]
pub(crate) fn receiver_inputs_reject_conflicting_reads_producers_and_parent_chains() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked("v:3.{nested:{->$};->$}", false);
        let (&id, read) = checker.local_reads.first_key_value().unwrap();
        let local = read.local;
        let dispatch = reports.receivers[&local].1;
        let (input, block) = (
            checker.dispatch_ops[&dispatch].input,
            checker.dispatch_ops[&dispatch].block,
        );
        match fault {
            0 => reports.effects.get_mut(&id).unwrap().0 += 1,
            1 => checker.local_reads.get_mut(&id).unwrap().storage = usize::MAX,
            2 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            3 => reports.receivers.get_mut(&local).unwrap().1 = id,
            4 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.receiver = Shape::Other;
            }
            5 => checker.points[input].span.end = usize::MAX,
            6 => checker.points[id].parent = Some(id),
            7 => checker.points[id].block = Some(block),
            8 => checker
                .dispatch_ops
                .get_mut(&dispatch)
                .unwrap()
                .edges
                .clear(),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .read_receiver_input(&reports, id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
