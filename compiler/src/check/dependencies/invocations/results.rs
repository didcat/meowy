use super::{tests::check, *};
use crate::check::dependencies::{
    CoercionKind, ScalarKind,
    bodies::{Completion, completion::Shape},
};

#[test]
pub(crate) fn call_signatures_keep_exact_scalar_reference_and_aggregate_result_shapes() {
    for (source, shape) in [
        ("f:(){};f()", Shape::Scalar(ScalarKind::Null)),
        ("f:(){->true};f()", Shape::Scalar(ScalarKind::Bool)),
        (
            "f<uint8>:(){->7};f()",
            Shape::Scalar(ScalarKind::Int {
                bits: 8,
                signed: false,
            }),
        ),
        (
            "f<int64>:(){->7};f()",
            Shape::Scalar(ScalarKind::Int {
                bits: 64,
                signed: true,
            }),
        ),
        (
            "f<float32>:(){->1.5};f()",
            Shape::Scalar(ScalarKind::Float { bits: 32 }),
        ),
        (
            "f<string>:(){->\"value\"};f()",
            Shape::Scalar(ScalarKind::String),
        ),
        (
            "f<&int32>:(p<&int32>){->p};n:1;f(&n)",
            Shape::SharedScalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
        ),
        (
            "f<&{n<int32>}>:(p<&{n<int32>}>){->p};r:{->n:1};f(&r)",
            Shape::Reference(hir::ReferenceMode::Shared),
        ),
        (
            "f<{-><int32>;tag<boolean>;n<int32>}>:(){->7;->tag:true;->n:2};f()",
            Shape::Record { fields: 2 },
        ),
        ("f<int32[3]>:(){->[1,2]};f()", Shape::List { capacity: 3 }),
        ("f<int32><null>:(){->7};f()", Shape::Union { members: 2 }),
        ("f<never>:(){'loop{'loop.restart()}};f()", Shape::Never),
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.invocations.len(), 1);
        let call = checker.invocations.first_key_value().unwrap().1;
        let function = checker.functions[call.function].as_ref().unwrap();
        let completion = Completion::of(&function.result);
        assert_eq!(function.id, call.function);
        assert_eq!(completion.result, shape, "{source}");
        assert!(completion.valid());
        assert_eq!(call.may_return, completion.normal);
        assert_eq!(checker.points[call.point].owner, call.owner);
        assert_eq!(
            call.edges
                .iter()
                .any(|edge| edge.to == Port::Normal(call.point)),
            completion.normal
        );
    }
}

#[test]
pub(crate) fn call_signature_results_precede_outer_conversions_aliases_and_recursion() {
    let source = "f<uint8>:(n<uint8>){->n};alias:f;a<uint8><null>:f(1);b<uint8><null>:2.(alias);recur<uint8>:(n<uint8>){->recur(n)}";
    crate::compile(source).unwrap();
    let checker = check(source);
    assert_eq!(checker.invocations.len(), 3);
    let mut owners = BTreeSet::new();
    for call in checker.invocations.values() {
        let function = checker.functions[call.function].as_ref().unwrap();
        assert_eq!(
            function.result,
            hir::Type::Int {
                bits: 8,
                signed: false
            }
        );
        let outer = checker.points[call.point].parent.unwrap();
        let coercion = &checker.coercions[&outer];
        assert_eq!(coercion.input, call.point);
        assert!(!coercion.primary);
        assert_eq!(
            coercion.kind,
            if call.owner == 0 {
                CoercionKind::Convert
            } else {
                CoercionKind::Forward
            }
        );
        assert_eq!(coercion.owner, call.owner);
        assert!(call.may_return);
        owners.insert(call.owner);
    }
    assert_eq!(owners.len(), 2);
    assert!(owners.contains(&0));
}

#[test]
pub(crate) fn call_return_boundaries_follow_signatures_when_bodies_never_complete() {
    for (signature, shape, may_return) in [
        (
            "<int32>",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true,
            }),
            true,
        ),
        ("<{tag<boolean>}>", Shape::Record { fields: 1 }, true),
        ("<never>", Shape::Never, false),
        ("", Shape::Never, false),
    ] {
        let source = format!("d:@\"debug\";f{signature}:(){{d.panic(\"stop\")}};f()");
        crate::compile(&source).unwrap();
        let checker = check(&source);
        let call = checker.invocations.first_key_value().unwrap().1;
        let function = checker.functions[call.function].as_ref().unwrap();
        assert_eq!(Completion::of(&function.result).result, shape);
        assert_eq!(call.may_return, may_return);
        assert_eq!(
            checker.bodies[&function.body.id].completion.result,
            Shape::Never
        );
        assert!(!checker.bodies[&function.body.id].completion.normal);
    }
}
