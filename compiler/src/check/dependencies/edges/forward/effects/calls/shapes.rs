use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn call_result_reports_keep_signature_shapes_after_function_hir_is_transferred() {
    let source = "f<uint8>:(n<uint8>){->n};alias:f;a<uint8><null>:f(1);b<uint8><null>:2.(alias);shared<&int32>:(p<&int32>){->p};n:1;p:shared(&n);row<{tag<boolean>}>:(){->tag:true};r:row();list<int32[3]>:(){->[1]};xs:list();choice<int32><null>:(){->null};u:choice();recur<uint8>:(n<uint8>){->recur(n)}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    assert!(checker.functions.is_empty());
    assert_eq!(checker.invocations.len(), 7);
    let mut shapes = Vec::new();
    for call in checker.invocations.values() {
        let (
            owner,
            Effect::Call {
                site,
                function,
                args,
                shape,
                may_return,
                operation,
                result,
                ..
            },
        ) = &reports.effects[&call.point]
        else {
            panic!()
        };
        assert_eq!(
            (*owner, *site, *function),
            (call.owner, call.site, call.function)
        );
        assert_eq!(*args, call.args);
        assert_eq!(*shape, call.shape);
        assert!(*may_return && *operation && *result);
        shapes.push(*shape);
    }
    for shape in [
        Shape::Scalar(ScalarKind::Int {
            bits: 8,
            signed: false,
        }),
        Shape::SharedScalar(ScalarKind::Int {
            bits: 32,
            signed: true,
        }),
        Shape::Record { fields: 1 },
        Shape::List { capacity: 3 },
        Shape::Union { members: 2 },
    ] {
        assert!(shapes.contains(&shape));
    }
    assert!(reports.slot_uses.is_empty());
    assert_eq!(
        checker.call_graph(&reports, Span::default()).unwrap(),
        reports.calls
    );
}

#[test]
pub(crate) fn call_result_shapes_survive_independent_and_duplicate_observations() {
    let (mut checker, mut reports) = checked("f<uint8>:(n<uint8>){->n};x:f(1)", false);
    let call = checker.invocations.first_key_value().unwrap().1;
    let (id, owner, shape) = (call.point, call.owner, call.shape);
    for ports in [
        vec![],
        vec![Port::Entry(id)],
        vec![Port::Operation(id)],
        vec![Port::Normal(id)],
        vec![Port::Normal(id), Port::Operation(id), Port::Normal(id)],
    ] {
        let operation = ports.contains(&Port::Operation(id));
        let result = ports.contains(&Port::Normal(id));
        for (_, walk) in reports.entries.values_mut() {
            walk.ports.clear();
        }
        reports.entries.get_mut(&owner).unwrap().1.ports = ports;
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (effects, parts) = checker
            .operation_effects_with_room(&reports, Span::default(), 1, 1, 0)
            .unwrap();
        assert_eq!(effects.len(), usize::from(operation || result));
        assert_eq!(parts, 1 - effects.len());
        if operation || result {
            let (
                _,
                Effect::Call {
                    shape: found,
                    operation: op,
                    result: end,
                    ..
                },
            ) = &effects[&id]
            else {
                panic!()
            };
            assert_eq!((*found, *op, *end), (shape, operation, result));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_result_shapes_preserve_signature_boundaries_for_panicking_bodies() {
    for (signature, may_return) in [("int32", true), ("{tag<boolean>}", true), ("never", false)] {
        let source = format!("d:@\"debug\";f<{signature}>:(){{d.panic(\"stop\")}};f()");
        let (checker, reports) = checked(&source, false);
        let call = checker.invocations.first_key_value().unwrap().1;
        let (
            _,
            Effect::Call {
                shape,
                operation,
                result,
                ..
            },
        ) = &reports.effects[&call.point]
        else {
            panic!()
        };
        assert_eq!(*shape, call.shape);
        assert_eq!(*shape == Shape::Never, !may_return);
        assert!(*operation);
        assert_eq!(*result, may_return);
        let body = reports.entries[&(call.function + 1)].0;
        assert_eq!(checker.bodies[&body].completion.result, Shape::Never);
        assert!(
            !reports.entries[&(call.function + 1)]
                .1
                .ports
                .contains(&Port::BlockResult(body))
        );
    }
}

#[test]
pub(crate) fn call_result_shapes_reject_conflicting_stages_and_retained_reports() {
    let (mut checker, mut reports) = checked("f<int32>:(){->1};f()", false);
    let id = checker.invocations.first_key_value().unwrap().1.point;
    let calls = checker.call_effect_index(Span::default()).unwrap();
    let mut stage = checker
        .call_effect_stage(&reports, &calls, 0, Port::Normal(id), Span::default())
        .unwrap()
        .unwrap();
    stage.shape = Shape::Scalar(ScalarKind::Bool);
    let mut effects = Effects::new();
    let mut parts = 0;
    assert!(
        checker
            .record_call_effect(stage, &mut effects, 1, &mut parts, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert!(effects.is_empty());
    assert_eq!(parts, 0);
    let (_, Effect::Call { shape, .. }) = reports.effects.get_mut(&id).unwrap() else {
        panic!()
    };
    *shape = stage.shape;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert!(
        checker
            .call_graph(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("identity")
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
