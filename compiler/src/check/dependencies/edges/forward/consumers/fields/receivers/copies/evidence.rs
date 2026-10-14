use super::{
    boundaries::{SOURCE, copy_read},
    *,
};

#[test]
pub(crate) fn shared_receiver_copy_fields_drop_missing_links_and_reject_stale_graphs() {
    for fault in 0..15 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let read = copy_read(&checker, &reports, id);
        let local = checker.local_reads[&read].local;
        let binding = reports.shared_initializers[&local].init;
        let copy = checker.local_reads[&copy_read(&checker, &reports, binding.statement)].local;
        let copied = reports.shared_initializers[&copy].init;
        let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
        let original = *checker
            .local_reads
            .iter()
            .find(|(id, op)| {
                op.local == cell
                    && checker.points[**id].site == checker.points[copied.statement].site
            })
            .unwrap()
            .0;
        match fault {
            0 => {
                reports.shared_initializers.remove(&local);
            }
            1 => {
                reports.effects.remove(&binding.statement);
            }
            2 => {
                reports.effects.remove(&read);
            }
            3 => {
                reports.shared_initializers.remove(&copy);
            }
            4 => {
                reports.effects.remove(&copied.statement);
            }
            5 => {
                reports.effects.remove(&original);
            }
            6 => {
                reports.receivers.remove(&cell);
            }
            7 => {
                reports.effects.remove(&receiver);
            }
            8 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = false;
            }
            9 => {
                let source = reports.results[&slot.block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            10 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            11 => {
                checker.fields.get_mut(&id).unwrap().shared_primary = None;
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = None;
            }
            12 => {
                checker.proofs.mutable.insert(copy);
            }
            13 => {
                let op = checker.local_reads.get_mut(&original).unwrap();
                op.normal = false;
                op.edges.truncate(1);
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&original).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            14 => {
                reports
                    .shared_initializers
                    .get_mut(&copy)
                    .unwrap()
                    .init
                    .input = None;
                let op = checker.operations.get_mut(&copied.statement).unwrap();
                op.input = None;
                op.edges = vec![Edge::new(
                    Port::Operation(copied.statement),
                    Port::Normal(copied.statement),
                    Route::Next,
                )];
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&copied.statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&Port::Operation(id));
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        assert!(
            checker
                .field_results(&reports, Span::default())
                .expect_err(&format!("stale fields, fault {fault}"))
                .message
                .contains("identity")
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .expect_err(&format!("stale walk, fault {fault}"))
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        reports.slot_uses = expected;
        reports.field_results = checker.field_results(&reports, Span::default()).unwrap().0;
        assert_eq!(reports.field_results.len(), 1);
        if matches!(fault, 7 | 9) {
            assert!(
                checker
                    .direct_sources(&reports, Span::default())
                    .expect_err("stale upstream result")
                    .message
                    .contains("result-consumer identity")
            );
            continue;
        }
        reports.direct_sources = checker.direct_sources(&reports, Span::default()).unwrap().0;
        assert!(
            !reports
                .direct_sources
                .values()
                .any(|(_, direct)| direct.source.is_some_and(|source| source.field == id))
        );
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap();
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
