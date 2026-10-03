use super::{super::tests::checked, *};

#[test]
pub(crate) fn list_effect_selection_reuses_validation_with_exact_constant_work() {
    let source = "<T>:<int32><boolean>;<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<T[3]><string[3]>:[v,v,v]}";
    let (mut checker, reports) = checked(source, false);
    let (&id, op) = checker.lists.first_key_value().unwrap();
    let owner = op.owner;
    let valid = checker
        .validate_list_producer(&reports, owner, id, Span::default())
        .unwrap()
        .unwrap();
    assert!(valid.terminal && valid.stopped.is_none());
    let plans = checker.list_inputs.clone();
    let lists = checker.lists.clone();
    let counts = checker.edge_counts();
    let mut work = None;
    for port in [
        Port::Projection { point: id, step: 0 },
        Port::Conversion { point: id, part: 0 },
        Port::Operation(id),
        Port::Normal(id),
    ] {
        let before = checker.flow.work;
        checker
            .select_list_stage(&valid, port, Span::default())
            .unwrap();
        let used = checker.flow.work - before;
        assert_eq!(*work.get_or_insert(used), used);
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - used + short;
            let result = checker.select_list_stage(&valid, port, Span::default());
            assert_eq!(result.is_ok(), short == 0);
            if let Err(error) = result {
                assert!(error.message.contains("budget"));
            }
        }
        checker.flow.work = 0;
        checker.flow.full = false;
    }
    for port in [
        Port::Entry(id),
        Port::Normal(id + 1),
        Port::Projection { point: id, step: 2 },
        Port::Projection { point: id, step: 3 },
        Port::Conversion { point: id, part: 2 },
        Port::Conversion { point: id, part: 3 },
    ] {
        assert!(
            checker
                .select_list_stage(&valid, port, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
    assert_eq!(checker.list_inputs, plans);
    assert_eq!(checker.lists, lists);
    assert_eq!(checker.edge_counts(), counts);
}

#[test]
pub(crate) fn list_effect_selection_keeps_stopped_projection_and_terminal_boundaries() {
    for (source, primary) in [
        (
            "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};xs<int32[2]><uint8[2]>:[stop(),{x:300;->x}]",
            false,
        ),
        (
            "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[2]><string[2]>:[v,{x:1;->x}]}",
            true,
        ),
    ] {
        let (mut checker, reports) = checked(source, false);
        let (&id, op) = checker.lists.first_key_value().unwrap();
        let owner = op.owner;
        let valid = checker
            .validate_list_producer(&reports, owner, id, Span::default())
            .unwrap()
            .unwrap();
        assert_eq!(valid.stopped, Some(0));
        assert!(!valid.terminal);
        for port in [
            Port::Projection { point: id, step: 0 },
            Port::Projection { point: id, step: 1 },
            Port::Conversion { point: id, part: 0 },
            Port::Operation(id),
            Port::Normal(id),
        ] {
            assert_eq!(
                checker
                    .select_list_stage(&valid, port, Span::default())
                    .is_ok(),
                primary && port == Port::Projection { point: id, step: 0 }
            );
        }
        checker.list_inputs.get_mut(&id).unwrap()[1].point = id;
        assert!(
            checker
                .validate_list_producer(&reports, owner, id, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}
