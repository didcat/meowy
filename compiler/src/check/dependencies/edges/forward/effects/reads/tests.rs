use super::{super::tests::checked, *};

#[test]
pub(crate) fn read_effects_preserve_alias_storage_control_and_parameter_owners() {
    let source = "flag:false;row:'out{|flag|{'out->n:1;copy:n};|!flag|{'out->n:2;copy:n}};f<int32>:(v<int32>){->v}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert!(checker.locals.is_empty());
    let mut aliases = Vec::new();
    for (&id, read) in &checker.local_reads {
        assert_eq!(
            reports.effects[&id],
            (
                read.owner,
                Effect::Read {
                    local: read.local,
                    storage: read.storage,
                    normal: read.normal,
                    control: read.control
                }
            )
        );
        if checker.proofs.aliases.contains_key(&read.local) {
            aliases.push((read.local, read.storage));
        }
    }
    assert_eq!(aliases.len(), 2);
    assert_ne!(aliases[0].0, aliases[1].0);
    assert_eq!(aliases[0].1, aliases[1].1);
    assert!(
        reports
            .effects
            .values()
            .any(|(owner, effect)| *owner != 0 && matches!(effect, Effect::Read { .. }))
    );
    let (_, reports) = checked("flag:false;n:1;|flag|copy:n", true);
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Read { control: true, .. }))
    );
}

#[test]
pub(crate) fn read_effects_keep_reference_cells_separate_from_pointee_loads() {
    let (checker, reports) = checked("n:=1;p:&n;q:p;copy:*q", false);
    let (&deref, load) = checker.derefs.first_key_value().unwrap();
    let raw = checker.narrowings[&load.input].input;
    let read = &checker.local_reads[&raw];
    let pointee = checker
        .place_borrows
        .first_key_value()
        .unwrap()
        .1
        .place
        .root;
    assert_ne!(read.storage, pointee);
    assert!(
        matches!(reports.effects[&raw].1, Effect::Read { storage, .. } if storage == read.local)
    );
    assert_eq!(reports.effects[&deref].1, Effect::Unknown);
}

#[test]
pub(crate) fn read_effects_keep_never_stopped_and_required_only_boundaries() {
    let (checker, reports) = checked("f<never>:(v<never>){->v}", false);
    let (&id, _) = checker.local_reads.first_key_value().unwrap();
    assert!(matches!(
        reports.effects[&id].1,
        Effect::Read { normal: false, .. }
    ));
    let (checker, reports) = checked(
        "n:1;stop<never>:(){'loop{'loop.restart()}};stop();copy:n",
        false,
    );
    assert_eq!(checker.local_reads.len(), 1);
    for id in checker.local_reads.keys() {
        assert!(!reports.effects.contains_key(id));
    }
    let (checker, reports) = checked("n:3;<T>:{size:n;-><uint8[size]>};values<T>:[7]", false);
    assert!(checker.local_reads.is_empty());
    assert!(checker.body_inputs.values().flatten().next().is_some());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Read { .. }))
    );
}

#[test]
pub(crate) fn read_effects_reject_invalid_bounds_aliases_points_and_edges_atomically() {
    for fault in 0..14 {
        let (mut checker, mut reports) = checked("row:{->n:1;copy:n}", false);
        let (&id, read) = checker.local_reads.first_key_value().unwrap();
        let local = read.local;
        let before = reports.effects.clone();
        match fault {
            0 => reports.locals = 0,
            1 => checker.local_reads.get_mut(&id).unwrap().local = usize::MAX,
            2 => checker.local_reads.get_mut(&id).unwrap().storage = usize::MAX,
            3 => checker.local_reads.get_mut(&id).unwrap().storage = reports.locals - 1,
            4 => checker.proofs.aliases.get_mut(&local).unwrap().root = reports.locals - 1,
            5 => checker.local_reads.get_mut(&id).unwrap().owner = 9,
            6 => checker.points[id].owner = 9,
            7 => checker.points[id].complete = false,
            8 => checker.points[id].kind = PointKind::Stmt,
            9 => checker.points[id].span = Span::default(),
            10 => checker.local_reads.get_mut(&id).unwrap().normal = false,
            11 => {
                checker
                    .local_reads
                    .get_mut(&id)
                    .unwrap()
                    .edges
                    .last_mut()
                    .unwrap()
                    .route = Route::Returned
            }
            12 => checker.local_reads.get_mut(&id).unwrap().edges.clear(),
            13 => {
                reports.index.operations.remove(&id);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .code,
            "B001",
            "fault {fault}"
        );
        assert_eq!(reports.effects, before);
    }
}

#[test]
pub(crate) fn read_effects_deduplicate_visits_and_bound_exact_work() {
    let (mut checker, mut reports) = checked("n:1;copy:n", false);
    let id = *checker.local_reads.first_key_value().unwrap().0;
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
