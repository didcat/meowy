use super::*;
use crate::check::dependencies::edges::forward::effects::tests::checked;

#[test]
pub(crate) fn record_receiver_inputs_reuse_checked_eligibility_across_scopes_and_owners() {
    let source = "r:{->n:3};v:r.{nested:{copy:$;->$.n};inner:$.{->$.n};->$.n};f<int32>:(){r:{->n:4};->r.{->$.n}}";
    let (mut checker, reports) = checked(source, false);
    assert!(checker.locals.is_empty());
    let reads: Vec<_> = checker
        .local_reads
        .iter()
        .filter_map(|(&id, read)| {
            reports
                .receivers
                .get(&read.local)
                .map(|&(owner, dispatch, _)| (id, owner, dispatch))
        })
        .collect();
    assert!(reads.iter().any(|(_, owner, _)| *owner != 0));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (id, owner, dispatch) in reads {
        let op = &checker.dispatch_ops[&dispatch];
        let (input, local) = (op.input, op.local);
        assert!(reports.eligible.contains(&local));
        assert!(!reports.initializers.contains_key(&local));
        assert_eq!(
            checker
                .receiver_value_input(&reports, id, owner, Span::default(), true)
                .unwrap(),
            Some(input)
        );
        assert_eq!(
            checker
                .read_receiver_input(&reports, id, owner, Span::default())
                .unwrap(),
            None
        );
        assert_eq!(
            checker
                .read_initializer_input(&reports, id, owner, Span::default())
                .unwrap(),
            None
        );
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn record_receiver_inputs_keep_ineligible_and_unobserved_records_opaque() {
    for source in [
        "r:{->n:=1};v:r.{copy:$;->$.n}",
        "base:1;r:{->n:2;->p:&base};v:r.{copy:$;->$.n}",
        "base:1;r:{->n:2;->inner:{->p:&base}};v:r.{copy:$;->$.n}",
        "r:{->n:2;->inner:{->n:=3}};v:r.{copy:$;->$.n}",
        "r:{->n:2;->h:@\"memory\".heap};v:r.{copy:$;->$.n}",
        "r:{->n:1};v:(&r).{copy:$;->$.n}",
        "v:3.{copy:$;->$}",
        "v:[1,2].{copy:$;->3}",
        "n<int32><null>:1;v:n.{copy:$;->3}",
        "d:@\"debug\";r:{->n:1};v:r.{d.panic(\"stop\");copy:$}",
    ] {
        let (mut checker, reports) = checked(source, false);
        let reads: Vec<_> = checker
            .local_reads
            .iter()
            .filter(|(_, read)| reports.receivers.contains_key(&read.local))
            .map(|(&id, read)| (id, read.owner))
            .collect();
        assert!(!reads.is_empty(), "{source}");
        for (id, owner) in reads {
            assert_eq!(
                checker
                    .receiver_value_input(&reports, id, owner, Span::default(), true)
                    .unwrap(),
                None,
                "{source}"
            );
        }
    }
}

#[test]
pub(crate) fn record_receiver_inputs_require_initialization_and_eligibility_without_result_visits()
{
    let (mut checker, mut reports) = checked("r:{->n:1};v:r.{->$.n}", false);
    let (&dispatch, op) = checker.dispatch_ops.first_key_value().unwrap();
    let (local, input) = (op.local, op.input);
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local == local)
        .unwrap()
        .0;
    for initialized in [false, true] {
        for result in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            assert_eq!(
                checker
                    .receiver_value_input(&reports, id, 0, Span::default(), true)
                    .unwrap(),
                initialized.then_some(input)
            );
        }
    }
    reports.eligible.remove(&local);
    assert_eq!(
        checker
            .receiver_value_input(&reports, id, 0, Span::default(), true)
            .unwrap(),
        None
    );
    reports.eligible.insert(local);
    checker.proofs.mutable.insert(local);
    assert_eq!(
        checker
            .receiver_value_input(&reports, id, 0, Span::default(), true)
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn record_receiver_inputs_reject_stale_headers_scopes_and_exact_work_exhaustion() {
    for fault in 0..6 {
        let (mut checker, mut reports) = checked("r:{->n:1};v:r.{nested:{->$.n}}", false);
        let (&dispatch, op) = checker.dispatch_ops.first_key_value().unwrap();
        let (local, input) = (op.local, op.input);
        let id = *checker
            .local_reads
            .iter()
            .find(|(_, read)| read.local == local)
            .unwrap()
            .0;
        match fault {
            0 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            1 => reports.receivers.get_mut(&local).unwrap().1 = id,
            2 => checker.dispatch_ops.get_mut(&dispatch).unwrap().receiver = Shape::Other,
            3 => checker.points[input].span.end = usize::MAX,
            4 => checker.points[id].parent = Some(id),
            5 => {
                checker
                    .dispatch_ops
                    .get_mut(&dispatch)
                    .unwrap()
                    .edges
                    .clear();
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .receiver_value_input(&reports, id, 0, Span::default(), true)
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, mut reports) = checked("r:{->n:1};v:r.{->$.n}", false);
    let op = checker.dispatch_ops.values().next().unwrap();
    let (local, input) = (op.local, op.input);
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local == local)
        .unwrap()
        .0;
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .receiver_value_input(&reports, id, 0, Span::default(), true)
            .unwrap(),
        Some(input)
    );
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.receiver_value_input(&reports, id, 0, Span::default(), true);
        if short == 0 {
            assert_eq!(result.unwrap(), Some(input));
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
