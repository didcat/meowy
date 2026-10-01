use super::{tests::check, *};

#[test]
pub(crate) fn projection_counts_keep_owned_loaded_narrowed_and_address_bounds() {
    for source in [
        "r:{->a:0;->n:1;->z:2};h:{->a:0;->p:&r};q:&(h.p.n)",
        "r:{->a:0;->n:1;->z:2};h:{->a:0;->p:&r};v:&h;q:&(v.p.n)",
        "<R>:<{a<int32>;n<int32>;z<int32>}>;r<R>:{->a:0;->n:1;->z:2};h:{->a:0;->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let plan = checker.projections.values().next().unwrap();
        let bounds = plan
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Field { index, count, .. } | Step::Address { index, count } => {
                    Some((*index, *count))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(bounds, [(1, 2), (1, 3)]);
    }
    let mut checker = Checker::new();
    let body = checker
        .block(
            &crate::parser::parse("x:*(&({->a:0;->n:1;->z:2}.n))").unwrap(),
            None,
            None,
        )
        .unwrap();
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    assert_eq!(
        plan.steps.last(),
        Some(&Step::Address { index: 1, count: 3 })
    );
    let before = plan.clone();
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    checker.entry_reports(&program, Span::default()).unwrap();
    assert!(checker.locals.is_empty());
    assert_eq!(checker.projections[&id], before);
}

#[test]
pub(crate) fn projection_counts_validate_exact_bounds_and_capture_work_atomically() {
    let mut checker = check("r:{->a:0;->n:1;->z:2};h:{->a:0;->p:&r};q:&(h.p.n)");
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let plan = plan.clone();
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    for step in 0..2 {
        let mut invalid = plan.clone();
        match &mut invalid.steps[step] {
            Step::Field { index, count, .. } | Step::Address { index, count } => *count = *index,
            _ => panic!(),
        }
        assert!(
            checker
                .capture_projection(id, invalid)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert!(checker.projections.is_empty());
    }
    let before = checker.flow.work;
    checker.capture_projection(id, plan.clone()).unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.projections.clear();
        checker.projection_items = 0;
        checker.projection_edges = 0;
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.capture_projection(id, plan.clone());
        assert_eq!(result.is_ok(), spare == 0);
        if result.is_ok() {
            assert_eq!(checker.projections[&id], plan);
        } else {
            assert!(checker.projections.is_empty());
            assert_eq!((checker.projection_items, checker.projection_edges), (0, 0));
        }
    }
}
