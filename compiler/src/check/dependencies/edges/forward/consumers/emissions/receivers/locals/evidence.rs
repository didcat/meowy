use super::{
    boundaries::{SOURCE, input_read},
    *,
};

#[test]
pub(crate) fn shared_local_receiver_emissions_preserve_earlier_links_when_evidence_is_missing() {
    for fault in 0..11 {
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
        let slot = reports.slot_uses[&Port::Emission(op.targets[0].id)].1;
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
                let source = reports.results[&slot.block].1.dispatch.unwrap();
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            7 => {
                reports
                    .effects
                    .remove(checker.typed_ops.keys().last().unwrap());
            }
            8 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.composed.as_mut().unwrap().shared_primary = None;
                checker
                    .emissions
                    .get_mut(&id)
                    .unwrap()
                    .composed
                    .as_mut()
                    .unwrap()
                    .shared_primary = None;
            }
            9 => {
                let (_, Effect::Emission(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                op.initialized.fill(false);
            }
            10 => {
                checker.proofs.mutable.insert(local);
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
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
