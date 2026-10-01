use super::{super::tests::checked, *};

#[test]
pub(crate) fn projection_effects_share_exact_work_effect_and_payload_limits() {
    let source = "<R>:<{n<int32>}>;<H>:<{p<&R><null>}>;r<R>:{->n:1};h:{->inner<H><null>:{->p:&r}};|h.inner<H>|{|h.inner.p<&R>|q:&(h.inner.p.n)}";
    for (missing, parts, pass) in [(0, 10, true), (0, 9, false), (1, 10, false)] {
        let (mut checker, mut reports) = checked(source, false);
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.extend(walk.ports.clone());
        }
        let expected = reports.effects.clone();
        let ops = checker.projections.clone();
        let counts = checker.edge_counts();
        let limit = expected.len() - missing;
        let before = checker.flow.work;
        let result = checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
        assert_eq!(result.is_ok(), pass);
        let work = checker.flow.work - before;
        if let Ok(actual) = result {
            assert_eq!(actual, expected);
            for spare in [0, 1] {
                checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
                let result =
                    checker.operation_effects_limited(&reports, Span::default(), limit, parts, 0);
                assert_eq!(result.is_ok(), spare == 0);
                if let Ok(actual) = result {
                    assert_eq!(actual, expected);
                }
            }
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.projections, ops);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn projection_effects_preserve_partial_records_on_conflicts_and_limits() {
    let source = "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)";
    let (mut checker, _) = checked(source, false);
    let id = *checker.projections.first_key_value().unwrap().0;
    let port = Port::Projection { point: id, step: 0 };
    let mut effects = Effects::new();
    let mut parts = 5;
    assert!(
        checker
            .record_projection_effect(0, port, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 5);
    parts = 6;
    assert!(
        checker
            .record_projection_effect(0, port, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 6);
    checker
        .record_projection_effect(0, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(parts, 0);
    let expected = effects.clone();
    for fault in 0..12 {
        let mut effects = expected.clone();
        let (owner, Effect::Projection(op)) = effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.parent += 1,
            2 => op.site += 1,
            3 => op.parent_mode = crate::hir::ReferenceMode::Exclusive,
            4 => op.control = true,
            5 => {
                let ProjectionStep::Field { count, .. } = &mut op.steps[0] else {
                    panic!()
                };
                *count += 1;
            }
            6 => {
                let ProjectionStep::Field { narrow, .. } = &mut op.steps[0] else {
                    panic!()
                };
                *narrow = false;
            }
            7 => {
                let ProjectionStep::Field { index, .. } = &mut op.steps[0] else {
                    panic!()
                };
                *index += 1;
            }
            8 => op.steps[0] = ProjectionStep::Load(crate::hir::ReferenceMode::Shared),
            9 => op.projected.clear(),
            10 => op.converted.clear(),
            11 => {
                effects.insert(id, (0, Effect::Unknown));
            }
            _ => unreachable!(),
        }
        let before = effects.clone();
        assert!(
            checker
                .record_projection_effect(
                    0,
                    Port::Normal(id),
                    &mut effects,
                    1,
                    &mut parts,
                    Span::default()
                )
                .is_err()
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
    for invalid in [
        Port::Entry(id),
        Port::Projection { point: id, step: 2 },
        Port::Conversion { point: id, part: 1 },
    ] {
        assert!(
            checker
                .record_projection_effect(0, invalid, &mut effects, 1, &mut parts, Span::default())
                .is_err()
        );
        assert_eq!(effects, expected);
    }
    checker
        .record_projection_effect(0, port, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert!(
        checker
            .record_projection_effect(
                0,
                Port::Normal(id),
                &mut effects,
                1,
                &mut parts,
                Span::default()
            )
            .is_err()
    );
    assert_eq!(effects, expected);
    assert_eq!(parts, 0);
}

#[test]
pub(crate) fn projection_effects_bound_maximum_paths_and_independent_conversion_flags() {
    let (mut checker, mut reports) = checked("r:{->n:1};h:{->p:&r};q:&(h.p.n)", false);
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let mut plan = plan.clone();
    let max = crate::list::MAX_WRITE_PATH;
    plan.steps = vec![
        ProjectionStep::Field {
            index: 0,
            count: 1,
            narrow: true
        };
        max
    ];
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    checker.capture_projection(id, plan).unwrap();
    let port = Port::Conversion {
        point: id,
        part: max - 1,
    };
    reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
    let before = reports.effects.clone();
    let effects = checker
        .operation_effects_limited(&reports, Span::default(), 1, max * 3, 0)
        .unwrap();
    let (_, Effect::Projection(op)) = &effects[&id] else {
        panic!()
    };
    assert_eq!(op.steps.len(), max);
    assert!(op.projected.iter().all(|seen| !seen));
    assert_eq!(op.converted.iter().filter(|&&seen| seen).count(), 1);
    assert!(op.converted[max - 1] && !op.acquired && !op.result);
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, max * 3 - 1, 0)
            .is_err()
    );
    checker
        .projections
        .get_mut(&id)
        .unwrap()
        .steps
        .push(ProjectionStep::Address { index: 0, count: 1 });
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), 1, usize::MAX, 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(reports.effects, before);
}
