use super::{super::super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn field_result_lookup_preserves_owners_and_reuses_bounded_qualification() {
    let (mut checker, reports) = checked("r:{->n:1};a:r.n;b:r.n;f<int32>:(){->{->n:2}.n}");
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let mut ctx = Lookup::new(&reports, roots + reports.field_results.len());
    for (&port, &(owner, slot)) in &reports.field_results {
        let Port::Normal(id) = port else { panic!() };
        assert_eq!(
            checker
                .field_result_source(&mut ctx, id, owner, Span::default())
                .unwrap(),
            Some(slot)
        );
        let parts = ctx.parts;
        assert_eq!(
            checker
                .field_result_source(&mut ctx, id, owner, Span::default())
                .unwrap(),
            Some(slot)
        );
        assert_eq!(ctx.parts, parts);
        assert!(
            checker
                .field_result_source(&mut ctx, id, owner + 1, Span::default())
                .is_err()
        );
    }
    assert_eq!(ctx.parts, 0);
    assert_eq!(ctx.checked.len(), reports.field_results.len());
}

#[test]
pub(crate) fn field_result_lookup_keeps_missing_associations_opaque_and_rejects_corruption() {
    for fault in 0..8 {
        let (mut checker, mut reports) = checked("v:{->n:1}.n");
        let id = *checker.fields.first_key_value().unwrap().0;
        match fault {
            0 => {
                reports.field_results.clear();
            }
            1 => reports.field_results.get_mut(&Port::Normal(id)).unwrap().0 += 1,
            2 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(id))
                    .unwrap()
                    .1
                    .index = 0
            }
            3 => {
                reports
                    .field_results
                    .get_mut(&Port::Normal(id))
                    .unwrap()
                    .1
                    .block = usize::MAX
            }
            4 => {
                reports.slot_uses.clear();
            }
            5 => {
                reports.effects.remove(&id);
            }
            6 => reports.candidate_walk.visits.clear(),
            7 => {
                let Effect::Field { result, .. } = &mut reports.effects.get_mut(&id).unwrap().1
                else {
                    panic!()
                };
                *result = false;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut ctx = Lookup::new(&reports, MAX_EDGES);
        let result = checker.field_result_source(&mut ctx, id, 0, Span::default());
        if fault == 0 {
            assert_eq!(result.unwrap(), None);
            assert!(ctx.roots.is_none());
        } else {
            assert!(
                result.unwrap_err().message.contains("identity"),
                "fault {fault}"
            );
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn field_result_lookup_obeys_exact_cache_and_work_limits() {
    let (mut checker, reports) = checked("v:{->n:1}.n");
    let id = *checker.fields.first_key_value().unwrap().0;
    let parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count()
        + 1;
    let before = checker.flow.work;
    let mut ctx = Lookup::new(&reports, parts);
    let expected = checker
        .field_result_source(&mut ctx, id, 0, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    assert_eq!(ctx.parts, 0);
    assert!(
        checker
            .field_result_source(
                &mut Lookup::new(&reports, parts - 1),
                id,
                0,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result =
            checker.field_result_source(&mut Lookup::new(&reports, parts), id, 0, Span::default());
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, expected);
        }
    }
}
