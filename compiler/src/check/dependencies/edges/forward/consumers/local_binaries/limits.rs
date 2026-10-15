use super::*;
use crate::check::dependencies::operations::SharedRecord;

#[test]
pub(crate) fn shared_local_binaries_bound_maps_work_and_duplicate_visits_without_payload() {
    let (mut checker, mut reports) = checked(boundaries::SOURCE);
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
pub(crate) fn shared_local_binaries_keep_bind_read_source_and_binary_control_independent() {
    let (mut checker, mut reports) = checked(boundaries::SOURCE);
    for flags in 0..16 {
        for (&id, op) in &mut checker.operations {
            op.control = flags & 1 != 0;
            let (_, Effect::Storage { control, .. }) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            *control = op.control;
        }
        for (&id, op) in &mut checker.local_reads {
            op.control = flags & 2 != 0;
            let (_, Effect::Read { control, .. }) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            *control = op.control;
        }
        for (&id, op) in &mut checker.dispatch_ops {
            op.control = flags & 4 != 0;
            let (_, Effect::Dispatch(value)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            value.control = op.control;
        }
        for (&id, op) in &mut checker.binaries {
            op.control = flags & 8 != 0;
            let (_, Effect::Binary(value)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            value.control = op.control;
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
pub(crate) fn shared_local_binaries_share_exact_hop_limits_at_both_operand_positions() {
    for (tail, step) in [("copy==p", 0), ("p!=copy", 1)] {
        let mut base = None;
        for groups in [0, 4, 8] {
            let source = format!(
                "n:1;p:&n;r:{{->p;->tag:true}};copy:{}r{};{tail}",
                "(".repeat(groups),
                ")".repeat(groups)
            );
            let (mut checker, reports) = checked(&source);
            let (&id, op) = checker.binaries.first_key_value().unwrap();
            let BinaryClass::SharedScalar(primary) = op.types.inputs[step] else {
                panic!()
            };
            let target = grouped::Target::SharedCopies(SharedRecord {
                primary,
                fields: op.fields[step].unwrap(),
            });
            let input = op.inputs[step];
            let block = reports.slot_uses[&Port::Projection { point: id, step }]
                .1
                .block;
            let expected = reports.results[&block].1.consumer;
            assert!(expected.is_some());
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            let mut minimum = None;
            for limit in 0..64 {
                match checker.grouped_source_limited(
                    &reports,
                    input,
                    0,
                    Span::default(),
                    limit,
                    target,
                ) {
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
}
