use super::{super::tests::checked, *};
use crate::{
    check::dependencies::{ScalarKind, bodies::completion::Shape},
    hir,
};

#[test]
pub(crate) fn output_reports_retain_shapes_at_independent_projection_output_and_prefix_stages() {
    for method in ["print", "panic"] {
        let source = format!("d:@\"debug\";r:3.{{->$;->tag:true}};d.{method}(\"a{{r}}b{{1}}\")");
        let (mut checker, reports) = checked(&source, false);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let source = op.parts[1].unwrap().source;
        for port in [
            Port::Projection { point: id, step: 1 },
            Port::Output { point: id, part: 1 },
            Port::Operation(id),
        ] {
            let stage = checker
                .output_effect_stage(0, port, Span::default())
                .unwrap()
                .unwrap();
            let mut effects = Effects::new();
            let mut room = 1;
            for _ in 0..2 {
                checker
                    .record_output_effect(stage, &mut effects, 1, &mut room, Span::default())
                    .unwrap();
            }
            let (_, Effect::Output(output)) = &effects[&id] else {
                panic!()
            };
            assert!(!output.prefix);
            assert_eq!(output.terminal, port == Port::Operation(id));
            if port != Port::Operation(id) {
                assert_eq!(room, 0);
                assert_eq!(output.parts.len(), 1);
                let part = &output.parts[&1];
                assert_eq!(part.input.unwrap().source, source);
                assert_eq!(part.projection, matches!(port, Port::Projection { .. }));
                assert_eq!(part.output, matches!(port, Port::Output { .. }));
            }
            checker
                .validate_output_report(&reports, id, 0, output, Span::default())
                .unwrap();
        }
        if method == "panic" {
            let stage = checker
                .output_effect_stage(0, Port::Prefix(id), Span::default())
                .unwrap()
                .unwrap();
            let mut effects = Effects::new();
            checker
                .record_output_effect(stage, &mut effects, 1, &mut 0, Span::default())
                .unwrap();
            let (_, Effect::Output(output)) = &effects[&id] else {
                panic!()
            };
            assert!(output.prefix && !output.terminal && output.parts.is_empty());
            checker
                .validate_output_report(&reports, id, 0, output, Span::default())
                .unwrap();
        }
    }
}

#[test]
pub(crate) fn output_reports_reject_shape_conflicts_and_validate_unobserved_stopped_suffixes() {
    for fault in 0..8 {
        let (mut checker, reports) = checked(
            "d:@\"debug\";r:3.{->$;->tag:true};d.print(\"{r}{1}\")",
            false,
        );
        let id = *checker.outputs.keys().next().unwrap();
        let Effect::Output(mut output) = reports.effects[&id].1.clone() else {
            panic!()
        };
        let bad = match fault {
            0 => None,
            1 => Some(Shape::Never),
            2 => Some(Shape::Scalar(ScalarKind::Int {
                bits: 7,
                signed: true,
            })),
            3 => Some(Shape::Union { members: 1 }),
            4 => Some(Shape::Reference(hir::ReferenceMode::Shared)),
            _ => Some(Shape::Scalar(ScalarKind::Bool)),
        };
        if fault < 5 || fault == 6 {
            checker.outputs.get_mut(&id).unwrap().parts[0]
                .as_mut()
                .unwrap()
                .source = bad;
        }
        if fault < 6 {
            output
                .parts
                .get_mut(&0)
                .unwrap()
                .input
                .as_mut()
                .unwrap()
                .source = bad;
        }
        if fault == 7 {
            checker.outputs.get_mut(&id).unwrap().parts[1]
                .as_mut()
                .unwrap()
                .source = bad;
        }
        let before = format!("{reports:?}{:?}", checker.outputs);
        assert!(
            checker
                .validate_output_report(&reports, id, 0, &output, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.outputs), before);
    }
    let source = "d:@\"debug\";f<null>:(r<{-><never>;tag<boolean>}>){d.panic(\"a{r}tail{r}\")}";
    let (mut checker, reports) = checked(source, false);
    let (&id, op) = checker.outputs.first_key_value().unwrap();
    let owner = op.owner;
    let (_, Effect::Output(output)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(output.prefix && !output.terminal && !output.parts.contains_key(&3));
    checker
        .validate_output_report(&reports, id, owner, output, Span::default())
        .unwrap();
    checker.outputs.get_mut(&id).unwrap().parts[3]
        .as_mut()
        .unwrap()
        .source = None;
    assert!(
        checker
            .validate_output_report(&reports, id, owner, output, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
}

#[test]
pub(crate) fn output_reports_reject_changed_merge_shapes_and_bound_exact_work() {
    let (mut checker, _) = checked("d:@\"debug\";r:3.{->$;->tag:true};d.print(r)", false);
    let id = *checker.outputs.keys().next().unwrap();
    let stage = checker
        .output_effect_stage(0, Port::Projection { point: id, step: 0 }, Span::default())
        .unwrap()
        .unwrap();
    let Kind::Projection { input, .. } = stage.kind else {
        panic!()
    };
    let mut effects = Effects::new();
    let mut room = 1;
    checker
        .record_output_effect(stage, &mut effects, 1, &mut room, Span::default())
        .unwrap();
    let expected = effects.clone();
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
    ] {
        let changed = Stage {
            kind: Kind::Projection {
                part: 0,
                input: FormatInput { source, ..input },
            },
            ..stage
        };
        assert!(
            checker
                .record_output_effect(changed, &mut effects, 1, &mut room, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, expected);
        assert_eq!(room, 0);
    }
    let start = checker.flow.work;
    let mut fresh = Effects::new();
    checker
        .record_output_effect(stage, &mut fresh, 1, &mut 1, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let mut fresh = Effects::new();
        let mut room = 1;
        let result = checker.record_output_effect(stage, &mut fresh, 1, &mut room, Span::default());
        if short == 0 {
            result.unwrap();
            assert_eq!(fresh, expected);
            assert_eq!(room, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
            assert!(fresh.is_empty());
            assert_eq!(room, 1);
        }
    }
}
