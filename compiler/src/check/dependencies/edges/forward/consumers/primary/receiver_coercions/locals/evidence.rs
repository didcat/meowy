use super::{
    boundaries::{SOURCE, input_read},
    *,
};

#[test]
pub(crate) fn shared_local_receiver_coercions_preserve_earlier_links_when_evidence_is_missing() {
    for fault in 0..13 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
        let (&cell, &(_, receiver, _)) = reports.receivers.last_key_value().unwrap();
        let read = input_read(&checker, &reports, receiver);
        let local = checker.local_reads[&read].local;
        let binding = reports.shared_initializers[&local].init;
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
                let source = reports.results[&block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            7 => {
                reports.results.remove(&block);
            }
            8 => {
                checker.coercions.get_mut(&id).unwrap().fields = None;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = None;
            }
            9 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            10 => {
                checker.proofs.mutable.insert(local);
            }
            11 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = false;
            }
            12 => {
                reports
                    .shared_initializers
                    .get_mut(&local)
                    .unwrap()
                    .init
                    .input = None;
                let op = checker.operations.get_mut(&binding.statement).unwrap();
                op.input = None;
                op.edges = vec![Edge::new(
                    Port::Operation(binding.statement),
                    Port::Normal(binding.statement),
                    Route::Next,
                )];
                let (_, Effect::Storage { input, .. }) =
                    reports.effects.get_mut(&binding.statement).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        let mut expected = reports.slot_uses.clone();
        expected.remove(&port);
        assert_eq!(expected.len(), 1);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected,
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
