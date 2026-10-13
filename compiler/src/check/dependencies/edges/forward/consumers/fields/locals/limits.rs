use super::{super::super::tests::checked, boundaries::SOURCE, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn shared_local_fields_preserve_independent_bind_read_field_control_and_source_walks() {
    let (mut checker, mut reports) = checked(SOURCE);
    assert_eq!(
        reports
            .direct_sources
            .values()
            .filter(|(_, direct)| direct.source.is_some())
            .count(),
        2
    );
    assert_eq!(
        reports
            .expanded_walk
            .visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Field(..)))
            .count(),
        2
    );
    for binding in [false, true] {
        for read in [false, true] {
            for field in [false, true] {
                for (&id, op) in &mut checker.operations {
                    op.control = binding;
                    if let Some((_, Effect::Storage { control, .. })) = reports.effects.get_mut(&id)
                    {
                        *control = binding;
                    }
                }
                for (&id, op) in &mut checker.local_reads {
                    op.control = read;
                    if let Some((_, Effect::Read { control, .. })) = reports.effects.get_mut(&id) {
                        *control = read;
                    }
                }
                for (&id, op) in &mut checker.fields {
                    op.control = field;
                    let (_, Effect::Field { control, .. }) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    *control = field;
                }
                let before = format!("{reports:?}{:?}", checker.edge_counts());
                assert_eq!(
                    checker.slot_uses(&reports, Span::default()).unwrap(),
                    reports.slot_uses
                );
                assert_eq!(
                    checker.field_results(&reports, Span::default()).unwrap().0,
                    reports.field_results
                );
                assert_eq!(
                    checker.direct_sources(&reports, Span::default()).unwrap().0,
                    reports.direct_sources
                );
                assert_eq!(
                    checker
                        .direct_walk_report(&reports, Span::default())
                        .unwrap()
                        .0,
                    reports.expanded_walk
                );
                assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
            }
        }
    }
}

#[test]
pub(crate) fn shared_local_fields_keep_source_initialization_and_field_stages_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let id = *checker.fields.keys().last().unwrap();
    let slot = reports.slot_uses[&Port::Operation(id)].1;
    let source = reports.results[&slot.block].1.dispatch.unwrap();
    let expected = reports.slot_uses.clone();
    for initialized in [false, true] {
        for published in [false, true] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&source).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = published;
            for operation in [false, true] {
                for result in [false, true] {
                    let (
                        _,
                        Effect::Field {
                            operation: observed,
                            result: completed,
                            ..
                        },
                    ) = reports.effects.get_mut(&id).unwrap()
                    else {
                        panic!()
                    };
                    *observed = operation;
                    *completed = result;
                    if !operation && !result {
                        assert!(
                            checker
                                .slot_uses(&reports, Span::default())
                                .unwrap_err()
                                .message
                                .contains("field-effect identity")
                        );
                        continue;
                    }
                    let mut wanted = expected.clone();
                    if !published || !operation {
                        wanted.remove(&Port::Operation(id));
                    }
                    reports.slot_uses = checker.slot_uses(&reports, Span::default()).unwrap();
                    assert_eq!(reports.slot_uses, wanted);
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
                        (published && operation && result).then_some(slot)
                    );
                }
            }
        }
    }
}

#[test]
pub(crate) fn shared_local_fields_bound_exact_slot_maps_roots_and_work() {
    let (mut checker, mut reports) = checked(SOURCE);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit)
            .unwrap(),
        reports.slot_uses
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
            assert_eq!(result.unwrap(), reports.slot_uses);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    checker.flow = crate::flow::Flow::new();
    reports.parts = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    assert!(reports.parts > 0);
    assert_eq!(
        checker.field_results(&reports, Span::default()).unwrap(),
        (reports.field_results.clone(), 0)
    );
    reports.parts -= 1;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .field_results(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn shared_local_fields_share_exact_hop_limits_across_copy_chains() {
    let mut base = None;
    for groups in [0, 4, 8] {
        let source = format!(
            "n:1;p:&n;r:{{->p;->tag:true}};copy:{}r{};out:copy.tag",
            "(".repeat(groups),
            ")".repeat(groups)
        );
        let (mut checker, reports) = checked(&source);
        let (&id, op) = checker.fields.first_key_value().unwrap();
        let input = op.input;
        let target = grouped::Target::SharedLocal(SharedRecord {
            primary: op.shared_primary.unwrap(),
            fields: op.count,
        });
        let block = reports.slot_uses[&Port::Operation(id)].1.block;
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
