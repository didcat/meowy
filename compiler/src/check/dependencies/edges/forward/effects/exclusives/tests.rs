use super::{super::tests::checked, *};

#[test]
pub(crate) fn exclusive_effects_keep_paths_lengths_and_per_stage_observations() {
    for source in [
        "r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])",
        "xs:[{->n:=1}];p:&!(xs[1].n)",
        "xs:=[true];p:&!(xs[1])",
        "xs<float32[1]>:=[1.25];p:&!(xs[1])",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.exclusives.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                op.owner,
                Effect::Exclusive(Observed {
                    place: op.place.clone(),
                    storage: op.storage,
                    steps: op.steps.clone(),
                    counts: op.counts.clone(),
                    access: op.access.clone(),
                    normal: true,
                    control: false,
                    addresses: vec![true; op.steps.len() + 1],
                    reservations: op
                        .steps
                        .iter()
                        .map(|step| matches!(step, PathStep::Index { .. }))
                        .collect(),
                    acquired: true,
                    result: true,
                })
            )
        );
        assert!(!checker.place_borrows.contains_key(&id));
        assert!(!checker.derefs.contains_key(&id));
    }
}

#[test]
pub(crate) fn exclusive_effects_keep_address_reservation_acquisition_and_result_visits_independent()
{
    let (mut checker, mut reports) =
        checked("r:{->rows:=[{->xs:=[1]}]};p:&!(r.rows[1].xs[1])", false);
    let id = *checker.exclusives.first_key_value().unwrap().0;
    for port in (0..=3)
        .map(|step| Port::Address { point: id, step })
        .chain([
            Port::Reserve { point: id, step: 0 },
            Port::Reserve { point: id, step: 2 },
            Port::Operation(id),
            Port::Normal(id),
        ])
    {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 15, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Exclusive(op)) = &effects[&id] else {
            panic!()
        };
        for (step, &seen) in op.addresses.iter().enumerate() {
            assert_eq!(seen, port == Port::Address { point: id, step });
        }
        for (step, &seen) in op.reservations.iter().enumerate() {
            assert_eq!(seen, port == Port::Reserve { point: id, step });
        }
        assert_eq!(op.acquired, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn exclusive_effects_exclude_disconnected_stages_after_stopped_indices() {
    for (source, stop) in [
        ("xs:=[[1]];'out{p:&!(xs[{'out.leave()}][1])}", 0),
        ("xs:=[[1]];'out{p:&!(xs[1][{'out.leave()}])}", 1),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.exclusives.first_key_value().unwrap();
        let (_, Effect::Exclusive(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.steps, op.steps);
        assert_eq!(observed.access, op.access);
        assert!(!observed.normal && !observed.acquired && !observed.result);
        assert_eq!(
            observed.addresses,
            (0..=2).map(|step| step <= stop).collect::<Vec<_>>()
        );
        assert_eq!(
            observed.reservations,
            (0..2).map(|step| step <= stop).collect::<Vec<_>>()
        );
        assert!(!reports.index.operations.contains_key(&id));
    }
}

#[test]
pub(crate) fn exclusive_effects_keep_alias_storage_owners_and_control() {
    let source = "flag:false;xs:=[1];|flag|{p:&!(xs[1])};f:(){r:{->xs:=[1];p:&!(xs[1]);*p=2}}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.exclusives.len(), 2);
    assert!(checker.exclusives.values().any(|op| op.control));
    assert!(checker.exclusives.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.exclusives {
        let (owner, Effect::Exclusive(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.storage, op.storage);
        if let Some(alias) = checker.proofs.aliases.get(&op.place.root) {
            assert_eq!(observed.storage, alias.root);
        }
    }
}
