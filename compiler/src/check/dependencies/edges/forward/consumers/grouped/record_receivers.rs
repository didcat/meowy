use super::{super::tests::checked, *};

#[test]
pub(crate) fn record_receiver_paths_share_hop_limits_across_both_sides_of_the_receiver_jump() {
    let mut base = None;
    for (outer, inner) in [(0, 0), (8, 0), (0, 8), (8, 8)] {
        let source = format!(
            "r:{{->n:1}};out:{}r{}.{{->{}${}.n}}",
            "(".repeat(outer),
            ")".repeat(outer),
            "(".repeat(inner),
            ")".repeat(inner)
        );
        let (mut checker, reports) = checked(&source);
        let (&id, op) = checker.fields.first_key_value().unwrap();
        let input = op.input;
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let expected = reports.results[&slot.block].1.consumer;
        assert!(expected.is_some());
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut minimum = None;
        for hops in 0..64 {
            match checker.grouped_source_limited(
                &reports,
                input,
                0,
                Span::default(),
                hops,
                Target::Receiver,
            ) {
                Ok(actual) => {
                    assert_eq!(actual, expected);
                    minimum = Some(hops);
                    break;
                }
                Err(error) => assert!(error.message.contains("budget")),
            }
        }
        let hops = minimum.unwrap();
        assert_eq!(hops, *base.get_or_insert(hops) + outer + inner);
        let start = checker.flow.work;
        assert_eq!(
            checker
                .grouped_source_limited(&reports, input, 0, Span::default(), hops, Target::Receiver)
                .unwrap(),
            expected
        );
        let work = checker.flow.work - start;
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.grouped_source_limited(
                &reports,
                input,
                0,
                Span::default(),
                hops,
                Target::Receiver,
            );
            if short == 0 {
                assert_eq!(result.unwrap(), expected);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_record_receiver_paths_reject_cycles_through_receiver_inputs_and_initializers()
{
    let (mut checker, mut reports) = checked("r:{->n:1};out:r.{alias:(($));->alias.n}");
    let alias = checker.local_reads.values().last().unwrap().local;
    let id = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local != alias && !reports.receivers.contains_key(&read.local))
        .unwrap()
        .0;
    let read = checker.local_reads.get_mut(&id).unwrap();
    read.local = alias;
    read.storage = alias;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *local = alias;
    *storage = alias;
    let input = checker.fields.values().next().unwrap().input;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .record_receiver_consumer(&reports, input, 0, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .direct_graph(&reports, Span::default(), MAX_EDGES)
            .err()
            .unwrap()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn record_receiver_paths_require_a_record_hop_and_leave_other_targets_unchanged() {
    for source in [
        "r:{->n:1};alias:((r));out:alias.n",
        "r:3.{->n:$};alias:((r));out:alias.n",
    ] {
        let (mut checker, reports) = checked(source);
        let input = checker.fields.values().next().unwrap().input;
        assert_eq!(
            checker
                .record_receiver_consumer(&reports, input, 0, Span::default())
                .unwrap(),
            None
        );
        assert_eq!(reports.slot_uses.len(), 1);
    }
    let (mut checker, reports) = checked("r:{->n:1};out:r.{->$.n}");
    let input = checker.fields.values().next().unwrap().input;
    assert_eq!(
        checker
            .grouped_consumer(&reports, input, 0, Span::default())
            .unwrap(),
        None
    );
    assert_eq!(
        checker
            .dispatch_consumer(&reports, input, 0, Span::default())
            .unwrap(),
        None
    );
    assert!(
        checker
            .record_receiver_consumer(&reports, input, 0, Span::default())
            .unwrap()
            .is_some()
    );
}
