use super::{super::tests::checked, *};

#[test]
pub(crate) fn index_effect_stages_preserve_roots_loads_lengths_and_read_boundaries() {
    for (source, load, length) in [
        ("xs<int32[3]>:[1,2];xs[1]", false, Some(2)),
        ("xs<int32[3]>:=[1,2];p:&xs;p[1]", true, None),
        ("xs<int32[3]>:[1,2];p:&xs;(*p)[1]", false, None),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.indices.first_key_value().unwrap();
        let receiver = op.receiver;
        let access = op.access.unwrap();
        for (port, kind) in [
            (Port::Snapshot(id), Kind::Snapshot),
            (Port::Operation(id), Kind::Read),
        ] {
            let stage = checker
                .index_effect_stage(&reports, 0, port, Span::default())
                .unwrap()
                .unwrap();
            assert_eq!(stage.receiver, receiver);
            assert_eq!(stage.access, access);
            assert_eq!(stage.access.length, length);
            assert_eq!(stage.access.capacity, 3);
            assert!(stage.access.normal && stage.access.may_return);
            assert_eq!(stage.load, load);
            assert_eq!(stage.kind, kind);
        }
        let result = checker.index_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        );
        assert_eq!(result.is_ok(), load);
        if let Ok(Some(stage)) = result {
            assert_eq!(stage.kind, Kind::Load);
        }
    }
}

#[test]
pub(crate) fn index_effect_stages_keep_partial_snapshots_without_terminal_reads() {
    let (mut checker, reports) = checked("xs:[1];p:&xs;'out{p[{'out.leave()}]}", false);
    let id = *checker.indices.first_key_value().unwrap().0;
    assert!(!reports.index.operations.contains_key(&id));
    for port in [Port::Projection { point: id, step: 0 }, Port::Snapshot(id)] {
        let stage = checker
            .index_effect_stage(&reports, 0, port, Span::default())
            .unwrap()
            .unwrap();
        assert!(!stage.access.may_return && !stage.access.normal);
    }
    assert!(
        checker
            .index_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .is_err()
    );
    let (mut checker, reports) = checked(
        "stop<never>:(){'loop{'loop.restart()}};stop()[missing]",
        false,
    );
    let id = *checker.indices.first_key_value().unwrap().0;
    for port in [Port::Snapshot(id), Port::Operation(id)] {
        assert!(
            checker
                .index_effect_stage(&reports, 0, port, Span::default())
                .is_err()
        );
    }
}

#[test]
pub(crate) fn index_effect_stages_require_exact_selectors_and_registered_reads() {
    let (mut checker, mut reports) = checked("xs:[1];p:&xs;p[1]", false);
    let id = *checker.indices.first_key_value().unwrap().0;
    assert!(
        checker
            .index_effect_stage(
                &reports,
                0,
                Port::Projection { point: id, step: 1 },
                Span::default()
            )
            .is_err()
    );
    reports.index.operations.remove(&id);
    assert!(
        checker
            .index_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .is_err()
    );
    assert!(
        checker
            .index_effect_stage(&reports, 0, Port::Snapshot(id), Span::default())
            .unwrap()
            .is_some()
    );
    checker
        .indices
        .get_mut(&id)
        .unwrap()
        .access
        .as_mut()
        .unwrap()
        .normal = false;
    checker.indices.get_mut(&id).unwrap().edges.pop();
    reports.index.operations.insert(id, 0);
    let stage = checker
        .index_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap()
        .unwrap();
    assert!(!stage.access.normal && stage.access.may_return);
}

#[test]
pub(crate) fn index_effect_stages_bound_work_and_leave_other_families_unhandled() {
    let (mut checker, reports) = checked("xs<int32[2]>:[1];xs.add(2);xs[1]", false);
    let method = *checker.methods.first_key_value().unwrap().0;
    assert!(
        checker
            .index_effect_stage(&reports, 0, Port::Snapshot(method), Span::default())
            .unwrap()
            .is_none()
    );
    let id = *checker.indices.first_key_value().unwrap().0;
    let port = Port::Operation(id);
    let counts = checker.edge_counts();
    let before = checker.flow.work;
    let expected = checker
        .index_effect_stage(&reports, 0, port, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.index_effect_stage(&reports, 0, port, Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(stage) = result {
            assert_eq!(stage, expected);
        }
        assert_eq!(checker.edge_counts(), counts);
    }
}
