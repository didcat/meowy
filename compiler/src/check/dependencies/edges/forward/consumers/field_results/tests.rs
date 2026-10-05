use super::{super::tests::checked, *};

#[test]
pub(crate) fn field_result_slots_qualify_direct_copies_and_independent_owners() {
    let source = "r:{->n:1};s:((r))~<{n<int32>}>;a:s.n;f<int32>:(){->{->n:2}.n}";
    let (mut checker, reports) = checked(source);
    let ids: Vec<_> = checker
        .fields
        .iter()
        .map(|(&id, op)| (id, op.owner))
        .collect();
    assert!(ids.iter().any(|(_, owner)| *owner != 0));
    for (id, owner) in ids {
        let slot = checker
            .field_result_slot(
                &reports,
                id,
                owner,
                &reports.effects[&id].1,
                Span::default(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(reports.slot_uses[&Port::Operation(id)], (owner, slot));
        assert_eq!(checker.bodies[&slot.block].owner, owner);
    }
}

#[test]
pub(crate) fn field_result_slots_require_both_visits_and_an_existing_operation_link() {
    let (mut checker, mut reports) = checked("v:{->n:1}.n");
    let id = *checker.fields.first_key_value().unwrap().0;
    let expected = reports.slot_uses[&Port::Operation(id)].1;
    for (operation, result) in [(true, true), (true, false), (false, true)] {
        let Effect::Field {
            operation: op,
            result: value,
            ..
        } = &mut reports.effects.get_mut(&id).unwrap().1
        else {
            panic!()
        };
        *op = operation;
        *value = result;
        assert_eq!(
            checker
                .field_result_slot(&reports, id, 0, &reports.effects[&id].1, Span::default())
                .unwrap(),
            (operation && result).then_some(expected)
        );
    }
    let Effect::Field { operation, .. } = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    *operation = true;
    reports.slot_uses.clear();
    assert_eq!(
        checker
            .field_result_slot(&reports, id, 0, &reports.effects[&id].1, Span::default())
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn field_result_slots_keep_never_loads_calls_and_field_values_opaque() {
    for source in [
        "f<never>:(r<{n<never>}>){->r.n}",
        "r:{->n:1};p:&r;x:p.n",
        "r:{->n:1};p:&r;x:(*p).n",
        "f<{n<int32>}>:(){->n:1};x:f().n",
    ] {
        let (mut checker, reports) = checked(source);
        let (&id, op) = checker.fields.last_key_value().unwrap();
        let owner = op.owner;
        assert_eq!(
            checker
                .field_result_slot(
                    &reports,
                    id,
                    owner,
                    &reports.effects[&id].1,
                    Span::default()
                )
                .unwrap(),
            None,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn field_result_slots_keep_inner_slot_identity_without_forwarding_its_value() {
    let (mut checker, reports) = checked("r:{->inner:{->n:1}};x:r.inner.n");
    let ids: Vec<_> = checker
        .fields
        .iter()
        .map(|(&id, op)| (id, op.owner))
        .collect();
    let slots: Vec<_> = ids
        .into_iter()
        .map(|(id, owner)| {
            checker
                .field_result_slot(
                    &reports,
                    id,
                    owner,
                    &reports.effects[&id].1,
                    Span::default(),
                )
                .unwrap()
        })
        .collect();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots.iter().filter(|slot| slot.is_some()).count(), 1);
    let slot = slots.into_iter().flatten().next().unwrap();
    assert_eq!(
        reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index],
        super::super::super::results::Sources::Unknown
    );
}

#[test]
pub(crate) fn field_result_slots_reject_mismatched_operation_links() {
    for fault in 0..3 {
        let (mut checker, mut reports) = checked("a:{->n:1}.n;b:{->n:2}.n");
        let id = *checker.fields.last_key_value().unwrap().0;
        let (owner, slot) = reports.slot_uses.get_mut(&Port::Operation(id)).unwrap();
        match fault {
            0 => *owner += 1,
            1 => slot.index = 0,
            2 => slot.block = usize::MAX,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .field_result_slot(&reports, id, 0, &reports.effects[&id].1, Span::default())
            .unwrap_err();
        assert!(error.message.contains("identity"));
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
