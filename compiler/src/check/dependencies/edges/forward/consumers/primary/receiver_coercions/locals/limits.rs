use super::{boundaries::SOURCE, *};
use crate::check::dependencies::operations::SharedRecord;

#[test]
pub(crate) fn shared_local_receiver_coercions_bound_maps_work_and_duplicate_visits() {
    let (mut checker, mut reports) = checked(SOURCE);
    let effects = reports.effects.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        effects
    );
    let expected = reports.slot_uses.clone();
    assert_eq!(expected.len(), 2);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + expected.len();
    reports.parts = 0;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - start;
    assert!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.slot_uses_limited(&reports, Span::default(), limit);
        if short == 0 {
            assert_eq!(result.unwrap(), expected);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn shared_local_receiver_coercions_keep_bind_read_source_receiver_and_conversion_control_independent()
 {
    let (mut checker, mut reports) = checked(SOURCE);
    for flags in 0..32 {
        for (&id, op) in &mut checker.operations {
            op.control = flags & 1 != 0;
            if let Some((_, Effect::Storage { control, .. })) = reports.effects.get_mut(&id) {
                *control = op.control;
            }
        }
        for (&id, op) in &mut checker.local_reads {
            op.control = flags & 2 != 0;
            if let Some((_, Effect::Read { control, .. })) = reports.effects.get_mut(&id) {
                *control = op.control;
            }
        }
        for (&id, op) in &mut checker.dispatch_ops {
            op.control = flags & if op.shared_primary.is_some() { 4 } else { 8 } != 0;
            let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            observed.control = op.control;
        }
        for (&id, op) in &mut checker.coercions {
            op.control = flags & 16 != 0;
            if let Some((_, Effect::Coercion(observed))) = reports.effects.get_mut(&id) {
                observed.control = op.control;
            }
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            reports.slot_uses
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_local_receiver_coercions_share_exact_hops_across_nested_receivers_and_copies()
{
    let mut base = None;
    for groups in [0, 4, 8] {
        let source = format!(
            "n:1;p:&n;r:{{->p;->tag:true}};alias:{}r{};out:alias.{{inner:$.{{copy<&int32><null>:$}}}}",
            "(".repeat(groups),
            ")".repeat(groups)
        );
        let (mut checker, reports) = checked(&source);
        let (&id, op) = checker.coercions.iter().find(|(_, op)| op.primary).unwrap();
        let Some(Shape::SharedScalar(primary)) = op.source else {
            panic!()
        };
        let input = op.input;
        let target = grouped::Target::SharedCopies(SharedRecord {
            primary,
            fields: op.fields.unwrap(),
        });
        let block = reports.slot_uses[&Port::Projection { point: id, step: 0 }]
            .1
            .block;
        let expected = reports.results[&block].1.consumer;
        assert!(expected.is_some());
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut minimum = None;
        for limit in 0..64 {
            match checker.grouped_source_limited(&reports, input, 0, Span::default(), limit, target)
            {
                Ok(actual) => {
                    assert_eq!(actual, expected);
                    minimum = Some(limit);
                    break;
                }
                Err(error) => assert!(error.message.contains("budget")),
            }
        }
        let hops = minimum.unwrap();
        assert_eq!(hops, *base.get_or_insert(hops) + groups);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
