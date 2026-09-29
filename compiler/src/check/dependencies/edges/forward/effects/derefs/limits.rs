use super::{super::tests::checked, *};

#[test]
pub(crate) fn deref_effects_reject_invalid_points_owners_and_edges_atomically() {
    for fault in 0..22 {
        let (mut checker, mut reports) = checked("n:1;x:*(&n)", false);
        let (&id, deref) = checker.derefs.first_key_value().unwrap();
        let input = deref.input;
        let before = reports.effects.clone();
        match fault {
            0 => checker.derefs.get_mut(&id).unwrap().owner = 9,
            1 => checker.points[id].owner = 9,
            2 => checker.points[id].complete = false,
            3 => checker.points[id].kind = PointKind::Stmt,
            4 => checker.points[id].span = Span::default(),
            5 => checker.derefs.get_mut(&id).unwrap().input = usize::MAX,
            6 => checker.points[input].parent = None,
            7 => checker.points[input].owner = 9,
            8 => checker.points[input].block = None,
            9 => checker.points[input].complete = false,
            10 => checker.points[input].kind = PointKind::Stmt,
            11 => checker.derefs.get_mut(&id).unwrap().normal = false,
            12 => checker.derefs.get_mut(&id).unwrap().edges.clear(),
            13 => checker.derefs.get_mut(&id).unwrap().edges[0].to = Port::Normal(input),
            14 => checker.derefs.get_mut(&id).unwrap().edges[1].from = Port::Entry(input),
            15 => checker.derefs.get_mut(&id).unwrap().edges[1].to = Port::Normal(id),
            16 => checker.derefs.get_mut(&id).unwrap().edges[2].route = Route::Returned,
            17 => checker.derefs.get_mut(&id).unwrap().edges[1].route = Route::Backedge,
            18 => {
                reports.index.operations.remove(&id);
            }
            19 => {
                reports.index.operations.insert(id, 9);
            }
            20 => checker.derefs.get_mut(&id).unwrap().span = Span::default(),
            21 => checker.points.truncate(id),
            _ => unreachable!(),
        }
        let counts = checker.edge_counts();
        let derefs = checker.derefs.clone();
        assert_eq!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .code,
            "B001",
            "fault {fault}"
        );
        assert_eq!(reports.effects, before);
        assert_eq!(checker.derefs, derefs);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn deref_effects_deduplicate_visits_and_bound_exact_work() {
    let (mut checker, mut reports) = checked("n:1;p:&n;x:*p", false);
    let id = *checker.derefs.first_key_value().unwrap().0;
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
    let derefs = checker.derefs.clone();
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.operation_effects(&reports, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.derefs, derefs);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn deref_effects_obey_effect_limits_without_variable_payload_copies() {
    for spare in [0, 1] {
        let (mut checker, reports) = checked("n:1;p:&n;x:*p;y:*p", false);
        let expected = reports.effects.clone();
        let counts = checker.edge_counts();
        let result = checker.operation_effects_limited(
            &reports,
            Span::default(),
            expected.len() - spare,
            0,
            0,
        );
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(effects) = result {
            assert_eq!(effects, expected);
            assert_eq!(
                effects
                    .values()
                    .filter(|(_, effect)| matches!(effect, Effect::Deref { .. }))
                    .count(),
                2
            );
        }
        assert_eq!(reports.effects, expected);
        assert_eq!(checker.edge_counts(), counts);
    }
}

#[test]
pub(crate) fn deref_effects_keep_implicit_loads_and_reborrows_separate() {
    for source in [
        "row:{->n:1};p:&row;x:p.n",
        "xs:[1];p:&xs;x:p[1]",
        "n:=1;p:&!n;q:&(*p)",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.derefs.is_empty());
        assert!(
            !reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Deref { .. }))
        );
    }
    for (source, code) in [
        ("x:*1", "E222"),
        ("n:=1;p:&n;n=2;x:*p", "E302"),
        ("p:&(1+2);x:*p", "E303"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}
