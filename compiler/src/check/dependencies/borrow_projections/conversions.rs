use super::{tests::check, *};

#[test]
pub(crate) fn projection_conversions_follow_each_owned_field_before_reborrow() {
    let source = "<R>:<{n<int32>}>;<H>:<{p<&R><null>}>;r<R>:{->n:1};h:{->inner<H><null>:{->p:&r}};|h.inner<H>|{|h.inner.p<&R>|q:&(h.inner.p.n)}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let field = |step| Port::Projection { point: id, step };
    let convert = |part| Port::Conversion { point: id, part };
    assert_eq!(plan.steps.len(), 3);
    assert_eq!(
        plan.edges,
        [
            Edge::new(Port::Entry(id), Port::Entry(plan.parent), Route::Next),
            Edge::new(Port::Normal(plan.parent), field(0), Route::Next),
            Edge::new(field(0), convert(0), Route::Next),
            Edge::new(convert(0), field(1), Route::Next),
            Edge::new(field(1), convert(1), Route::Next),
            Edge::new(convert(1), field(2), Route::Next),
            Edge::new(field(2), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]
    );
}

#[test]
pub(crate) fn projection_conversions_preserve_later_loads_owner_and_control() {
    let source =
        "<R>:<{n<int32>}>;<H>:<{p<&R>}>;f:(h<{maybe<&H><null>}>){|h.maybe<&H>|q:&(h.maybe.p.n)}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    assert_eq!(
        plan.steps,
        [
            Step::Field {
                index: 0,
                narrow: true
            },
            Step::Load(hir::ReferenceMode::Shared),
            Step::Field {
                index: 0,
                narrow: false
            },
            Step::Address(0),
        ]
    );
    assert_ne!(plan.owner, 0);
    assert_eq!(plan.mode, Some(hir::ReferenceMode::Shared));
    assert!(plan.edges.contains(&Edge::new(
        Port::Conversion { point: id, part: 0 },
        Port::Projection { point: id, step: 1 },
        Route::Next
    )));
    assert_eq!(
        plan.edges
            .iter()
            .filter(|edge| matches!(edge.to, Port::Conversion { .. }))
            .count(),
        1
    );
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;r:{->n:1};h:{->p:&r};|flag|q:&(h.p.n)").unwrap(),
            None,
            None,
        )
        .unwrap();
    let plan = checker.projections.values().next().unwrap();
    assert!(plan.control);
    assert!(
        !plan
            .edges
            .iter()
            .any(|edge| matches!(edge.to, Port::Conversion { .. }))
    );
}

#[test]
pub(crate) fn projection_conversions_reject_loaded_flags_and_publish_budgets_atomically() {
    let source = "<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p<&R><null>:&r};|h.p<&R>|q:&(h.p.n)";
    let mut checker = check(source);
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let plan = plan.clone();
    let counts = (checker.projection_items, checker.projection_edges);
    checker.capture_projection(id, plan.clone()).unwrap();
    assert_eq!((checker.projection_items, checker.projection_edges), counts);
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    let mut invalid = plan.clone();
    invalid
        .steps
        .insert(0, Step::Load(hir::ReferenceMode::Shared));
    assert!(
        checker
            .capture_projection(id, invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(checker.projections.is_empty());
    checker.narrowing_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_projection(id, plan)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.projections.is_empty());
    assert_eq!((checker.projection_items, checker.projection_edges), (0, 0));
}

#[test]
pub(crate) fn projection_conversions_keep_path_step_limits_independent_of_edge_counts() {
    let mut checker = check("r:{->n:1};h:{->p:&r};q:&(h.p.n)");
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let mut plan = plan.clone();
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    plan.steps = vec![
        Step::Field {
            index: 0,
            narrow: true
        };
        crate::list::MAX_WRITE_PATH
    ];
    checker.capture_projection(id, plan.clone()).unwrap();
    assert_eq!(checker.projection_items, crate::list::MAX_WRITE_PATH + 1);
    assert_eq!(
        checker.projection_edges,
        crate::list::MAX_WRITE_PATH * 2 + 3
    );
    assert!(
        checker
            .projection_step(&mut plan, Step::Address(0))
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(
        checker.projections[&id].steps.len(),
        crate::list::MAX_WRITE_PATH
    );
}
