use super::{super::tests::context, *};

#[test]
pub(crate) fn argument_forest_view_borrows_complete_history_and_indexes_original_markers() {
    let (mut checker, reports) =
        context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,2);f(copy,3)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (view, parts) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert!(std::ptr::eq(view.reports, &reports));
    assert!(std::ptr::eq(view.walk, &reports.argument_walk));
    assert_eq!(view.index.len(), reports.argument_sources.len());
    for (&point, &position) in &view.index {
        let (owner, direct) = reports.argument_sources[&point];
        assert_eq!(view.walk.visits[position], Visit::Argument(owner, direct));
    }
    assert!(
        view.walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Shared(_)))
    );
    assert!(parts < MAX_EDGES);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_forest_view_rejects_missing_extra_changed_and_reordered_visits() {
    for fault in 0..8 {
        let (mut checker, mut reports) =
            context("f:(a<int32>,b<int32>){};r:{->n:1};copy:r.n;f(copy,copy)");
        let visits = &mut reports.argument_walk.visits;
        match fault {
            0 => {
                visits.remove(0);
            }
            1 => visits.push(visits[0]),
            2 => visits.swap(0, 1),
            3 => {
                let Visit::Argument(owner, _) = &mut visits[0] else {
                    panic!()
                };
                *owner += 1;
            }
            4 => {
                let Visit::Argument(_, direct) = &mut visits[0] else {
                    panic!()
                };
                direct.point = usize::MAX;
            }
            5 => {
                let index = visits
                    .iter()
                    .position(|visit| matches!(visit, Visit::Value(..)))
                    .unwrap();
                visits.remove(index);
            }
            6 => {
                let Visit::Root(root) = &mut visits[1] else {
                    panic!()
                };
                root.owner += 1;
            }
            7 => {
                let Visit::Shared(root) = visits.last_mut().unwrap() else {
                    panic!()
                };
                root.slot.index = usize::MAX;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_forest_view_bounds_index_payload_and_exact_combined_work() {
    let (mut checker, reports) = context("f:(a<int32>,b<int32>){};f(1,2)");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let (view, left) = checker
        .argument_forest_view(&reports, Span::default(), 2, MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - start;
    let index = view.index;
    let used = MAX_EDGES - left;
    let (_, parts) = checker
        .argument_walk_report(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(parts - left, 2);
    for (limit, parts) in [(1, used), (2, used - 1), (2, used)] {
        let result = checker.argument_forest_view(&reports, Span::default(), limit, parts);
        if limit == 2 && parts == used {
            let (view, left) = result.unwrap();
            assert_eq!(left, 0);
            assert_eq!(view.index, index);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for short in [0, 1] {
        checker.flow = crate::flow::Flow::new();
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        let result = checker.argument_forest_view(&reports, Span::default(), 2, used);
        if short == 0 {
            assert_eq!(result.unwrap().1, 0);
            assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_forest_view_rejects_oversized_retained_walk_before_reconstruction() {
    let (mut checker, mut reports) = context("f:(a<int32>){};f(1)");
    reports.argument_walk.visits = vec![reports.argument_walk.visits[0]; MAX_EDGES + 1];
    let before = (checker.flow.work, reports.parts, checker.edge_counts());
    assert!(
        checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        (checker.flow.work, reports.parts, checker.edge_counts()),
        before
    );
}
