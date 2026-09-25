use super::*;

pub(crate) fn check(source: &str) -> Checker {
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    checker
}

#[test]
pub(crate) fn projection_plans_distinguish_owned_fields_loads_and_final_addresses() {
    use hir::ReferenceMode::Shared;
    for (source, steps) in [
        (
            "r:{->n:1};h:{->p:&r};q:&(h.p.n)",
            vec![Step::Field(0), Step::Address(0)],
        ),
        (
            "r:{->n:1};h:{->p:&r};v:&h;q:&(v.p.n)",
            vec![Step::Load(Shared), Step::Field(0), Step::Address(0)],
        ),
        (
            "r:{->n:1};h:{->p:&r};m:{->v:&h};p:&m;q:&(p.v.p.n)",
            vec![
                Step::Load(Shared),
                Step::Field(0),
                Step::Load(Shared),
                Step::Field(0),
                Step::Address(0),
            ],
        ),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (&id, plan) = checker.projections.first_key_value().unwrap();
        assert_eq!(plan.steps, steps);
        assert_eq!(plan.mode, Some(Shared));
        assert!(plan.site.is_some());
        assert_eq!(checker.points[plan.parent].parent, Some(id));
        assert_eq!(plan.span, checker.points[id].span);
    }
}

#[test]
pub(crate) fn projection_plans_preserve_materialization_and_indexed_parent_boundaries() {
    let source = "<R>:<{n<int32>}>;make<R>:(){->n:1};x:*(&(make().n))";
    crate::compile(source).unwrap();
    let checker = check(source);
    let plan = checker.projections.values().next().unwrap();
    let [Step::Materialize { local, statement }, Step::Address(0)] = plan.steps.as_slice() else {
        panic!()
    };
    assert_eq!(checker.proofs.temporaries[local], *statement);
    assert_eq!(
        checker.invocations.values().next().unwrap().point,
        plan.parent
    );
    for source in ["x:*(&((*(&{->n:1})).n))", "rows:[{->n:1}];q:&(rows[1].n)"] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let plan = checker.projections.values().next().unwrap();
        assert_eq!(plan.steps, [Step::Address(0)]);
        assert!(
            checker.elements.contains_key(&plan.parent) || !checker.proofs.temporaries.is_empty()
        );
    }
}

#[test]
pub(crate) fn projection_plans_preserve_stopped_parents_control_and_original_errors() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};f<never>:(){&(stop().missing)};f()"#;
    crate::compile(source).unwrap();
    let checker = check(source);
    let plan = checker.projections.values().next().unwrap();
    assert!(plan.steps.is_empty());
    assert!(plan.site.is_none() && plan.mode.is_none());
    assert_ne!(plan.owner, 0);
    let mut checker = Checker::new();
    checker.derived.insert(0);
    checker
        .block(
            &crate::parser::parse("flag:false;r:{->n:1};p:&r;|flag|q:&(p.n)").unwrap(),
            None,
            None,
        )
        .unwrap();
    assert!(checker.projections.values().next().unwrap().control);
    for (source, code) in [
        ("r:{->n:1};p:&r;q:&(p.missing)", "E201"),
        ("p:&({->n:1}.n);x:*p", "E303"),
        ("r:={->n:1};parent:&r;p:&(parent.n);r={->n:2};x:*p", "E302"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}

#[test]
pub(crate) fn projection_plans_bound_steps_and_publish_atomically() {
    let mut checker = check("r:{->n:1};p:&r;q:&(p.n)");
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let plan = plan.clone();
    let count = checker.projection_items;
    checker.capture_projection(id, plan.clone()).unwrap();
    assert_eq!(checker.projection_items, count);
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    let mut invalid = plan.clone();
    invalid.steps.push(Step::Field(0));
    assert!(
        checker
            .capture_projection(id, invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let mut invalid = plan.clone();
    invalid.site = Some(checker.reborrows);
    assert!(
        checker
            .capture_projection(id, invalid)
            .unwrap_err()
            .message
            .contains("identity")
    );
    let mut full = plan.clone();
    full.steps = vec![Step::Field(0); crate::list::MAX_WRITE_PATH];
    assert!(
        checker
            .projection_step(&mut full, Step::Address(0))
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(full.steps.len(), crate::list::MAX_WRITE_PATH);
    checker.projection_items = MAX_ITEMS;
    assert!(
        checker
            .capture_projection(id, plan)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.projections.is_empty());
    assert_eq!(checker.projection_items, MAX_ITEMS);
}

#[test]
pub(crate) fn projection_edges_follow_checked_steps_before_reborrow_and_result() {
    for source in [
        "r:{->n:1};h:{->p:&r};v:&h;q:&(v.p.n)",
        "<R>:<{n<int32>}>;make<R>:(){->n:1};x:*(&(make().n))",
        "rows:[{->xs:[{->n:1}]}];q:&(rows[1].xs[1].n);x:*q",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        for (&id, plan) in &checker.projections {
            assert_eq!(
                plan.edges[0],
                Edge::new(Port::Entry(id), Port::Entry(plan.parent), Route::Next)
            );
            let mut from = Port::Normal(plan.parent);
            for step in 0..plan.steps.len() {
                let to = Port::Projection { point: id, step };
                assert!(plan.edges.contains(&Edge::new(from, to, Route::Next)));
                from = to;
            }
            assert!(
                plan.edges
                    .contains(&Edge::new(from, Port::Operation(id), Route::Next))
            );
            assert!(plan.edges.contains(&Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Next
            )));
            assert!(
                !plan
                    .edges
                    .iter()
                    .any(|edge| edge.from == Port::Entry(id) && edge.to == Port::Normal(id))
            );
        }
        assert_eq!(
            checker.projection_edges,
            checker
                .projections
                .values()
                .map(|plan| plan.edges.len())
                .sum()
        );
    }
}

#[test]
pub(crate) fn projection_edges_preserve_stopped_and_opaque_parent_boundaries() {
    let source = r#"d:@"debug";stop<never>:(){d.panic("stop")};q:&(stop().missing)"#;
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    assert_eq!(
        plan.edges,
        [Edge::new(
            Port::Entry(id),
            Port::Entry(plan.parent),
            Route::Next
        )]
    );
    let checker = check("<R>:<{n<int32>}>;get<&R>:(p<&R>){->p};r<R>:{->n:1};q:&(get(&r).n)");
    let plan = checker.projections.values().next().unwrap();
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(plan.parent, call.point);
    assert!(call.edges.contains(&Edge::new(
        Port::Operation(call.point),
        Port::Normal(call.point),
        Route::Returned
    )));
    assert!(
        !plan
            .edges
            .iter()
            .any(|edge| edge.from == Port::Entry(plan.parent))
    );
}

#[test]
pub(crate) fn projection_edges_publish_atomically_without_allocating_source_ids() {
    let mut checker = check("r:{->n:1};p:&r;q:&(p.n)");
    let (&id, plan) = checker.projections.first_key_value().unwrap();
    let plan = plan.clone();
    let ids = (checker.points.len(), checker.reborrows);
    let count = checker.projection_edges;
    checker.capture_projection(id, plan.clone()).unwrap();
    assert_eq!(checker.projection_edges, count);
    assert_eq!((checker.points.len(), checker.reborrows), ids);
    checker.projections.clear();
    checker.projection_items = 0;
    checker.projection_edges = 0;
    checker.points[plan.parent].parent = None;
    assert!(
        checker
            .capture_projection(id, plan.clone())
            .unwrap_err()
            .message
            .contains("identity")
    );
    checker.points[plan.parent].parent = Some(id);
    checker.reborrow_edges = super::super::edges::MAX_EDGES;
    assert!(
        checker
            .capture_projection(id, plan)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert!(checker.projections.is_empty());
    assert_eq!((checker.projection_items, checker.projection_edges), (0, 0));
    assert_eq!((checker.points.len(), checker.reborrows), ids);
}
