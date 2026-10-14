use super::{boundaries::SOURCE, *};
use crate::check::dependencies::edges::forward::results::inputs::graph::Visit;

#[test]
pub(crate) fn shared_receiver_copy_fields_bound_maps_result_roots_and_exact_work() {
    let (mut checker, mut reports) = checked(SOURCE);
    let limit = reports.effects.len()
        + reports.blocks.len()
        + reports.results.len()
        + reports.consumers.len()
        + reports.eligible.len()
        + reports.initializers.len()
        + reports.slot_uses.len();
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    assert!(roots > 0);
    let parts = reports.parts;
    for kind in 0..3 {
        reports.parts = [0, roots, parts][kind];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let run = |checker: &mut Checker| -> Result<()> {
            match kind {
                0 => assert_eq!(
                    checker.slot_uses_limited(&reports, Span::default(), limit)?,
                    reports.slot_uses
                ),
                1 => assert_eq!(
                    checker.field_results(&reports, Span::default())?,
                    (reports.field_results.clone(), 0)
                ),
                2 => assert_eq!(
                    checker.direct_walk_report(&reports, Span::default())?.0,
                    reports.expanded_walk
                ),
                _ => unreachable!(),
            }
            Ok(())
        };
        checker.flow = crate::flow::Flow::new();
        run(&mut checker).unwrap();
        let work = checker.flow.work;
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = run(&mut checker);
            assert_eq!(result.is_ok(), short == 0, "kind {kind}");
            if short == 1 {
                assert!(result.unwrap_err().message.contains("budget"));
            }
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
    checker.flow = crate::flow::Flow::new();
    reports.parts = roots - 1;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .slot_uses_limited(&reports, Span::default(), limit - 1)
            .unwrap_err()
            .message
            .contains("budget")
    );
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
pub(crate) fn shared_receiver_copy_fields_bound_alternating_receiver_and_local_hops() {
    let mut base = None;
    for groups in [0, 4, 8] {
        let source = format!(
            "n:1;p:&n;r:{{->p;->tag:true}};out:r.{{first:$;inner:first.{{second:$;alias:{}second{};->alias.tag}}}}",
            "(".repeat(groups),
            ")".repeat(groups)
        );
        let (mut checker, reports) = checked(&source);
        let (&id, op) = checker.fields.first_key_value().unwrap();
        let input = op.input;
        let target = grouped::Target::SharedField(SharedRecord {
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

#[test]
pub(crate) fn shared_receiver_copy_fields_keep_binding_read_source_receiver_and_field_control_independent()
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
        for (&id, op) in &mut checker.fields {
            op.control = flags & 16 != 0;
            let (_, Effect::Field { control, .. }) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            *control = op.control;
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
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
