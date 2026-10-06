use super::{super::tests::checked, *};
use crate::check::dependencies::edges::forward::results::{Sources, inputs::graph::Visit};
use std::collections::BTreeSet;

mod boundaries;
mod limits;

#[test]
pub(crate) fn record_dispatch_fields_preserve_slots_owners_and_expanded_field_visits() {
    let source = "r:3.{->z:$;->a:4};copy:((r~<{z<int32>;a<int32>}>));out:{->copy.z;->again:r.a};f<int32>:(){r:4.{->{->n:$}};->r.n}";
    let (mut checker, reports) = checked(source);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    for (&id, field) in &checker.fields {
        let (owner, slot) = reports.slot_uses[&Port::Operation(id)];
        assert_eq!(owner, field.owner);
        assert_eq!(slot.index, field.index + 1);
        assert_eq!(reports.field_results[&Port::Normal(id)], (owner, slot));
        let row = &reports.results[&slot.block].1;
        let dispatch = row.dispatch.unwrap();
        assert_eq!(checker.dispatch_ops[&dispatch].block, slot.block);
        assert!(row.consumer.is_none() && !reports.consumers.contains_key(&dispatch));
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    let mut linked = 0;
    for (&key, &(owner, direct)) in &reports.direct_sources {
        let Some(source) = direct.source else {
            continue;
        };
        let input = reports.candidate_inputs[&key].1;
        assert_eq!(direct.point, input.point);
        assert!(direct.block.is_none() && direct.dispatch.is_none());
        assert_eq!(
            reports.field_results[&Port::Normal(source.field)],
            (owner, source.slot)
        );
        assert!(
            reports
                .candidate_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        assert!(
            reports
                .expanded_walk
                .visits
                .contains(&Visit::Field(key, input, source))
        );
        linked += 1;
    }
    assert_eq!(linked, 3);
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
    );
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn record_dispatch_fields_keep_operation_result_and_initialization_independent() {
    let (mut checker, mut reports) = checked("r:3.{->n:1};x:r.n");
    let id = *checker.fields.keys().next().unwrap();
    let slot = reports.slot_uses[&Port::Operation(id)].1;
    let dispatch = reports.results[&slot.block].1.dispatch.unwrap();
    for initialized in [true, false] {
        for completed in [true, false] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = completed;
            for (operation, result) in [(true, true), (true, false), (false, true)] {
                let (
                    _,
                    Effect::Field {
                        operation: op,
                        result: value,
                        ..
                    },
                ) = reports.effects.get_mut(&id).unwrap()
                else {
                    panic!()
                };
                *op = operation;
                *value = result;
                assert_eq!(
                    checker
                        .field_slot(&reports, id, 0, &reports.effects[&id].1, Span::default())
                        .unwrap(),
                    (operation && completed).then_some(slot)
                );
                reports.slot_uses = checker.slot_uses(&reports, Span::default()).unwrap();
                assert_eq!(
                    checker
                        .field_result_slot(
                            &reports,
                            id,
                            0,
                            &reports.effects[&id].1,
                            Span::default()
                        )
                        .unwrap(),
                    (operation && result && completed).then_some(slot)
                );
            }
        }
    }
}

#[test]
pub(crate) fn record_dispatch_fields_keep_mutable_histories_unknown_and_nonscalars_opaque() {
    let (_, reports) = checked("n:3.{->n:=1}.n;out:{->n}");
    let slot = reports.slot_uses.values().next().unwrap().1;
    assert_eq!(
        reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index],
        Sources::Unknown
    );
    assert!(
        reports
            .expanded_walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Unknown(root) if root.slot == slot))
    );
    for source in [
        "r:3.{->n:$};p:&r;x:p.n",
        "r:3.{->n:$};p:&r;x:(*p).n",
        "f<{n<int32>}>:(){->3.{->n:$}};x:f().n",
        "n:(3.{->n:{->a:1}}).n",
        "n:(3.{->n:[$]}).n",
        "n:1;x:2.{->n:&n}.n",
        "v<int32><null>:1;n:3.{->n:v}.n",
        "r:3.{->n:=1};x:r.n",
    ] {
        let (_, reports) = checked(source);
        assert!(
            reports
                .slot_uses
                .keys()
                .all(|port| matches!(port, Port::Emission(_))),
            "{source}"
        );
        assert!(reports.field_results.is_empty(), "{source}");
    }
}
