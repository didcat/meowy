use super::{super::tests::checked, *};

#[test]
pub(crate) fn projection_effects_keep_steps_modes_counts_and_conditional_parents() {
    for source in [
        "r:{->n:1};h:{->p:&r};q:&(h.p.n)",
        "r:{->n:1};h:{->p:&r};m:{->v:&h};p:&m;q:&(p.v.p.n)",
        "x:*(&({->n:1}.n))",
        "rows:[{->n:1}];q:&(rows[1].n)",
        "<R>:<{n<int32>}>;make<R>:(){->n:1};x:*(&(make().n))",
        "<R>:<{n<int32>}>;get<&R>:(p<&R>){->p};r<R>:{->n:1};q:&(get(&r).n)",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.projections.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                op.owner,
                Effect::Projection(Observed {
                    parent: op.parent,
                    steps: op.steps.clone(),
                    site: op.site.unwrap(),
                    parent_mode: op.mode.unwrap(),
                    control: false,
                    projected: vec![true; op.steps.len()],
                    converted: op
                        .steps
                        .iter()
                        .map(|step| matches!(step, ProjectionStep::Field { narrow: true, .. }))
                        .collect(),
                    acquired: true,
                    result: true,
                })
            )
        );
        assert!(!checker.reborrow_ops.contains_key(&id));
        for call in checker.invocations.values() {
            assert_eq!(call.point, op.parent);
            assert_eq!(call.edges.last().unwrap().route, Route::Returned);
        }
    }
}

#[test]
pub(crate) fn projection_effects_keep_step_conversion_acquisition_and_result_visits_independent() {
    let source = "<R>:<{n<int32>}>;<H>:<{p<&R><null>}>;r<R>:{->n:1};h:{->inner<H><null>:{->p:&r}};|h.inner<H>|{|h.inner.p<&R>|q:&(h.inner.p.n)}";
    crate::compile(source).unwrap();
    let (mut checker, mut reports) = checked(source, false);
    let (&id, op) = checker.projections.first_key_value().unwrap();
    assert_eq!(op.steps.len(), 3);
    let (_, Effect::Projection(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.converted, [true, true, false]);
    for port in (0..3)
        .map(|step| Port::Projection { point: id, step })
        .chain([
            Port::Conversion { point: id, part: 0 },
            Port::Conversion { point: id, part: 1 },
            Port::Operation(id),
            Port::Normal(id),
        ])
    {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 9, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Projection(op)) = &effects[&id] else {
            panic!()
        };
        for (step, &seen) in op.projected.iter().enumerate() {
            assert_eq!(seen, port == Port::Projection { point: id, step });
        }
        for (part, &seen) in op.converted.iter().enumerate() {
            assert_eq!(seen, port == Port::Conversion { point: id, part });
        }
        assert_eq!(op.acquired, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn projection_effects_preserve_nested_loads_owners_and_control() {
    let source = "flag:false;<R>:<{n<int32>}>;r<R>:{->n:1};h:{->p:&r};v:&h;|flag|q:&(v.p.n);f:(p<&R>){q:&(p.n)}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.projections.len(), 2);
    assert!(checker.projections.values().any(|op| op.control));
    assert!(checker.projections.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.projections {
        let (owner, Effect::Projection(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.site, op.site.unwrap());
        assert_eq!(observed.steps, op.steps);
        assert!(observed.acquired && observed.result);
    }
}
