use super::{super::tests::checked, *};

#[test]
pub(crate) fn dispatch_effects_reject_changed_or_invalid_receiver_shapes_atomically() {
    use crate::check::dependencies::{ScalarKind, bodies::completion::Shape};
    for shape in [
        Shape::Other,
        Shape::Never,
        Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        }),
    ] {
        let (mut checker, reports) = checked("v:3.{->$}", false);
        let id = *checker.dispatch_ops.keys().next().unwrap();
        let before = reports.effects.clone();
        checker.dispatch_ops.get_mut(&id).unwrap().receiver = shape;
        assert!(
            checker
                .dispatch_result_body(&reports, id, 0, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        let mut effects = before.clone();
        assert!(
            checker
                .record_dispatch_effect(0, Port::Operation(id), &mut effects, 100, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(effects, before);
        if shape != Shape::Other {
            assert!(
                checker
                    .validate_dispatch_effect(&reports, 0, Port::Operation(id), Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity")
            );
        }
    }
}

#[test]
pub(crate) fn dispatch_effects_keep_receiver_binding_body_and_independent_visits() {
    for source in [
        "v:3.{->$}",
        "v:3.{}",
        "n:1;v:(&n).{->*$}",
        "v:3.{f<()->int32>;f<int32>:(){->1};->$}",
        "v<{x<int32>;y<int32>}>:{->((3.{->x:$}));->y:4}",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, mut reports) = checked(source, false);
        let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
        let owner = op.owner;
        assert_eq!(
            reports.effects[&id],
            (
                owner,
                Effect::Dispatch(Observed {
                    input: op.input,
                    local: op.local,
                    block: op.block,
                    receiver: op.receiver,
                    normal: true,
                    control: false,
                    initialized: true,
                    result: true,
                })
            )
        );
        for port in [Port::Operation(id), Port::Normal(id)] {
            for (_, walk) in reports.entries.values_mut() {
                walk.ports.clear();
            }
            reports.entries.get_mut(&owner).unwrap().1.ports = vec![port, port];
            let effects = checker
                .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
                .unwrap();
            assert_eq!(effects.len(), 1);
            let (_, Effect::Dispatch(observed)) = &effects[&id] else {
                panic!()
            };
            assert_eq!(observed.initialized, port == Port::Operation(id));
            assert_eq!(observed.result, port == Port::Normal(id));
        }
    }
}

#[test]
pub(crate) fn dispatch_effects_keep_nested_owners_control_and_conditional_calls() {
    let source = "flag:false;g<int32>:(){->3};|flag|v:g().{inner:4.{->$};->$};f<int32>:(n<int32>){->n.{->$}}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.dispatch_ops.len(), 3);
    assert!(checker.dispatch_ops.values().any(|op| op.control));
    assert!(checker.dispatch_ops.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.dispatch_ops {
        let (owner, Effect::Dispatch(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.input, op.input);
        assert_eq!(observed.local, op.local);
        assert_eq!(observed.block, op.block);
        assert_eq!(observed.control, op.control);
        assert!(observed.initialized && observed.result);
    }
    assert_eq!(checker.invocations.len(), 1);
    let call = checker.invocations.values().next().unwrap();
    assert!(matches!(
        reports.effects[&call.point].1,
        Effect::Call { .. }
    ));
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
}

#[test]
pub(crate) fn dispatch_effects_preserve_initialization_before_stopped_bodies() {
    for (tail, init) in [
        ("v:stop().{}", false),
        ("v:3.{stop()}", true),
        ("stop();v:3.{->$}", false),
        ("'out{v:3.{'out.leave()}}", true),
        ("f<{x<int32>}>:(){->3.{stop()}}", true),
    ] {
        let source = format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};{tail}");
        crate::compile(&source).unwrap();
        let (checker, reports) = checked(&source, false);
        let (&id, _) = checker.dispatch_ops.first_key_value().unwrap();
        assert_eq!(reports.effects.contains_key(&id), init);
        if init {
            let (_, Effect::Dispatch(op)) = reports.effects[&id] else {
                panic!()
            };
            assert!(op.initialized && !op.result && !op.normal);
        }
    }
}
