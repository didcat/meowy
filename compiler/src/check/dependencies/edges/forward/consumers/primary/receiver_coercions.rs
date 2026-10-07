use super::{super::tests::checked, *};
use crate::check::dependencies::CoercionKind;
use std::collections::BTreeSet;

mod boundaries;

#[test]
pub(crate) fn receiver_coercion_primaries_keep_source_shapes_ports_and_independent_owners() {
    for (init, dispatch) in [("{->3;->tag:true}", false), ("3.{->$;->tag:true}", true)] {
        let source = format!(
            "r:{init};out:r.{{alias:(($));a<int32>:alias;b<int32><null>:alias;inner:$.{{c<int32>:$}}}};f<int32>:(){{r:{init};->r.{{copy<int32>:$;->copy}}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 4);
        let mut owners = BTreeSet::new();
        let mut converted = false;
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point, step: 0 } = port else {
                panic!()
            };
            let (_, Effect::Coercion(op)) = &reports.effects[&point] else {
                panic!()
            };
            assert!(op.primary && op.projected);
            assert_eq!(slot.index, 0);
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(op.source, Some(slots[0].shape));
            assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, point, owner, Span::default())
                    .unwrap(),
                None
            );
            owners.insert(owner);
            converted |= op.op == CoercionKind::Convert;
        }
        assert!(converted);
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    for (ty, value) in [
        ("uint8", "7"),
        ("float32", "1.5"),
        ("boolean", "true"),
        ("string", "\"cat\""),
        ("null", "null"),
    ] {
        let source = format!("n<{ty}>:{value};r:n.{{->$;->tag:true}};out:r.{{copy<{ty}>:$}}");
        let (_, reports) = checked(&source);
        assert_eq!(reports.slot_uses.len(), 1, "{source}");
    }
}

#[test]
pub(crate) fn receiver_coercion_primaries_keep_projection_initialization_and_source_results_independent()
 {
    let (mut checker, mut reports) = checked("r:3.{->$;->tag:true};out:r.{copy<int32><null>:$}");
    let (&port, &(_, slot)) = reports.slot_uses.first_key_value().unwrap();
    let Port::Projection { point: id, step: 0 } = port else {
        panic!()
    };
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let receiver = *checker
        .dispatch_ops
        .iter()
        .find(|(_, op)| matches!(op.receiver, Shape::Record { .. }))
        .unwrap()
        .0;
    let expected = reports.slot_uses.clone();
    for published in [false, true] {
        let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
            panic!()
        };
        op.result = published;
        op.initialized = !published;
        for initialized in [false, true] {
            for completed in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&receiver).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = completed;
                for (projected, operation, result) in [
                    (true, false, false),
                    (true, true, true),
                    (false, true, false),
                    (false, false, true),
                ] {
                    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected = projected;
                    op.operation = operation;
                    op.result = result;
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        if projected && initialized && published {
                            expected.clone()
                        } else {
                            Uses::new()
                        }
                    );
                }
            }
        }
    }
    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.projected = true;
    op.operation = false;
    op.result = false;
    reports.index.operations.remove(&id);
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        expected
    );
    let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    op.operation = true;
    assert!(
        checker
            .slot_uses(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(reports.slot_uses, expected);
}

#[test]
pub(crate) fn receiver_coercion_primaries_keep_ineligible_and_unprojected_sources_opaque() {
    for source in [
        "r:{->[3];->tag:true};out:r.{copy<int32[1]>:$}",
        "base:1;r:{->&base;->tag:true};out:r.{copy<&int32>:$}",
        "n<int32><null>:1;r:{->n;->tag:true};out:r.{copy<int32><null>:$}",
        "r:{->3;->tag:true};out:r.{copy<{-><int32>;tag<boolean>}><null>:$}",
        "r:={->3;->tag:true};out:r.{copy<int32>:$}",
        "r:{->3;->tag:=true};out:r.{copy<int32>:$}",
        "r:{->3;->tag:true};p:&r;out:(*p).{copy<int32>:$}",
        "get<{-><int32>;tag<boolean>}>:(){->3;->tag:true};out:get().{copy<int32>:$}",
        "f:(r<{-><int32>;tag<boolean>}>){out:r.{copy<int32>:$}}",
        "d:@\"debug\";r:{->3;->tag:true};out:r.{d.panic(\"stop\");copy<int32>:$}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn receiver_coercion_primaries_keep_nested_ports_and_links_before_stopped_bodies() {
    let source = "d:@\"debug\";r:{->3;->tag:true};out:r.{copy<int32>:$;d.print($);xs<int32[1]>:[$];n:-$;direct:$+1;whole:{->$};d.panic(\"stop\")}";
    let (checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 5);
    let mut coercions = 0;
    for port in reports.slot_uses.keys() {
        let Port::Projection { point, step: 0 } = port else {
            panic!()
        };
        match reports.effects[point].1 {
            Effect::Coercion(_) => {
                coercions += 1;
                assert!(!checker.outputs.contains_key(point));
            }
            Effect::Binary(_) | Effect::Output(_) => (),
            _ => panic!(),
        }
        assert!(!checker.lists.contains_key(point));
    }
    assert_eq!(coercions, 3);
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Dispatch(op) if op.initialized && !op.result))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Binary(op) if op.projected[0]))
    );
}
