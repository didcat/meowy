use super::*;

#[test]
pub(crate) fn equality_context_reports_keep_reference_projection_sources_and_order() {
    for (body, dispatch) in [
        ("{->p;->tag:true}", false),
        ("(({->p;->tag:true}))", false),
        ("p.{->$;->tag:true}", true),
        ("((p.{->$;->tag:true}))", true),
    ] {
        let source =
            format!("n:1;p:&n;x:p=={body};y:{body}!=p;f<boolean>:(p<&int32>){{->p=={body}}}");
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        assert_eq!(checker.binaries.len(), 3);
        for (&id, op) in &checker.binaries {
            assert_eq!(
                op.types.inputs,
                [Class::Reference(crate::hir::ReferenceMode::Shared); 2]
            );
            assert!(op.plan.equality && !op.plan.checked);
            assert_eq!(op.plan.primary.iter().filter(|&&value| value).count(), 1);
            let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(observed.inputs, op.inputs);
            assert_eq!(observed.projected, op.plan.primary);
            assert!(observed.operation && observed.result);
            for (step, &input) in op.inputs.iter().enumerate() {
                assert_eq!(checker.points[input].parent, Some(id));
                let slot = reports.slot_uses.get(&Port::Projection { point: id, step });
                if op.plan.primary[step] && !dispatch {
                    let &(owner, slot) = slot.unwrap();
                    assert_eq!((owner, slot.index), (op.owner, 0));
                    let crate::check::dependencies::bodies::Layout::Slots(slots) =
                        &checker.bodies[&slot.block].layout
                    else {
                        panic!()
                    };
                    assert!(slots[0].field.is_none());
                    assert_eq!(
                        slots[0].shape,
                        crate::check::dependencies::bodies::completion::Shape::Reference(
                            crate::hir::ReferenceMode::Shared
                        )
                    );
                    assert!(reports.results[&slot.block].1.dispatch.is_none());
                } else {
                    assert!(slot.is_none());
                }
            }
            assert_eq!(
                op.edges.last(),
                Some(&Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Next
                ))
            );
        }
        assert!(checker.binaries.values().any(|op| op.owner == 0));
        assert!(checker.binaries.values().any(|op| op.owner != 0));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap(),
            reports.effects
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn equality_context_reports_keep_list_shapes_and_stopped_boundaries() {
    let source = "a<uint8[3]>:[1];b<uint8[3]>:[1,2];x:a==((0.{->[1,2]}));r:{->a;->tag:true};y:r=={->b;->tag:false}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let classes: Vec<_> = checker
        .binaries
        .values()
        .map(|op| op.types.inputs[0])
        .collect();
    assert_eq!(
        classes,
        [Class::List { capacity: 3 }, Class::Record { fields: 1 }]
    );
    for (&id, op) in &checker.binaries {
        let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(observed.inputs, op.inputs);
        assert_eq!(observed.types, op.types);
        assert_eq!(observed.projected, [false; 2]);
        assert!(observed.operation && observed.result);
    }
    for source in [
        "d:@\"debug\";v:[1];x:v==(({d.panic(\"stop\");->[1];->tag:true}))",
        "d:@\"debug\";n:1;p:&n;x:p==((p.{d.panic(\"stop\");->$;->tag:true}))",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        assert_eq!(op.plan.normal, [true, false]);
        assert!(!op.plan.equality);
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(!reports.entries[&op.owner].1.ports.contains(&port));
        }
    }
}

#[test]
pub(crate) fn equality_context_rejections_publish_no_mismatched_binary_plan() {
    for body in ["r", "((r))", "r.{->(($))}", "{->((r))}"] {
        let source = format!("v:[1];r:{{->[1];->tag:true}};good:v==v;bad:v=={body}");
        let mut checker = Checker::new();
        let error = checker
            .block(&crate::parser::parse(&source).unwrap(), None, None)
            .unwrap_err();
        assert_eq!(error.code, "E222");
        assert_eq!(checker.binaries.len(), 1);
        let (_, op) = checker.binaries.first_key_value().unwrap();
        assert_eq!(op.plan.primary, [false; 2]);
        assert!(checker.point.is_none());
    }
}
