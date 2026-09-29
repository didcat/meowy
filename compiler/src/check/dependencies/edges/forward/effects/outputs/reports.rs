use super::{super::tests::checked, *};

#[test]
pub(crate) fn output_effects_collect_aliases_literals_dynamic_parts_and_primary_stages() {
    for method in ["print", "panic"] {
        let source = format!(
            "flag:false;d:@\"debug\";show:d.{method};|flag|(\"a{{({{->7;->tag:true}})}}b{{1}}\").(show)"
        );
        let (checker, reports) = checked(&source, true);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let (owner, Effect::Output(output)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert!(output.control);
        assert_eq!(output.panic, method == "panic");
        assert_eq!(output.prefix, output.panic);
        assert!(output.terminal);
        assert_eq!(output.parts.len(), 4);
        assert_eq!(output.total, 4);
        assert_eq!(output.stopped, None);
        for (&part, found) in &output.parts {
            assert_eq!(found.input, op.parts[part]);
            assert!(found.output);
            assert_eq!(found.projection, part == 1);
        }
    }
    for method in ["print", "panic"] {
        let (_, reports) = checked(&format!("d:@\"debug\";d.{method}(\"\")"), false);
        assert!(
            reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Output(output) if output.terminal))
        );
    }
}

#[test]
pub(crate) fn output_effects_keep_partial_prefixes_and_never_primary_projections() {
    for method in ["print", "panic"] {
        let source = format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};d.{method}(\"a{{stop()}}tail{{1}}\")"
        );
        let (checker, reports) = checked(&source, false);
        let (&id, _) = checker
            .outputs
            .iter()
            .find(|(_, output)| output.owner == 0)
            .unwrap();
        let Effect::Output(output) = &reports.effects[&id].1 else {
            panic!()
        };
        assert_eq!(output.prefix, method == "panic");
        assert!(!output.terminal);
        assert_eq!(output.stopped, Some(1));
        assert_eq!(output.parts.keys().copied().collect::<Vec<_>>(), [0]);
        assert!(!reports.index.operations.contains_key(&id));
        let source = format!(
            "d:@\"debug\";f<null>:(r<{{-><never>;tag<boolean>}}> ){{d.{method}(\"a{{r}}tail{{1}}\")}}"
        );
        let (checker, reports) = checked(&source, false);
        let (&id, _) = checker.outputs.first_key_value().unwrap();
        let (owner, Effect::Output(output)) = &reports.effects[&id] else {
            panic!()
        };
        assert_ne!(*owner, 0);
        assert_eq!(output.parts.len(), 2);
        assert!(output.parts[&1].projection);
        assert!(!output.parts[&1].output);
        assert!(!output.terminal);
    }
}

#[test]
pub(crate) fn output_effects_bound_part_copies_deduplicate_visits_and_preserve_reports() {
    let (mut checker, mut reports) =
        checked("d:@\"debug\";d.print(\"{({->7;->tag:true})}\")", false);
    let id = *checker.outputs.first_key_value().unwrap().0;
    reports.entries.get_mut(&0).unwrap().1.ports.extend([
        Port::Projection { point: id, step: 0 },
        Port::Output { point: id, part: 0 },
        Port::Operation(id),
    ]);
    let expected = reports.effects.clone();
    assert_eq!(
        checker
            .operation_effects_limited(&reports, Span::default(), MAX_EDGES, 1, MAX_EDGES)
            .unwrap(),
        expected
    );
    assert!(
        checker
            .operation_effects_limited(&reports, Span::default(), MAX_EDGES, 0, MAX_EDGES)
            .is_err()
    );
    let before = checker.flow.work;
    checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.operation_effects(&reports, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn output_effects_fail_before_aggregation_changes_for_limits_or_conflicts() {
    let (mut checker, _) = checked("d:@\"debug\";d.print(\"a{1}\")", false);
    let id = *checker.outputs.first_key_value().unwrap().0;
    let stage = checker
        .output_effect_stage(0, Port::Output { point: id, part: 0 }, Span::default())
        .unwrap()
        .unwrap();
    let mut effects = BTreeMap::new();
    let mut parts = 0;
    assert!(
        checker
            .record_output_effect(stage, &mut effects, 1, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    parts = 1;
    assert!(
        checker
            .record_output_effect(stage, &mut effects, 0, &mut parts, Span::default())
            .is_err()
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 1);
    checker
        .record_output_effect(stage, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    let before = effects.clone();
    assert_eq!(parts, 0);
    checker
        .record_output_effect(stage, &mut effects, 1, &mut parts, Span::default())
        .unwrap();
    for changed in [
        Stage { owner: 9, ..stage },
        Stage {
            panic: true,
            ..stage
        },
        Stage {
            control: true,
            ..stage
        },
        Stage {
            total: stage.total + 1,
            ..stage
        },
        Stage {
            stopped: Some(1),
            ..stage
        },
        Stage {
            kind: Kind::Part {
                part: 0,
                input: checker.outputs[&id].parts[1],
            },
            ..stage
        },
    ] {
        assert!(
            checker
                .record_output_effect(changed, &mut effects, 1, &mut parts, Span::default())
                .is_err()
        );
        assert_eq!(effects, before);
        assert_eq!(parts, 0);
    }
}
