use super::{boundaries::SOURCE, *};

#[test]
pub(crate) fn shared_local_receiver_fields_drop_missing_links_and_reject_stale_graphs() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.keys().last().unwrap();
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        let (&local, row) = reports.shared_initializers.last_key_value().unwrap();
        let statement = row.init.statement;
        let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
        match fault {
            0 => {
                reports.shared_initializers.remove(&local);
            }
            1 => {
                reports.effects.remove(&statement);
            }
            2 => {
                let read = *checker
                    .local_reads
                    .iter()
                    .find(|(_, op)| op.local == local)
                    .unwrap()
                    .0;
                reports.effects.remove(&read);
            }
            3 => {
                reports.receivers.remove(&cell);
            }
            4 => {
                reports.effects.remove(&receiver);
            }
            5 => {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = false;
            }
            6 => {
                let source = reports.results[&slot.block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            7 => {
                let typed = *checker.typed_ops.keys().last().unwrap();
                reports.effects.remove(&typed);
            }
            8 => {
                checker.fields.get_mut(&id).unwrap().shared_primary = None;
                let (_, Effect::Field { shared_primary, .. }) =
                    reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *shared_primary = None;
            }
            9 => {
                checker.proofs.mutable.insert(local);
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
        assert!(!reports.field_results.contains_key(&Port::Normal(id)));
        if matches!(fault, 4 | 6) {
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert!(
                checker
                    .direct_sources(&reports, Span::default())
                    .expect_err(&format!("stale dispatch result, fault {fault}"))
                    .message
                    .contains("result-consumer identity")
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            continue;
        }
        reports.direct_sources = checker
            .direct_sources(&reports, Span::default())
            .unwrap_or_else(|error| panic!("fault {fault}: {error:?}"))
            .0;
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
