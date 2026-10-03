use super::{super::super::tests::checked, *};

#[test]
pub(crate) fn output_edges_preserve_print_panic_parts_and_stopped_suffixes() {
    for method in ["print", "panic"] {
        for body in [
            format!("d.{method}(\"\")"),
            format!("d.{method}(\"text\")"),
            format!("d.{method}(\"a{{({{->7;->tag:true}})}}b{{1}}\")"),
            format!("f<null>:(r<{{-><never>;tag<boolean>}}> ){{d.{method}(\"a{{r}}tail{{1}}\")}}"),
            format!("stop<never>:(){{d.panic(\"stop\")}};d.{method}(\"a{{stop()}}tail{{1}}\")"),
        ] {
            let (mut checker, reports) = checked(&format!("d:@\"debug\";{body}"), false);
            let outputs = checker.outputs.clone();
            let effects = reports.effects.clone();
            let counts = checker.edge_counts();
            for (&id, op) in &outputs {
                checker
                    .validate_output_edges(id, op.owner, Span::default())
                    .unwrap();
            }
            assert_eq!(checker.outputs, outputs);
            assert_eq!(reports.effects, effects);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn output_edges_reject_exact_sequence_and_identity_corruption_atomically() {
    for fault in 0..21 {
        let source = "d:@\"debug\";f<null>:(r<{-><never>;tag<boolean>}>){d.panic(\"a{r}tail{1}\")}";
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.outputs.first_key_value().unwrap();
        let (owner, input, suffix) = (
            op.owner,
            op.parts[1].unwrap().point,
            op.parts[3].unwrap().point,
        );
        let op = checker.outputs.get_mut(&id).unwrap();
        match fault {
            0 => op.edges.clear(),
            1 => op.edges[0].route = Route::Returned,
            2 => op.edges.swap(0, 1),
            3 => op.edges.push(Edge::new(
                Port::Operation(id),
                Port::Normal(id),
                Route::Returned,
            )),
            4 => op.edges.last_mut().unwrap().to = Port::Projection { point: id, step: 2 },
            5 => op.owner += 1,
            6 => checker.points[id].owner += 1,
            7 => checker.points[id].complete = false,
            8 => checker.points[id].kind = PointKind::Stmt,
            9 => checker.points[id].span = Span::default(),
            10 => op.stopped = Some(0),
            11 => op.stopped = Some(usize::MAX),
            12 => checker.points[input].parent = None,
            13 => checker.points[input].block = None,
            14 => checker.points[suffix].complete = false,
            15 => checker.points[suffix].kind = PointKind::Stmt,
            16 => checker.points[suffix].owner += 1,
            17 => op.parts[3].as_mut().unwrap().point = usize::MAX,
            18 => op.parts[1].as_mut().unwrap().point = id,
            19 => op.panic = false,
            20 => op.stopped = None,
            _ => unreachable!(),
        }
        let outputs = checker.outputs.clone();
        let effects = reports.effects.clone();
        let counts = checker.edge_counts();
        let error = checker
            .validate_output_edges(id, owner, Span::default())
            .unwrap_err();
        assert!(error.message.contains("identity mismatch"), "fault {fault}");
        assert_eq!(checker.outputs, outputs);
        assert_eq!(reports.effects, effects);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn output_edges_bound_parts_edges_and_exact_work() {
    let (mut checker, _) = checked("d:@\"debug\";d.print(\"a{1}\")", false);
    let id = *checker.outputs.first_key_value().unwrap().0;
    let outputs = checker.outputs.clone();
    let counts = checker.edge_counts();
    let start = checker.flow.work;
    checker
        .validate_output_edges(id, 0, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        assert_eq!(
            checker
                .validate_output_edges(id, 0, Span::default())
                .is_ok(),
            spare == 0
        );
        assert_eq!(checker.outputs, outputs);
        assert_eq!(checker.edge_counts(), counts);
    }
    for edges in [false, true] {
        checker.flow = crate::flow::Flow::new();
        checker.outputs = outputs.clone();
        let op = checker.outputs.get_mut(&id).unwrap();
        let limit = crate::check::dependencies::sequences::MAX_ITEMS;
        if edges {
            op.edges.resize(
                limit * 3 + 3,
                Edge::new(Port::Entry(id), Port::Normal(id), Route::Next),
            );
        } else {
            op.parts.resize(limit + 1, None);
        }
        assert!(
            checker
                .validate_output_edges(id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
}
