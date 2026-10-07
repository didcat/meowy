use super::{super::tests::checked, *};
use std::collections::BTreeSet;

mod boundaries;

#[test]
pub(crate) fn record_receiver_fields_keep_source_slots_wrappers_and_independent_owners() {
    for (source, dispatch) in [("{->z:9;->a:true}", false), ("3.{->z:9;->a:true}", true)] {
        let source = format!(
            "r:{source};out:r.{{alias:(($));inner:$.{{->$.z}};nested:{{->alias.a}};->$.z}};f<boolean>:(){{r:{source};->r.{{->$.a}}}}"
        );
        let (mut checker, reports) = checked(&source);
        let mut owners = BTreeSet::new();
        assert_eq!(reports.slot_uses.len(), checker.fields.len());
        for (&id, op) in &checker.fields {
            let (owner, slot) = reports.slot_uses[&Port::Operation(id)];
            assert_eq!((owner, slot.index), (op.owner, op.index + 1));
            assert_eq!(reports.results[&slot.block].1.dispatch.is_some(), dispatch);
            assert_eq!(reports.field_results[&Port::Normal(id)], (owner, slot));
            owners.insert(owner);
        }
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
pub(crate) fn record_receiver_fields_require_initialization_and_field_operations_independently() {
    let (mut checker, mut reports) = checked("r:{->n:1};out:r.{->$.n}");
    let id = *checker.fields.keys().next().unwrap();
    let dispatch = *checker.dispatch_ops.keys().next().unwrap();
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 1);
    for initialized in [false, true] {
        for completed in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = completed;
            for (operation, result) in [(true, false), (true, true), (false, true)] {
                let (
                    _,
                    Effect::Field {
                        operation: seen,
                        result: done,
                        ..
                    },
                ) = reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *seen = operation;
                *done = result;
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    if operation && initialized {
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
pub(crate) fn record_receiver_fields_preserve_guarded_scopes_and_reads_before_stopped_bodies() {
    for source in [
        "r:{->n:1};out:r.{inner:{->($.n)}}",
        "r:{->n:1};flag:=true;out:r.{|flag|->$.n;|!flag|->$.n}",
        "d:@\"debug\";r:{->n:1};out:r.{copy:$.n;d.panic(\"stop\")}",
    ] {
        let (checker, reports) = checked(source);
        assert_eq!(reports.slot_uses.len(), checker.fields.len());
        for (&id, op) in &checker.fields {
            assert!(!op.load);
            assert!(reports.slot_uses.contains_key(&Port::Operation(id)));
            assert!(reports.field_results.contains_key(&Port::Normal(id)));
        }
    }
}

#[test]
pub(crate) fn record_receiver_fields_keep_ineligible_load_call_parameter_and_aggregate_paths_opaque()
 {
    for source in [
        "r:{->n:=1};out:r.{->$.n}",
        "base:1;r:{->n:2;->p:&base};out:r.{->$.n}",
        "r:{->n:1};out:(&r).{->$.n}",
        "r:={->n:1};out:r.{->$.n}",
        "r:{->n:1};p:&r;out:(*p).{->$.n}",
        "get<{n<int32>}>:(){->n:1};out:get().{->$.n}",
        "f<int32>:(r<{n<int32>}>){->r.{->$.n}}",
        "r:{->inner:{->n:1}};out:r.{->$.inner}",
        "r:{->xs:[1]};out:r.{->$.xs}",
    ] {
        let (_, reports) = checked(source);
        assert!(reports.slot_uses.is_empty(), "{source}");
        assert!(reports.field_results.is_empty(), "{source}");
    }
}

#[test]
pub(crate) fn record_receiver_fields_keep_consumer_ports_distinct() {
    let source =
        "d:@\"debug\";r:{->3;->n:4};out:r.{d.print($);a:$+1;xs<int32[1]>:[$];copy:{->$};->$.n}";
    let (checker, reports) = checked(source);
    assert_eq!(reports.slot_uses.len(), 6);
    let (&port, _) = reports
        .slot_uses
        .iter()
        .find(|(port, _)| matches!(port, Port::Operation(_)))
        .unwrap();
    let Port::Operation(id) = port else { panic!() };
    assert!(checker.fields.contains_key(&id));
    for op in checker
        .emissions
        .values()
        .filter(|op| op.composed.is_some())
    {
        for (index, target) in op.targets.iter().enumerate() {
            assert_eq!(reports.slot_uses[&Port::Emission(target.id)].1.index, index);
        }
    }
    let (&port, _) = reports
        .slot_uses
        .iter()
        .find(|(port, _)| matches!(port,
            Port::Projection { point, .. } if matches!(reports.effects[point].1, Effect::Coercion(_))))
        .unwrap();
    let Port::Projection { point, step: 0 } = port else {
        panic!()
    };
    assert!(matches!(reports.effects[&point].1, Effect::Coercion(_)));
    for &id in checker.binaries.keys().chain(checker.outputs.keys()) {
        assert_eq!(
            reports.slot_uses[&Port::Projection { point: id, step: 0 }]
                .1
                .index,
            0
        );
    }
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Coercion(op) if op.primary))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Output(_)))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, op)| matches!(op, Effect::Emission(op) if op.composed.is_some()))
    );
}
