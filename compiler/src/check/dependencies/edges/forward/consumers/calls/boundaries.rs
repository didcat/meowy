use super::*;

pub(super) const SOURCE: &str = "f:(a<&int32><null>,b<&int32><null>){};n:1;p:&n;r:{->p;->tag:true};out:r.{first:$;second:first;f(first,second)}";

#[test]
pub(crate) fn call_argument_sources_reject_late_identity_and_caller_scope_faults_atomically() {
    for fault in 0..11 {
        let (mut checker, mut reports) = checked(SOURCE);
        let call = checker.invocations.first_key_value().unwrap().1;
        let first = projection(&checker, call.args[0]);
        let id = projection(&checker, call.args[1]);
        let input = checker.coercions[&id].input;
        assert!(first < id);
        assert_eq!(reports.slot_uses.len(), 2);
        match fault {
            0..=4 => {
                let (_, Effect::Coercion(op)) = reports.effects.get_mut(&id).unwrap() else {
                    panic!()
                };
                match fault {
                    0 => op.input = usize::MAX,
                    1 => op.source = None,
                    2 => op.fields = Some(usize::MAX),
                    3 => op.control = !op.control,
                    4 => {
                        op.projected = false;
                        op.operation = false;
                        op.result = false;
                    }
                    _ => unreachable!(),
                }
            }
            5 => reports.effects.get_mut(&id).unwrap().0 = call.function + 1,
            6 => checker.points[input].parent = None,
            7 => checker.coercions.get_mut(&id).unwrap().edges[1].route = Route::Checked,
            8 => {
                reports.index.operations.remove(&id);
            }
            9 => checker.points[input].owner = call.function + 1,
            10 => {
                let read = checker.narrowings[&input].input;
                assert!(!checker.narrowings[&input].changed);
                checker.local_reads.get_mut(&read).unwrap().storage = usize::MAX;
                let (_, Effect::Read { storage, .. }) = reports.effects.get_mut(&read).unwrap()
                else {
                    panic!()
                };
                *storage = usize::MAX;
            }
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}{:?}",
            checker.coercions,
            checker.local_reads,
            checker.edge_counts()
        );
        let error = checker.slot_uses(&reports, Span::default()).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(
            format!(
                "{reports:?}{:?}{:?}{:?}",
                checker.coercions,
                checker.local_reads,
                checker.edge_counts()
            ),
            before
        );
    }
}

#[test]
pub(crate) fn call_argument_sources_keep_call_coercion_and_receiver_control_independent() {
    let (mut checker, mut reports) = checked(SOURCE);
    let expected = reports.slot_uses.clone();
    for flags in 0..8 {
        for call in checker.invocations.values_mut() {
            call.control = flags & 1 != 0;
            let (_, Effect::Call { control, .. }) = reports.effects.get_mut(&call.point).unwrap()
            else {
                panic!()
            };
            *control = call.control;
        }
        for (&id, op) in &mut checker.coercions {
            op.control = flags & 2 != 0;
            if let Some((_, Effect::Coercion(observed))) = reports.effects.get_mut(&id) {
                observed.control = op.control;
            }
        }
        for (&id, op) in &mut checker.dispatch_ops {
            op.control = flags & 4 != 0;
            let (_, Effect::Dispatch(observed)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            observed.control = op.control;
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker.slot_uses(&reports, Span::default()).unwrap(),
            expected
        );
        assert_eq!(
            checker.call_graph(&reports, Span::default()).unwrap(),
            reports.calls
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
