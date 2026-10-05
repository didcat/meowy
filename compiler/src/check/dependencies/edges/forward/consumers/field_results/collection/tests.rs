use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn field_results_link_normal_ports_to_existing_forest_roots() {
    let (mut checker, reports) = checked("r:{->n:1};s:{->r};a:s.n;f<int32>:(){->{->n:2}.n}");
    assert_eq!(reports.field_results.len(), 2);
    for (&port, &(owner, slot)) in &reports.field_results {
        let Port::Normal(id) = port else { panic!() };
        assert_eq!(reports.slot_uses[&Port::Operation(id)], (owner, slot));
        assert!(reports.candidate_walk.visits.iter().any(
            |visit| matches!(visit, Visit::Root(root) if root.owner == owner && root.slot == slot)
        ));
    }
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        reports.field_results
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn field_results_preserve_sparse_flags_and_allocate_roots_only_for_eligible_links() {
    let (mut checker, mut reports) = checked("v:{->n:1}.n");
    let id = *checker.fields.first_key_value().unwrap().0;
    for (operation, result) in [(true, false), (false, true)] {
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
                .field_results_limited(&reports, Span::default(), MAX_EDGES, 0)
                .unwrap(),
            (Uses::new(), 0)
        );
    }
    let Effect::Field { operation, .. } = &mut reports.effects.get_mut(&id).unwrap().1 else {
        panic!()
    };
    *operation = true;
    reports.slot_uses.clear();
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, 0)
            .unwrap(),
        (Uses::new(), 0)
    );
}

#[test]
pub(crate) fn field_results_preserve_unknown_empty_and_multiple_histories_without_value_forwarding()
{
    use super::super::super::super::results::Sources;
    for (source, count) in [
        ("v:{->n:=1}.n", None),
        ("v:(({->inner:{->n:1}}).inner).n", None),
        ("flag:=false;r:{|flag|->n:1;|!flag|->n:2};v:r.n", Some(2)),
        ("r<{n<null>}>:{};v:r.n", Some(0)),
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(reports.field_results.len(), 1, "{source}");
        let (_, slot) = reports.field_results.values().next().unwrap();
        let history = &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index];
        if let Some(count) = count {
            let Sources::Candidates(values) = history else {
                panic!()
            };
            assert_eq!(values.len(), count);
        } else {
            assert_eq!(*history, Sources::Unknown);
        }
        for (&port, &(owner, slot)) in &reports.field_results {
            let Port::Normal(id) = port else { panic!() };
            assert_eq!(checker.fields[&id].owner, owner);
            assert_eq!(reports.slot_uses[&Port::Operation(id)].1, slot);
        }
    }
}
