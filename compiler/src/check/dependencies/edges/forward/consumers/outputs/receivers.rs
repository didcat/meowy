use super::*;

mod boundaries;
mod stages;

#[test]
pub(crate) fn receiver_output_primaries_keep_sparse_parts_shapes_and_independent_owners() {
    for (init, dispatch) in [("{->3;->tag:true}", false), ("3.{->$;->tag:true}", true)] {
        for method in ["print", "panic"] {
            let source = format!(
                "d:@\"debug\";r:{init};out:r.{{alias:(($));inner:$.{{d.{method}(\"a{{alias}}b{{1}}c{{(($))}}d{{alias}}e\")}}}};f<null>:(){{r:{init};out:r.{{d.print($)}}}}"
            );
            let (mut checker, reports) = checked(&source);
            assert_eq!(reports.slot_uses.len(), 4);
            let mut steps = BTreeMap::new();
            for (&port, &(owner, slot)) in &reports.slot_uses {
                let Port::Projection { point, step } = port else {
                    panic!()
                };
                let (_, Effect::Output(output)) = &reports.effects[&point] else {
                    panic!()
                };
                let part = &output.parts[&step];
                let input = part.input.unwrap();
                assert!(part.projection && input.primary);
                assert_eq!(slot.index, 0);
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(input.source, Some(slots[0].shape));
                assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
                assert_eq!(checker.outputs[&point].owner, owner);
                assert_eq!(
                    checker
                        .grouped_consumer(&reports, input.point, owner, Span::default())
                        .unwrap(),
                    None
                );
                assert_eq!(
                    checker
                        .dispatch_consumer(&reports, input.point, owner, Span::default())
                        .unwrap(),
                    None
                );
                steps.entry(owner).or_insert_with(Vec::new).push(step);
            }
            assert_eq!(steps[&0], [1, 5, 7]);
            assert_eq!(steps[&1], [0]);
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert_eq!(
                checker.slot_uses(&reports, Span::default()).unwrap(),
                reports.slot_uses
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn receiver_output_primaries_keep_scalar_kinds_and_guarded_scopes() {
    for (ty, value) in [
        ("uint8", "7"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let source =
            format!("d:@\"debug\";n<{ty}>:{value};r:n.{{->$;->tag:true}};out:r.{{d.print($)}}");
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
    }
    let (checker, reports) = checked(
        "d:@\"debug\";flag:=true;r:{->3;->tag:true};out:r.{|flag|d.print($);|!flag|$.{d.print($)}}",
    );
    assert_eq!(reports.slot_uses.len(), 2);
    for &id in checker.outputs.keys() {
        assert!(
            reports
                .slot_uses
                .contains_key(&Port::Projection { point: id, step: 0 })
        );
    }
}
