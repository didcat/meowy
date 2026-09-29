use super::{super::tests::checked, *};

#[test]
pub(crate) fn output_effect_stages_keep_literal_dynamic_primary_and_alias_identity() {
    for method in ["print", "panic"] {
        let source =
            format!("d:@\"debug\";show:d.{method};(\"a{{({{->7;->tag:true}})}}b{{1}}\").(show)");
        let (mut checker, _) = checked(&source, false);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let owner = op.owner;
        let input = op.parts[1].unwrap();
        for (port, expected) in [
            (
                Port::Output { point: id, part: 0 },
                Kind::Part {
                    part: 0,
                    input: None,
                },
            ),
            (
                Port::Projection { point: id, step: 1 },
                Kind::Projection { part: 1, input },
            ),
            (
                Port::Output { point: id, part: 1 },
                Kind::Part {
                    part: 1,
                    input: Some(input),
                },
            ),
            (Port::Operation(id), Kind::Finish),
        ] {
            let stage = checker
                .output_effect_stage(owner, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.point, id);
            assert_eq!(stage.kind, expected);
            assert_eq!(stage.panic, method == "panic");
            assert_eq!(stage.total, 4);
            assert_eq!(stage.stopped, None);
        }
        assert_eq!(
            checker
                .output_effect_stage(owner, Port::Prefix(id), Span::default())
                .is_ok(),
            method == "panic"
        );
    }
}

#[test]
pub(crate) fn output_effect_stages_keep_never_primary_projection_without_output() {
    let source = "d:@\"debug\";f<null>:(r<{-><never>;tag<boolean>}>){d.panic(\"a{r}tail{1}\")}";
    let (mut checker, _) = checked(source, false);
    let (&id, op) = checker.outputs.first_key_value().unwrap();
    let owner = op.owner;
    assert_ne!(owner, 0);
    let stage = checker
        .output_effect_stage(
            owner,
            Port::Projection { point: id, step: 1 },
            Span::default(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(stage.stopped, Some(1));
    assert!(matches!(stage.kind, Kind::Projection { part: 1, .. }));
    for port in [
        Port::Output { point: id, part: 1 },
        Port::Output { point: id, part: 2 },
        Port::Operation(id),
    ] {
        assert!(
            checker
                .output_effect_stage(owner, port, Span::default())
                .is_err()
        );
    }
}

#[test]
pub(crate) fn output_effect_stages_reject_malformed_owner_root_and_part_metadata() {
    for fault in 0..8 {
        let (mut checker, _) = checked("d:@\"debug\";d.print(\"x{1}\")", false);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let input = op.parts[1].unwrap().point;
        match fault {
            0 => checker.outputs.get_mut(&id).unwrap().owner = 1,
            1 => checker.points[id].complete = false,
            2 => checker.points[id].kind = PointKind::Stmt,
            3 => checker.outputs.get_mut(&id).unwrap().stopped = Some(0),
            4 => checker.outputs.get_mut(&id).unwrap().stopped = Some(usize::MAX),
            5 => checker.points[input].parent = None,
            6 => checker.points[input].owner = 1,
            7 => checker.points[input].complete = false,
            _ => unreachable!(),
        }
        assert_eq!(
            checker
                .output_effect_stage(0, Port::Output { point: id, part: 1 }, Span::default())
                .unwrap_err()
                .code,
            "B001"
        );
    }
}

#[test]
pub(crate) fn output_effect_stages_bound_work_and_leave_other_producers_unhandled() {
    let (mut checker, _) = checked("d:@\"debug\";d.print(1)", false);
    let id = *checker.outputs.first_key_value().unwrap().0;
    let port = Port::Output { point: id, part: 0 };
    let before = checker.flow.work;
    let expected = checker
        .output_effect_stage(0, port, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.output_effect_stage(0, port, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(stage) = result {
            assert_eq!(stage, expected);
        }
    }
    checker.flow = crate::flow::Flow::new();
    assert!(
        checker
            .output_effect_stage(0, Port::Entry(id), Span::default())
            .unwrap()
            .is_none()
    );
    assert!(
        checker
            .output_effect_stage(
                0,
                Port::Projection {
                    point: usize::MAX,
                    step: 0
                },
                Span::default()
            )
            .unwrap()
            .is_none()
    );
    assert!(
        checker
            .output_effect_stage(0, Port::Prefix(usize::MAX), Span::default())
            .is_err()
    );
    checker
        .outputs
        .get_mut(&id)
        .unwrap()
        .parts
        .resize(crate::check::dependencies::sequences::MAX_ITEMS + 1, None);
    assert!(
        checker
            .output_effect_stage(0, port, Span::default())
            .unwrap_err()
            .message
            .contains("budget")
    );
}
