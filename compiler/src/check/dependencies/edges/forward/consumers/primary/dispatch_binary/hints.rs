use super::*;
use crate::check::dependencies::SequenceSource;

#[test]
pub(crate) fn binary_record_hints_keep_primary_sources_owners_and_order() {
    for (body, dispatch) in [
        ("{->4;->tag:false}", false),
        ("((({->4;->tag:false})))", false),
        ("4.{->$;->tag:false}", true),
        ("((4.{->$;->tag:false}))", true),
    ] {
        let source = format!(
            "a:{{->3;->tag:true}};x:a+{body};y:{body}+a;f<int32>:(){{a:{{->3;->tag:true}};->a+{body}}}"
        );
        let (mut checker, reports) = checked(&source);
        assert_eq!(checker.binaries.len(), 3);
        assert_eq!(checker.dispatch_ops.len(), if dispatch { 3 } else { 0 });
        assert_eq!(reports.slot_uses.len(), 6);
        for (&id, captured) in &checker.binaries {
            let (_, Effect::Binary(op)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(op.projected, [true; 2]);
            assert!(op.operation && op.result);
            for step in 0..2 {
                let port = Port::Projection { point: id, step };
                let (owner, slot) = reports.slot_uses[&port];
                assert_eq!((owner, slot.index), (captured.owner, 0));
                assert_eq!(checker.bodies[&slot.block].owner, owner);
                let Layout::Slots(slots) = &checker.bodies[&slot.block].layout else {
                    panic!()
                };
                assert_eq!(slots.len(), 2);
                assert!(slots[0].field.is_none());
                assert_eq!(slots[1].field.as_deref(), Some("tag"));
                assert!(captured.edges.contains(&Edge::new(
                    Port::Normal(op.inputs[step]),
                    port,
                    Route::Next
                )));
            }
            assert_eq!(
                checker.sequences[&SequenceSource::Expr(id)].edges,
                [Edge::new(
                    Port::Projection { point: id, step: 0 },
                    Port::Entry(op.inputs[1]),
                    Route::Next
                )]
            );
            assert_eq!(
                captured.edges.last(),
                Some(&Edge::new(
                    Port::Operation(id),
                    Port::Normal(id),
                    Route::Checked
                ))
            );
        }
        for (&id, op) in &checker.dispatch_ops {
            let (_, Effect::Dispatch(effect)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(effect.input, op.input);
            assert_eq!(effect.block, op.block);
            assert!(effect.initialized && effect.result);
            assert!(checker.proofs.receivers.contains(&op.local));
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
pub(crate) fn binary_record_hints_keep_stopped_prefix_and_atomic_budget_limits() {
    let (mut checker, reports) =
        checked("d:@\"debug\";x:3.{->$;->tag:true}+(({d.panic(\"stop\");->4;->tag:false}))");
    let (&id, _) = checker.binaries.first_key_value().unwrap();
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
    assert_eq!(
        checker.slot_uses(&reports, Span::default()).unwrap(),
        reports.slot_uses
    );

    let (mut checker, mut reports) = checked("a:{->3;->tag:true};x:a+((4.{->$;->tag:false}))");
    let expected = reports.slot_uses.clone();
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
