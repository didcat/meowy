use super::super::field_results::lookup::Lookup;
use super::*;

#[test]
pub(crate) fn receiver_sources_reject_overlapping_field_producers() {
    let (mut checker, reports) = checked("r:{->n:0};unused:r.n;v:{->3}.{->$}");
    let (&read, _) = checker
        .local_reads
        .iter()
        .find(|(_, read)| reports.receivers.contains_key(&read.local))
        .unwrap();
    checker
        .fields
        .insert(read, checker.fields.values().next().unwrap().clone());
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .scalar_block_source(&reports, read, 0, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .field_narrowing_source(
                &mut Lookup::new(&reports, MAX_EDGES),
                read,
                0,
                Span::default(),
                MAX_GROUPS
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn receiver_sources_requalify_stored_links_after_missing_or_changed_evidence() {
    for fault in 0..5 {
        let (mut checker, mut reports) = checked("v:{->3}.{->$};out:{->v}");
        let (&read, op) = checker
            .local_reads
            .iter()
            .find(|(_, read)| reports.receivers.contains_key(&read.local))
            .unwrap();
        let local = op.local;
        let dispatch = reports.receivers[&local].1;
        match fault {
            0 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                    panic!()
                };
                op.initialized = false;
            }
            1 => {
                reports.effects.remove(&read);
            }
            2 => {
                reports.receivers.remove(&local);
            }
            3 => reports.receivers.get_mut(&local).unwrap().0 += 1,
            4 => checker.points[read].parent = Some(read),
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn seeded_receiver_sources_reject_cycles_through_ordinary_initializer_jumps() {
    let (mut checker, mut reports) = checked("base:{->1};n:base;v:n.{copy:$;->copy}");
    let op = checker.dispatch_ops.values().next().unwrap();
    let input = op.input;
    let read = *checker
        .local_reads
        .keys()
        .find(|&&id| {
            checker.points[id].span == checker.points[input].span
                && checker.points[id].block == checker.points[input].block
        })
        .unwrap();
    let copy = checker.local_reads.last_key_value().unwrap().1.local;
    assert!(reports.initializers.contains_key(&copy));
    let op = checker.local_reads.get_mut(&read).unwrap();
    op.local = copy;
    op.storage = copy;
    let (_, Effect::Read { local, storage, .. }) = reports.effects.get_mut(&read).unwrap() else {
        panic!()
    };
    *local = copy;
    *storage = copy;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .grouped_consumer(&reports, input, 0, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(
        checker
            .field_narrowing_source(
                &mut Lookup::new(&reports, MAX_EDGES),
                input,
                0,
                Span::default(),
                MAX_GROUPS
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
