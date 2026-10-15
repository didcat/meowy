use super::{boundaries::SOURCE, *};

#[test]
pub(crate) fn receiver_copy_coercions_preserve_earlier_links_when_observations_are_missing() {
    for fault in 0..19 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker
            .coercions
            .iter()
            .find(|(_, op)| op.primary && op.kind == CoercionKind::Convert)
            .unwrap()
            .0;
        let port = Port::Projection { point: id, step: 0 };
        let block = reports.slot_uses[&port].1.block;
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
                let source = reports.results[&block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            10 => {
                reports.results.remove(&block);
            }
            11 => {
                checker.coercions.get_mut(&id).unwrap().fields = None;
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.fields = None;
            }
            12 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            13 => {
                checker.proofs.mutable.insert(copy);
            }
            14 | 15 => {
                let point = if fault == 14 { original } else { read };
                let op = checker.local_reads.get_mut(&point).unwrap();
                op.normal = false;
                op.edges.truncate(1);
                let (_, Effect::Read { normal, .. }) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                *normal = false;
            }
            16 => {
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
            17 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.projected = false;
            }
            18 => {
                reports.effects.remove(&id);
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
