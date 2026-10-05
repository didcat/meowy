use super::{super::super::tests::checked, *};

pub(super) const SOURCE: &str = "a:{->n:1}.n;b:{->n:2}.n";

#[test]
pub(crate) fn field_results_share_exact_map_root_scratch_and_work_limits() {
    let (mut checker, reports) = checked(SOURCE);
    let expected = &reports.field_results;
    let base = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len()
        + reports.candidate_inputs.len();
    let limit = base + expected.len();
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let work = checker.flow.work;
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), limit, roots)
            .unwrap(),
        (expected.clone(), 0)
    );
    let work = checker.flow.work - work;
    for (limit, parts) in [(base - 1, roots), (limit - 1, roots), (limit, roots - 1)] {
        assert!(
            checker
                .field_results_limited(&reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_results_limited(&reports, Span::default(), limit, roots);
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, (expected.clone(), 0));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn field_results_discard_late_header_link_and_root_failures_atomically() {
    for fault in 0..11 {
        let (mut checker, mut reports) = checked(SOURCE);
        let id = *checker.fields.last_key_value().unwrap().0;
        let slot = reports.slot_uses[&Port::Operation(id)].1;
        match fault {
            0..=3 => {
                let Effect::Field {
                    input,
                    control,
                    result,
                    operation,
                    ..
                } = &mut reports.effects.get_mut(&id).unwrap().1
                else {
                    panic!()
                };
                match fault {
                    0 => *input = usize::MAX,
                    1 => *control = !*control,
                    2 => {
                        *operation = false;
                        *result = false;
                    }
                    3 => {
                        reports.index.operations.remove(&id);
                    }
                    _ => unreachable!(),
                }
            }
            4 => reports.slot_uses.get_mut(&Port::Operation(id)).unwrap().0 += 1,
            5 => {
                reports
                    .slot_uses
                    .get_mut(&Port::Operation(id))
                    .unwrap()
                    .1
                    .index = 0
            }
            6 => reports
                .candidate_walk
                .visits
                .retain(|visit| !matches!(visit, Visit::Root(root) if root.slot == slot)),
            7..=9 => {
                let visit = reports
                    .candidate_walk
                    .visits
                    .iter_mut()
                    .find(|visit| matches!(visit, Visit::Root(root) if root.slot == slot))
                    .unwrap();
                let Visit::Root(root) = visit else { panic!() };
                match fault {
                    7 => root.owner += 1,
                    8 => root.slot.index = usize::MAX,
                    9 => root.slot.block = usize::MAX,
                    _ => unreachable!(),
                }
            }
            10 => {
                let visit = *reports
                    .candidate_walk
                    .visits
                    .iter()
                    .find(|visit| matches!(visit, Visit::Root(root) if root.slot == slot))
                    .unwrap();
                reports.candidate_walk.visits.push(visit);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn field_results_require_result_observation_and_reject_never_result_claims() {
    for source in [
        "f<never>:(r<{n<never>}>){->r.n}",
        "f<never>:(r<&{n<never>}>){->r.n}",
    ] {
        let (mut checker, mut reports) = checked(source);
        assert!(reports.field_results.is_empty());
        let id = *checker.fields.first_key_value().unwrap().0;
        let Effect::Field { result, .. } = &mut reports.effects.get_mut(&id).unwrap().1 else {
            panic!()
        };
        *result = true;
        let before = format!("{reports:?}");
        assert!(
            checker
                .field_results_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}"), before);
    }
    let (mut checker, mut reports) = checked(SOURCE);
    let ids: Vec<_> = checker.fields.keys().copied().collect();
    for (id, operation, result) in [(ids[0], true, false), (ids[1], false, true)] {
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
    }
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, 0)
            .unwrap(),
        (Uses::new(), 0)
    );
}

#[test]
pub(crate) fn field_results_keep_loads_opaque_and_do_not_rebuild_forest_visits() {
    let (mut checker, mut reports) = checked("r:{->n:1};p:&r;x:p.n;y:(*p).n");
    assert!(reports.field_results.is_empty());
    reports.candidate_walk.visits.clear();
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, 0)
            .unwrap(),
        (Uses::new(), 0)
    );
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.field_results.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    reports.effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let before = format!("{reports:?}");
    assert_eq!(
        checker
            .field_results_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        expected
    );
    assert_eq!(format!("{reports:?}"), before);
}
