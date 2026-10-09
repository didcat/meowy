use super::{tests::INT, *};
use crate::check::dependencies::edges::forward::consumers::tests::checked;

#[test]
pub(crate) fn dispatch_shared_primary_slots_keep_exact_referents_wrappers_and_owners() {
    for source in [
        "n:1;p:&n;x:p.{->$;->tag:true}==p;f<boolean>:(p<&int32>){->p.{->$;->tag:false}!=p}",
        "n<uint8>:1;p:&n;x:((p.{->$;->tag:true}))==p",
        "n<float32>:1.5;p:&n;x:p==((p.{->$;->tag:true}))",
        "n:true;p:&n;x:(p.{->$;->tag:true}~<{-><&boolean>;tag<boolean>}>)==p",
        "n:1;p:&n;x:((p.{->$;->tag:true}~<{-><&int32>;tag<boolean>}>))!=p",
        "n:\"cat\";p:&n;x:p.{->$;->tag:true}==p",
        "n:null;p:&n;x:p.{->$;->tag:true}==p",
    ] {
        let (mut checker, reports) = checked(source);
        let ops: Vec<_> = checker.binaries.values().cloned().collect();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for op in ops {
            for (step, &input) in op.inputs.iter().enumerate() {
                if !op.plan.primary[step] {
                    continue;
                }
                let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                    panic!()
                };
                let slot = checker
                    .dispatch_primary_shape(
                        &reports,
                        input,
                        op.owner,
                        Shape::SharedScalar(kind),
                        Span::default(),
                    )
                    .unwrap()
                    .unwrap_or_else(|| panic!("{source}: input {input}"));
                assert_eq!(slot.index, 0);
                let body = &checker.bodies[&slot.block];
                assert_eq!(body.owner, op.owner);
                let Layout::Slots(slots) = &body.layout else {
                    panic!()
                };
                assert_eq!(slots[0].shape, Shape::SharedScalar(kind));
                let result = &reports.results[&slot.block].1;
                assert!(result.dispatch.is_some() && result.consumer.is_none());
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn dispatch_shared_primary_slots_require_observed_results_and_exact_types() {
    for fault in 0..9 {
        let (mut checker, mut reports) = checked("n:1;p:&n;r:p.{->$;->tag:true}");
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let block = op.block;
        let mut ty = Shape::SharedScalar(INT);
        match fault {
            0 => ty = Shape::SharedScalar(ScalarKind::Bool),
            1 => {
                ty = Shape::SharedScalar(ScalarKind::Int {
                    bits: 16,
                    signed: true,
                })
            }
            2 => {
                ty = Shape::SharedScalar(ScalarKind::Int {
                    bits: 32,
                    signed: false,
                })
            }
            3 => reports.results.get_mut(&block).unwrap().0 += 1,
            4 => reports.results.get_mut(&block).unwrap().1.dispatch = None,
            5 => checker.bodies.get_mut(&block).unwrap().owner += 1,
            6..=8 => {
                let Layout::Slots(slots) = &mut checker.bodies.get_mut(&block).unwrap().layout
                else {
                    panic!()
                };
                match fault {
                    6 => slots[0].mutable = true,
                    7 => slots[0].field = Some("wrong".into()),
                    8 => slots[0].shape = Shape::SharedScalar(ScalarKind::Bool),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .dispatch_primary_shape(&reports, id, 0, ty, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let (mut checker, mut reports) = checked("n:1;p:&n;r:p.{->$;->tag:true}");
    let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
    let block = op.block;
    for initialized in [false, true] {
        for result in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            assert_eq!(
                checker
                    .dispatch_primary_shape(
                        &reports,
                        id,
                        0,
                        Shape::SharedScalar(INT),
                        Span::default()
                    )
                    .unwrap(),
                result.then_some(Slot { block, index: 0 })
            );
        }
    }
    reports.results.remove(&block);
    assert_eq!(
        checker
            .dispatch_primary_shape(&reports, id, 0, Shape::SharedScalar(INT), Span::default())
            .unwrap(),
        None
    );
}

#[test]
pub(crate) fn dispatch_shared_primary_slots_keep_stops_and_unsupported_shapes_opaque() {
    for source in [
        "n:1;p:&n;r:p.{->$}",
        "r:1.{->$;->tag:true}",
        "n:[1];p:&n;r:p.{->$;->tag:true}",
        "n:{->x:1};p:&n;r:p.{->$;->tag:true}",
        "n:1;p:&n;q:&p;r:q.{->$;->tag:true}",
        "n<int32><null>:1;p:&n;r:p.{->$;->tag:true}",
        "m:@\"memory\";n:m.heap;p:&n;r:p.{->$;->tag:true}",
        "d:@\"debug\";n:1;p:&n;r:p.{->$;->tag:true;d.panic(\"stop\")}",
        "d:@\"debug\";r:d.panic(\"stop\").{->tag:true}",
    ] {
        let (mut checker, reports) = checked(source);
        let id = *checker.dispatch_ops.keys().next().unwrap();
        assert_eq!(
            checker
                .dispatch_primary_shape(&reports, id, 0, Shape::SharedScalar(INT), Span::default())
                .unwrap(),
            None,
            "{source}"
        );
    }
    for value in ["r", "((r))", "r~<{-><&int32>;tag<boolean>}>"] {
        let source = format!("n:1;p:&n;r:p.{{->$;->tag:true}};x:({value})==p");
        let (mut checker, reports) = checked(&source);
        let input = checker.binaries.values().next().unwrap().inputs[0];
        assert_eq!(
            checker
                .dispatch_primary_shape(
                    &reports,
                    input,
                    0,
                    Shape::SharedScalar(INT),
                    Span::default()
                )
                .unwrap(),
            None
        );
    }
}
