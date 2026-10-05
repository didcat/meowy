use super::{super::super::super::tests::checked, *};

#[test]
pub(crate) fn field_narrowing_rejects_overlapping_producers_without_stored_associations() {
    for fault in 0..6 {
        for observed in [false, true] {
            let (mut checker, mut reports) = checked("r:{->n:1};x:(r.n);v<int32>:7;t:1~<int32>");
            let id = *checker.fields.first_key_value().unwrap().0;
            match fault {
                0 => {
                    checker
                        .narrowings
                        .insert(id, checker.narrowings.first_key_value().unwrap().1.clone());
                }
                1 => {
                    checker
                        .group_inputs
                        .insert(id, *checker.group_inputs.first_key_value().unwrap().1);
                }
                2 => {
                    checker
                        .coercions
                        .insert(id, checker.coercions.first_key_value().unwrap().1.clone());
                }
                3 => {
                    checker
                        .local_reads
                        .insert(id, checker.local_reads.first_key_value().unwrap().1.clone());
                }
                4 => {
                    checker
                        .typed_ops
                        .insert(id, checker.typed_ops.first_key_value().unwrap().1.clone());
                }
                5 => {
                    reports
                        .consumers
                        .insert(id, *reports.consumers.first_key_value().unwrap().1);
                }
                _ => unreachable!(),
            }
            if !observed {
                reports.field_results.remove(&Port::Normal(id));
            }
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            let error = checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    id,
                    0,
                    Span::default(),
                    MAX_GROUPS,
                )
                .unwrap_err();
            assert!(
                error.message.contains("identity"),
                "{fault}, {observed}: {error:?}"
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}

#[test]
pub(crate) fn seeded_field_narrowing_cycles_stop_after_individually_valid_edges() {
    let (mut checker, mut reports) = checked("r:{->n:1};x:r.n");
    let (&id, op) = checker
        .narrowings
        .iter()
        .find(|(_, op)| checker.fields.contains_key(&op.input))
        .unwrap();
    let field = op.input;
    let mut child = op.clone();
    child.input = id;
    child.edges = vec![
        Edge::new(Port::Entry(field), Port::Entry(id), Route::Next),
        Edge::new(Port::Normal(id), Port::Normal(field), Route::Next),
    ];
    checker.points[id].parent = Some(field);
    checker.points[field].span = child.span;
    checker.fields.remove(&field);
    checker.narrowings.insert(field, child);
    reports.field_results.remove(&Port::Normal(field));
    let (owner, mut effect) = reports.effects[&id].clone();
    let Effect::Narrowing(op) = &mut effect else {
        panic!()
    };
    op.input = id;
    reports.effects.insert(field, (owner, effect));
    for (start, next) in [(id, field), (field, id)] {
        assert_eq!(
            checker
                .unchanged_narrowing_input(&reports, start, owner, Span::default())
                .unwrap(),
            Some(next)
        );
        let error = checker
            .field_narrowing_source(
                &mut Lookup::new(&reports, MAX_EDGES),
                start,
                owner,
                Span::default(),
                MAX_GROUPS,
            )
            .unwrap_err();
        assert!(error.message.contains("field-narrowing identity"));
    }
}

#[test]
pub(crate) fn field_narrowing_bounds_exact_work_and_rejects_late_operand_corruption() {
    let (mut checker, reports) = checked("r:{->n:1};x:r.n");
    let (&id, _) = checker
        .narrowings
        .iter()
        .find(|(_, op)| checker.fields.contains_key(&op.input))
        .unwrap();
    let work = checker.flow.work;
    let expected = checker
        .field_narrowing_source(
            &mut Lookup::new(&reports, MAX_EDGES),
            id,
            0,
            Span::default(),
            1,
        )
        .unwrap();
    let work = checker.flow.work - work;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_narrowing_source(
            &mut Lookup::new(&reports, MAX_EDGES),
            id,
            0,
            Span::default(),
            1,
        );
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(result) = result {
            assert_eq!(result, expected);
        }
    }
    for fault in 0..3 {
        let (mut checker, reports) = checked("r:{->n:1};x:r.n");
        let (&id, op) = checker
            .narrowings
            .iter()
            .find(|(_, op)| checker.fields.contains_key(&op.input))
            .unwrap();
        let field = op.input;
        match fault {
            0 => checker.points[field].parent = None,
            1 => checker.points[field].complete = false,
            2 => checker.fields.get_mut(&field).unwrap().count = 0,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    id,
                    0,
                    Span::default(),
                    1
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
