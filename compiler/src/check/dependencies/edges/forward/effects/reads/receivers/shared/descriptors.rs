use super::*;
use crate::check::dependencies::edges::forward::effects::tests::checked;

#[test]
pub(crate) fn shared_receiver_descriptors_retain_exact_kinds_counts_and_independent_stages() {
    for binding in ["n:1", "n<uint8>:1", "n:true", "n:1.5", "n:\"text\""] {
        for fields in [1, 2] {
            let suffix = if fields == 2 { ";->n:3" } else { "" };
            let source = format!("{binding};p:&n;r:{{->p;->tag:true{suffix}}};out:r.{{->$.tag}}");
            let (mut checker, mut reports) = checked(&source, false);
            let (&local, &(owner, dispatch, primary)) =
                reports.receivers.first_key_value().unwrap();
            let primary = primary.unwrap();
            let input = checker.dispatch_ops[&dispatch].input;
            let id = *checker
                .local_reads
                .iter()
                .find(|(_, op)| op.local == local)
                .unwrap()
                .0;
            let edges = checker.local_reads[&id].edges.clone();
            for initialized in [false, true] {
                for result in [false, true] {
                    let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap()
                    else {
                        panic!()
                    };
                    op.initialized = initialized;
                    op.result = result;
                    for normal in [false, true] {
                        let read = checker.local_reads.get_mut(&id).unwrap();
                        read.normal = normal;
                        read.edges = edges[..1 + usize::from(normal)].to_vec();
                        let (_, Effect::Read { normal: seen, .. }) =
                            reports.effects.get_mut(&id).unwrap()
                        else {
                            panic!()
                        };
                        *seen = normal;
                        let source = checker
                            .shared_receiver_source(&reports, id, owner, primary, Span::default())
                            .unwrap();
                        assert_eq!(
                            source,
                            (initialized && normal)
                                .then_some((input, SharedRecord { primary, fields }))
                        );
                    }
                }
            }
        }
    }
}

#[test]
pub(crate) fn shared_receiver_descriptors_reject_stale_and_malformed_headers_atomically() {
    for fault in 0..6 {
        let (mut checker, mut reports) =
            checked("n:1;p:&n;r:{->p;->tag:true};out:r.{->$.tag}", false);
        let (&local, &(_, dispatch, primary)) = reports.receivers.first_key_value().unwrap();
        let mut primary = primary.unwrap();
        let id = *checker
            .local_reads
            .iter()
            .find(|(_, op)| op.local == local)
            .unwrap()
            .0;
        let op = checker.dispatch_ops.get_mut(&dispatch).unwrap();
        let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&dispatch).unwrap() else {
            panic!()
        };
        match fault {
            0 => observed.receiver = Shape::Record { fields: 2 },
            1 => op.receiver = Shape::Record { fields: 2 },
            2 | 3 => {
                op.receiver = if fault == 2 {
                    Shape::Scalar(primary)
                } else {
                    Shape::Record { fields: usize::MAX }
                };
                observed.receiver = op.receiver;
            }
            4 => primary = ScalarKind::Bool,
            5 => {
                primary = ScalarKind::Int {
                    bits: 7,
                    signed: true,
                };
                op.shared_primary = Some(primary);
                reports.receivers.get_mut(&local).unwrap().2 = Some(primary);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .shared_receiver_source(&reports, id, 0, primary, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_receiver_descriptors_share_exact_work_without_payload_allocation() {
    let (mut checker, mut reports) = checked(
        "n:1;p:&n;r:{->p;->tag:true};out:r.{nested:{->$.tag}}",
        false,
    );
    let (&local, &(_, _, primary)) = reports.receivers.first_key_value().unwrap();
    let primary = primary.unwrap();
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, op)| op.local == local)
        .unwrap()
        .0;
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert!(
        checker
            .shared_receiver_source(&reports, id, 0, primary, Span::default())
            .unwrap()
            .is_some()
    );
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.shared_receiver_source(&reports, id, 0, primary, Span::default());
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
