use super::{super::tests::checked, *};

#[test]
pub(crate) fn method_effect_stages_preserve_size_kinds_and_load_choices() {
    for (source, method, load) in [
        ("xs:[1];xs.size()", MethodKind::ListSize, false),
        ("xs:[1];p:&xs;p.size()", MethodKind::ListSize, true),
        ("xs:[1];p:&xs;(*p).size()", MethodKind::ListSize, false),
        ("\"é\".size()", MethodKind::StringSize, false),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.methods.first_key_value().unwrap();
        let receiver = op.receiver;
        let stage = checker
            .method_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            (stage.receiver, stage.method, stage.load, stage.kind),
            (receiver, method, load, Kind::Finish)
        );
        assert!(
            checker
                .method_effect_stage(&reports, 0, Port::Snapshot(id), Span::default())
                .is_err()
        );
        let result = checker.method_effect_stage(
            &reports,
            0,
            Port::Projection { point: id, step: 0 },
            Span::default(),
        );
        assert_eq!(result.is_ok(), load);
    }
}

#[test]
pub(crate) fn method_effect_stages_keep_add_snapshots_lengths_and_stopped_items() {
    for (source, load, length, finish) in [
        ("xs<int32[3]>:[1];xs.add(2)", false, Some(1), true),
        ("xs<int32[3]>:[1];p:&xs;p.add(2)", true, None, true),
        ("xs:[1];'out{xs.add({'out.leave()})}", false, Some(1), false),
        (
            "xs:[1];p:&xs;'out{p.add({'out.leave()})}",
            true,
            None,
            false,
        ),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.methods.first_key_value().unwrap();
        let method = op.kind;
        let stage = checker
            .method_effect_stage(&reports, 0, Port::Snapshot(id), Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(stage.method, method);
        assert_eq!(stage.load, load);
        assert_eq!(stage.kind, Kind::Snapshot);
        assert!(
            matches!(stage.method, MethodKind::Add { length: found, may_return, .. } if found == length && may_return == finish)
        );
        assert_eq!(
            checker
                .method_effect_stage(&reports, 0, Port::Operation(id), Span::default())
                .is_ok(),
            finish
        );
    }
}

#[test]
pub(crate) fn method_effect_stages_reject_stopped_receivers_and_unregistered_terminals() {
    let (mut checker, reports) = checked(
        "stop<never>:(){'loop{'loop.restart()}};stop().add(missing)",
        false,
    );
    let id = *checker.methods.first_key_value().unwrap().0;
    for port in [
        Port::Snapshot(id),
        Port::Operation(id),
        Port::Projection { point: id, step: 0 },
    ] {
        assert!(
            checker
                .method_effect_stage(&reports, 0, port, Span::default())
                .is_err()
        );
    }
    let (mut checker, mut reports) = checked("xs<int32[2]>:[1];p:&xs;p.add(2)", false);
    let id = *checker.methods.first_key_value().unwrap().0;
    reports.index.operations.remove(&id);
    assert!(
        checker
            .method_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .is_err()
    );
    assert!(
        checker
            .method_effect_stage(&reports, 0, Port::Snapshot(id), Span::default())
            .unwrap()
            .is_some()
    );
}

#[test]
pub(crate) fn method_effect_stages_bound_work_and_leave_index_stages_unhandled() {
    let (mut checker, reports) = checked("xs:[1];xs[1];xs.size()", false);
    let index = *checker.indices.first_key_value().unwrap().0;
    assert!(
        checker
            .method_effect_stage(&reports, 0, Port::Snapshot(index), Span::default())
            .unwrap()
            .is_none()
    );
    let id = *checker.methods.first_key_value().unwrap().0;
    let counts = checker.edge_counts();
    let before = checker.flow.work;
    let expected = checker
        .method_effect_stage(&reports, 0, Port::Operation(id), Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + spare;
        let result = checker.method_effect_stage(&reports, 0, Port::Operation(id), Span::default());
        assert_eq!(result.is_ok(), spare == 0);
        if let Ok(stage) = result {
            assert_eq!(stage, expected);
        }
        assert_eq!(checker.edge_counts(), counts);
    }
}
