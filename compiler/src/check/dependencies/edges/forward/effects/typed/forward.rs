use super::{super::tests::checked, *};

#[test]
pub(crate) fn ascription_inputs_require_unchanged_observed_results() {
    for (source, allowed) in [
        ("7~<int32>", true),
        ("{->n:1}~<{n<int32>}>", true),
        ("7<int32>", false),
        ("<U>:<int32><null>;7~<U>", false),
        ("d:@\"debug\";d.panic(\"stop\")~<int32>", false),
    ] {
        let (mut checker, mut reports) = checked(source, false);
        let (&id, op) = checker.typed_ops.first_key_value().unwrap();
        let (owner, input) = (op.owner, op.input);
        for result in [true, false] {
            for operation in [true, false] {
                if let Some((_, Effect::Typed(op))) = reports.effects.get_mut(&id) {
                    op.result = result;
                    op.operation = operation;
                }
                assert_eq!(
                    checker
                        .unchanged_ascription_input(&reports, id, owner, Span::default())
                        .unwrap(),
                    (allowed && result).then_some(input),
                    "{source}"
                );
            }
        }
        reports.effects.remove(&id);
        assert_eq!(
            checker
                .unchanged_ascription_input(&reports, id, owner, Span::default())
                .unwrap(),
            None
        );
    }
}

#[test]
pub(crate) fn ascription_inputs_reject_report_and_producer_corruption_without_mutation() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("{->n:1}~<{n<int32>}>", false);
        let id = *checker.typed_ops.first_key_value().unwrap().0;
        let (owner, Effect::Typed(op)) = reports.effects.get_mut(&id).unwrap() else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => op.input = usize::MAX,
            2 => op.op = TypedKind::Predicate,
            3 => op.changed = true,
            4 => op.normal = false,
            5 => op.control = !op.control,
            6 => {
                checker.typed_ops.remove(&id);
            }
            7 => checker.typed_ops.get_mut(&id).unwrap().owner += 1,
            8 => checker.typed_ops.get_mut(&id).unwrap().edges[1].route = Route::Checked,
            9 => checker.points[op.input].parent = None,
            10 => {
                reports.index.operations.remove(&id);
            }
            11 => checker.points[id].span.end += 1,
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.typed_ops,
            checker.edge_counts()
        );
        let error = checker
            .unchanged_ascription_input(&reports, id, 0, Span::default())
            .unwrap_err();
        assert!(
            error.message.contains("typed-effect identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}",
                checker.typed_ops,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn ascription_inputs_preserve_owner_control_and_exact_work_limits() {
    let source = "flag:false;|flag|7~<int32>;f<int32>:(){->7~<int32>}";
    let (mut checker, reports) = checked(source, true);
    let ids: Vec<_> = checker
        .typed_ops
        .iter()
        .map(|(&id, op)| (id, op.owner, op.input))
        .collect();
    assert!(checker.typed_ops.values().any(|op| op.control));
    assert!(ids.iter().any(|(_, owner, _)| *owner != 0));
    for (id, owner, input) in ids {
        let before = checker.flow.work;
        assert_eq!(
            checker
                .unchanged_ascription_input(&reports, id, owner, Span::default())
                .unwrap(),
            Some(input)
        );
        let work = checker.flow.work - before;
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.unchanged_ascription_input(&reports, id, owner, Span::default());
            assert_eq!(result.is_ok(), short == 0);
            if let Ok(result) = result {
                assert_eq!(result, Some(input));
            }
        }
        checker.flow.work = 0;
        checker.flow.full = false;
    }
}
