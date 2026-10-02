use super::{super::tests::checked, *};

#[test]
pub(crate) fn field_effects_reject_invalid_bounds_roots_and_load_edges_atomically() {
    for source in ["r:{->n:1};x:r.n", "r:{->n:1};p:&r;x:p.n"] {
        for fault in 0..26 {
            let (mut checker, mut reports) = checked(source, false);
            let (&id, field) = checker.fields.first_key_value().unwrap();
            let input = field.input;
            let op = field.edges.len() - 2;
            let before = reports.effects.clone();
            match fault {
                0 => checker.fields.get_mut(&id).unwrap().count = 0,
                1 => checker.fields.get_mut(&id).unwrap().index = usize::MAX,
                2 => checker.fields.get_mut(&id).unwrap().input = usize::MAX,
                3 => checker.fields.get_mut(&id).unwrap().owner = 9,
                4 => checker.points[id].owner = 9,
                5 => checker.points[id].complete = false,
                6 => checker.points[id].kind = PointKind::Stmt,
                7 => checker.points[id].span = Span::default(),
                8 => checker.points[input].parent = None,
                9 => checker.points[input].owner = 9,
                10 => checker.points[input].block = None,
                11 => checker.points[input].complete = false,
                12 => checker.points[input].kind = PointKind::Stmt,
                13 => checker.points[input].parent = Some(input),
                14 => checker.fields.get_mut(&id).unwrap().edges[0].to = Port::Normal(input),
                15 => checker.fields.get_mut(&id).unwrap().edges[1].from = Port::Entry(input),
                16 => checker.fields.get_mut(&id).unwrap().edges[op + 1].route = Route::Returned,
                17 => {
                    let field = checker.fields.get_mut(&id).unwrap();
                    field.load = !field.load;
                }
                18 => checker.fields.get_mut(&id).unwrap().normal = false,
                19 => checker.fields.get_mut(&id).unwrap().edges.clear(),
                20 => checker.fields.get_mut(&id).unwrap().edges[op].to = Port::Normal(id),
                21 => checker.fields.get_mut(&id).unwrap().edges[1].route = Route::Backedge,
                22 => {
                    reports.index.operations.remove(&id);
                }
                23 => {
                    reports.index.operations.insert(id, 9);
                }
                24 => checker.fields.get_mut(&id).unwrap().span = Span::default(),
                25 => checker.points.truncate(id),
                _ => unreachable!(),
            }
            let counts = checker.edge_counts();
            let fields = checker.fields.clone();
            assert_eq!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .code,
                "B001",
                "{source}, fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.fields, fields);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn field_effects_deduplicate_visits_and_bound_exact_work() {
    let (mut checker, mut reports) = checked("r:{->n:1};p:&r;x:p.n;y:r.n", false);
    let id = *checker.fields.first_key_value().unwrap().0;
    reports
        .entries
        .get_mut(&0)
        .unwrap()
        .1
        .ports
        .extend([Port::Operation(id); 3]);
    let expected = reports.effects.clone();
    let before = checker.flow.work;
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        expected
    );
    let work = checker.flow.work - before;
    let counts = checker.edge_counts();
    let fields = checker.fields.clone();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.operation_effects(&reports, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.fields, fields);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn field_effects_obey_effect_limits_without_variable_payload_copies() {
    for spare in [0, 1] {
        let (mut checker, reports) = checked("r:{->n:1};p:&r;x:p.n;y:r.n", false);
        let expected = reports.effects.clone();
        let counts = checker.edge_counts();
        let result = checker.operation_effects_limited(
            &reports,
            Span::default(),
            expected.len() - spare,
            4,
            0,
        );
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
            assert_eq!(
                effects
                    .values()
                    .filter(|(_, effect)| matches!(effect, Effect::Field { .. }))
                    .count(),
                2
            );
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn field_effects_keep_borrows_list_loads_and_original_errors_separate() {
    for source in ["r:{->n:1};p:r.&n", "xs:[1];p:&xs;x:p[1]"] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.fields.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Field { .. }))
        );
    }
    for (source, code) in [
        ("r:{->n:1};x:r.missing", "E201"),
        ("d:@\"debug\";v:d.panic(\"stop\").n", "E201"),
        ("r:{->n:=1};p:&r;r.n=2;x:p.n", "E302"),
        ("p:&({->n:1});x:p.n", "E303"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
