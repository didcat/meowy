use super::{tests::checked, *};
use crate::check::dependencies::edges::forward::results::Sources;
use std::collections::BTreeSet;

mod boundaries;
mod limits;

#[test]
pub(crate) fn shared_local_binaries_link_original_slots_at_both_operand_positions() {
    for init in [
        "{->p;->a:true;->z:3}",
        "p.{->$;->a:true;->z:3}",
        "3.{->p;->a:true;->z:3}",
    ] {
        let source = format!(
            "n:1;p:&n;r:{init};alias:((r~<{{-><&int32>;a<boolean>;z<int32>}}>));left:alias==p;right:p!=alias;same:r==alias;f<boolean>:(p<&int32>){{r:{init};->p==r}}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source);
        let mut steps = BTreeSet::new();
        let mut owners = BTreeSet::new();
        assert_eq!(reports.slot_uses.len(), 3);
        for (&port, &(owner, slot)) in &reports.slot_uses {
            let Port::Projection { point: id, step } = port else {
                panic!()
            };
            let (_, Effect::Binary(op)) = &reports.effects[&id] else {
                panic!()
            };
            assert!(op.projected[step]);
            assert_eq!(op.fields[step], Some(2));
            assert_eq!(slot.index, 0);
            let BinaryClass::SharedScalar(kind) = op.types.inputs[step] else {
                panic!()
            };
            let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                panic!()
            };
            assert_eq!(slots.len(), 3);
            assert_eq!(slots[0].shape, Shape::SharedScalar(kind));
            assert_eq!(
                reports.results[&slot.block].1.slots.as_ref().unwrap()[0],
                Sources::Unknown
            );
            steps.insert(step);
            owners.insert(owner);
        }
        assert_eq!(steps, BTreeSet::from([0, 1]));
        assert_eq!(owners, BTreeSet::from([0, 1]));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_binaries_keep_source_projection_operation_and_result_independent() {
    for (tail, step) in [("r==p", 0), ("p!=r", 1)] {
        let (mut checker, mut reports) =
            checked(&format!("n:1;p:&n;r:p.{{->$;->tag:true}};{tail}"));
        let id = *checker.binaries.first_key_value().unwrap().0;
        let port = Port::Projection { point: id, step };
        let slot = reports.slot_uses[&port].1;
        let source = reports.results[&slot.block].1.dispatch.unwrap();
        let expected = reports.slot_uses.clone();
        for initialized in [false, true] {
            for published in [false, true] {
                let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                    panic!()
                };
                op.initialized = initialized;
                op.result = published;
                for (projected, operation, result) in [
                    (true, false, false),
                    (true, true, true),
                    (false, true, false),
                    (false, false, true),
                ] {
                    let (_, Effect::Binary(op)) = reports.effects.get_mut(&id).unwrap() else {
                        panic!()
                    };
                    op.projected[step] = projected;
                    op.operation = operation;
                    op.result = result;
                    if !operation && !result {
                        reports.index.operations.remove(&id);
                    } else {
                        reports.index.operations.insert(id, 0);
                    }
                    assert_eq!(
                        checker.slot_uses(&reports, Span::default()).unwrap(),
                        if projected && published {
                            expected.clone()
                        } else {
                            Uses::new()
                        }
                    );
                }
            }
        }
        reports.index.operations.remove(&id);
        assert!(
            checker
                .slot_uses(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(reports.slot_uses, expected);
    }
}

#[test]
pub(crate) fn shared_local_binaries_preserve_stopped_prefixes_without_duplicate_coercion_ports() {
    let (checker, reports) =
        checked("d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};r==d.panic(\"stop\")");
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.plan.normal, [true; 2]);
    let (_, Effect::Binary(op)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(op.projected, [true, false]);
    assert!(!op.operation && !op.result);
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(
        reports
            .slot_uses
            .contains_key(&Port::Projection { point: id, step: 0 })
    );
    let (checker, reports) = checked("n:1;p:&n;out:{->p;->tag:true}.{p!=(($))}");
    assert_eq!(reports.slot_uses.len(), 1);
    assert!(reports.slot_uses.keys().all(|port| matches!(port,
        Port::Projection { point, step: 0 } if checker.coercions.contains_key(point)
    )));
    assert!(checker.binaries.values().all(|op| op.fields == [None; 2]));
}
