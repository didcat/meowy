use super::*;

mod boundaries;

#[test]
pub(crate) fn dispatch_output_primaries_keep_original_parts_shapes_and_independent_owners() {
    for method in ["print", "panic"] {
        let source = format!(
            "d:@\"debug\";r:3.{{->$;->tag:true}};copy:((r));d.{method}(\"a{{copy}}b{{1}}c{{(4.{{->$;->tag:false}})}}\");f<null>:(){{r:true.{{->$;->tag:true}};d.print(r)}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 3);
        let mut steps = BTreeMap::new();
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point, step } = port else {
                panic!()
            };
            let (_, Effect::Output(output)) = &reports.effects[&point] else {
                panic!()
            };
            let part = &output.parts[&step];
            assert!(part.projection && part.input.unwrap().primary);
            assert_eq!(slot.index, 0);
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(part.input.unwrap().source, Some(slots[0].shape));
            let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
            assert_eq!(checker.dispatch_ops[&dispatch].owner, owner);
            assert!(!reports.consumers.contains_key(&dispatch));
            steps.entry(owner).or_insert_with(Vec::new).push(step);
        }
        assert_eq!(steps[&0], [1, 5]);
        assert_eq!(steps[&1], [0]);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for setup in [
        "n<uint8>:3;r:n.{->$;->tag:true}",
        "n<float32>:1.5;r:n.{->$;->tag:true}",
        "r:\"cat\".{->$;->tag:true}",
        "r:3.{->tag:true}",
    ] {
        let (_, reports) = checked(&format!("d:@\"debug\";{setup};d.print(r)"));
        assert_eq!(reports.slot_uses.len(), 1);
    }
}

#[test]
pub(crate) fn dispatch_output_primaries_keep_sparse_visits_and_source_result_independent() {
    let (mut checker, mut reports) =
        checked("d:@\"debug\";r:3.{->$;->tag:true};d.panic(\"a{r}b\")");
    let id = *checker.outputs.keys().next().unwrap();
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    let Effect::Output(full) = reports.effects[&id].1.clone() else {
        panic!()
    };
    assert_eq!(expected.len(), 1);
    for initialized in [false, true] {
        for result in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            for state in 0..4 {
                let mut output = full.clone();
                output.prefix = state == 3;
                output.terminal = state == 2;
                output.parts.retain(|&part, _| part == 1 && state < 2);
                if let Some(part) = output.parts.get_mut(&1) {
                    part.projection = state == 0;
                    part.output = state == 1;
                }
                reports.effects.insert(id, (0, Effect::Output(output)));
                reports.index.operations.remove(&id);
                if state == 2 {
                    reports.index.operations.insert(id, 0);
                }
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    if state == 0 && result {
                        expected.clone()
                    } else {
                        Uses::new()
                    }
                );
            }
        }
    }
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn dispatch_output_primaries_preserve_prefix_links_before_stopped_operands() {
    for method in ["print", "panic"] {
        let source = format!(
            "d:@\"debug\";r:3.{{->$;->tag:true}};d.{method}(\"a{{r}}b{{d.panic(\"stop\")}}tail{{r}}\")"
        );
        let (checker, reports) = checked(&source);
        let (&id, op) = checker
            .outputs
            .iter()
            .find(|(_, op)| op.parts.len() > 1)
            .unwrap();
        let (_, Effect::Output(output)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(op.stopped, Some(3));
        assert_eq!(output.prefix, method == "panic");
        assert!(!output.terminal && !output.parts.contains_key(&5));
        assert!(op.parts[5].unwrap().source.is_some());
        assert_eq!(reports.slot_uses.len(), 1);
        assert!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 1 })
        );
    }
}

#[test]
pub(crate) fn dispatch_output_primaries_keep_opaque_shapes_and_inner_coercion_ports_separate() {
    for source in [
        "r:=3.{->$;->tag:true};d.print(r)",
        "r:3.{->$;->tag:true};p:&r;d.print(*p)",
        "f<{-><int32>;tag<boolean>}>:(){->3;->tag:true};d.print(f())",
        "f<null>:(r<{-><int32>;tag<boolean>}>){d.print(r)}",
        "f<null>:(r<{-><never>;tag<boolean>}>){d.print(r)}",
        "n<int32><null>:1;r:3.{->n;->tag:true};d.print(r)",
    ] {
        let (_, reports) = checked(&format!("d:@\"debug\";{source}"));
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (checker, reports) =
        checked("d:@\"debug\";r:3.{->$;->tag:true};d.print(-r);xs<int32[1]>:[r];out:{->r}");
    assert_eq!(reports.slot_uses.len(), 2);
    for port in reports.slot_uses.keys() {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        assert!(matches!(reports.effects[point].1, Effect::Coercion(_)));
        assert!(!checker.outputs.contains_key(point));
    }
}
