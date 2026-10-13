use super::*;

#[test]
pub(crate) fn shared_local_receiver_graphs_share_exact_source_cache_and_work_limits() {
    let (mut checker, reports) = checked(SOURCE);
    let keys: Vec<_> = reports
        .candidate_inputs
        .iter()
        .filter(|(_, (_, input))| input.source.is_some())
        .map(|(&key, _)| key)
        .collect();
    assert_eq!(keys.len(), 2);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    let mut ctx = Context::new(&reports, 3);
    for key in &keys {
        assert_eq!(
            checker
                .candidate_input(&mut ctx, *key, Span::default())
                .unwrap(),
            reports.candidate_inputs[key]
        );
    }
    let work = checker.flow.work - start;
    assert_eq!(
        (
            ctx.parts,
            ctx.sources.len(),
            ctx.emissions.len(),
            ctx.blocks.len()
        ),
        (0, 1, 1, 1)
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut ctx = Context::new(&reports, 3);
        let result: Result<Vec<_>> = keys
            .iter()
            .map(|key| checker.candidate_input(&mut ctx, *key, Span::default()))
            .collect();
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    checker.flow = crate::flow::Flow::new();
    let mut ctx = Context::new(&reports, 2);
    assert!(
        checker
            .candidate_input(&mut ctx, keys[0], Span::default())
            .unwrap_err()
            .message
            .contains("candidate-source budget")
    );
    assert!(ctx.sources.is_empty());
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn shared_local_receiver_graphs_bound_forest_room_and_exact_work() {
    let (mut checker, mut reports) = checked(SOURCE);
    let mut minimum = None;
    for parts in 0..256 {
        reports.parts = parts;
        checker.flow = crate::flow::Flow::new();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let result = checker.candidate_walk_report(&reports, Span::default());
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        match result {
            Ok((walk, _)) => {
                assert_eq!(walk, reports.candidate_walk);
                minimum = Some(parts);
                break;
            }
            Err(error) => assert!(error.message.contains("budget")),
        }
    }
    assert_eq!(reports.parts, minimum.unwrap());
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    checker.flow = crate::flow::Flow::new();
    checker
        .candidate_walk_report(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.candidate_walk_report(&reports, Span::default());
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
