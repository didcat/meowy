use super::{boundaries::SOURCE, *};

#[test]
pub(crate) fn receiver_copy_compositions_drop_missing_links_and_require_rebuilt_graphs() {
    for fault in 0..17 {
        let (mut checker, mut reports) = checked(SOURCE);
        let (&id, op) = checker
            .emissions
            .iter()
            .rev()
            .find(|(_, op)| op.composed.is_some())
            .unwrap();
        let ports: BTreeSet<_> = op
            .targets
            .iter()
            .map(|target| Port::Emission(target.id))
            .collect();
        let keys: Vec<_> = reports
            .candidate_inputs
            .iter()
            .filter(|(_, (_, input))| input.candidate.statement == id)
            .map(|(&key, _)| key)
            .collect();
        assert_eq!(keys.len(), 2);
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
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
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .shared_primary = None;
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.composed.as_mut().unwrap().shared_primary = None;
            }
            12 => {
                checker.proofs.mutable.insert(copy);
            }
            13 | 14 => {
                let point = if fault == 13 { original } else { read };
                let op = checker.local_reads.get_mut(&point).unwrap();
                op.normal = false;
                op.edges.truncate(1);
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            15 => {
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
            16 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.initialized.fill(false);
            }
            _ => unreachable!(),
        }
        let expected: Uses = reports
            .slot_uses
            .iter()
            .filter(|(port, _)| !ports.contains(port))
            .map(|(&port, &slot)| (port, slot))
            .collect();
        assert_eq!(expected.len(), 2);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        for error in [
            checker
                .candidate_inputs(&reports, Span::default())
                .expect_err("stale candidates"),
            checker
                .candidate_walk_report(&reports, Span::default())
                .expect_err("stale candidate walk"),
            checker
                .direct_walk_report(&reports, Span::default())
                .expect_err("stale expanded walk"),
        ] {
            assert!(
                error.message.contains("identity"),
                "fault {fault}: {error:?}"
            );
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        reports.slot_uses = expected;
        if matches!(fault, 7 | 9 | 16) {
            assert!(
                checker
                    .candidate_walk_report(&reports, Span::default())
                    .is_err()
            );
            assert!(
                checker
                    .direct_walk_report(&reports, Span::default())
                    .is_err()
            );
            continue;
        }
        let inputs = checker
            .candidate_inputs(&reports, Span::default())
            .unwrap()
            .0;
        for key in &keys {
            assert_eq!(inputs[key].1.source, None);
        }
        assert!(
            checker
                .candidate_walk_report(&reports, Span::default())
                .is_err()
        );
        reports.candidate_inputs = inputs;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for walk in [
            checker
                .candidate_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
        ] {
            for key in &keys {
                assert!(
                    walk.visits
                        .contains(&Visit::Unresolved(*key, reports.candidate_inputs[key].1))
                );
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
