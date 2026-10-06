use super::*;
use crate::check::dependencies::{ScalarKind, lists::tests::check};

#[test]
pub(crate) fn contextual_list_sources_capture_original_primary_kinds_before_conversion() {
    for (ty, shape) in [
        (
            "int8",
            Shape::Scalar(ScalarKind::Int {
                bits: 8,
                signed: true,
            }),
        ),
        (
            "uint32",
            Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: false,
            }),
        ),
        ("float32", Shape::Scalar(ScalarKind::Float { bits: 32 })),
        ("boolean", Shape::Scalar(ScalarKind::Bool)),
        ("null", Shape::Scalar(ScalarKind::Null)),
        ("int32[1]", Shape::List { capacity: 1 }),
        ("int32><null", Shape::Union { members: 2 }),
    ] {
        for convert in [false, true] {
            let extra = if ty == "boolean" { "int32" } else { "boolean" };
            let target = if convert {
                format!("{ty}><{extra}")
            } else {
                ty.to_owned()
            };
            let source = format!(
                "<T>:<{target}>;<R>:<{{-><{ty}>;tag<boolean>}}>;<S>:<{{-><string>;tag<boolean>}}>;f:(v<R><S>){{|v<R>|xs<T[1]><string[1]>:[v]}}"
            );
            crate::compile(&source).unwrap();
            let checker = check(&source);
            assert_eq!(checker.list_inputs.len(), 1, "{source}");
            let input = checker.list_inputs.values().next().unwrap()[0];
            assert!(input.primary);
            assert_eq!(input.source, Some(shape));
            assert_eq!(
                input.kind,
                if convert {
                    CoercionKind::Convert
                } else {
                    CoercionKind::Forward
                }
            );
        }
    }
}

#[test]
pub(crate) fn contextual_list_sources_preserve_sparse_plans_deferred_order_and_whole_records() {
    let source = "<T>:<int32><boolean>;<U>:<int32><string>;f:(v<boolean><string>){r:3.{->$;->tag:true};copy:((r));|v<boolean>|xs<T[5]><U[5]>:[1,r,copy,r,v]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    assert!(inputs[0].point > inputs[1].point);
    assert!(inputs[0].source.is_none() && inputs[4].source.is_none());
    for input in &inputs[1..4] {
        assert!(input.primary && input.kind == CoercionKind::Convert);
        assert_eq!(
            input.source,
            Some(Shape::Scalar(ScalarKind::Int {
                bits: 32,
                signed: true
            }))
        );
    }
    assert_eq!(
        checker.sequences[&SequenceSource::Expr(id)].items,
        inputs
            .iter()
            .map(|input| Some(input.point))
            .collect::<Vec<_>>()
    );
    let source = "<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;<T>:<R><boolean>;f:(v<R><S>){|v<R>|xs<T[1]><S[1]>:[v]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let input = checker.list_inputs.values().next().unwrap()[0];
    assert!(!input.primary && input.source.is_none());
    assert_eq!(input.kind, CoercionKind::Convert);
}

#[test]
pub(crate) fn contextual_list_sources_retain_never_before_checked_suffixes() {
    let source = "<R>:<{-><never>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[3]><string[3]>:[v,v,{x:1;->x}]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    assert!(checker.lists[&id].normal);
    for input in &inputs[..2] {
        assert!(input.primary && input.kind == CoercionKind::Stopped);
        assert_eq!(input.source, Some(Shape::Never));
        assert!(checker.points[input.point].complete);
    }
    assert!(inputs[2].source.is_none());
    let key = SequenceSource::Expr(id);
    assert!(checker.sequences[&key].edges.is_empty());
    assert_eq!(
        checker.endpoints[&key].last().unwrap().to,
        Port::Projection { point: id, step: 0 }
    );
}

#[test]
pub(crate) fn contextual_list_sources_reject_invalid_shapes_and_changed_replay_atomically() {
    let source = "<R>:<{-><int32>;tag<boolean>}>;<S>:<{-><string>;tag<boolean>}>;f:(v<R><S>){|v<R>|xs<int32[1]><string[1]>:[v]}";
    let mut checker = check(source);
    let (&id, inputs) = checker.list_inputs.first_key_value().unwrap();
    let inputs = inputs.clone();
    checker.owner = checker.points[id].owner;
    let before = format!(
        "{:?}{:?}{:?}{:?}",
        checker.list_inputs,
        checker.sequences,
        checker.endpoints,
        checker.edge_counts()
    );
    for source in [
        None,
        Some(Shape::Never),
        Some(Shape::Scalar(ScalarKind::Bool)),
        Some(Shape::Scalar(ScalarKind::Int {
            bits: 7,
            signed: true,
        })),
    ] {
        let mut changed = inputs.clone();
        changed[0].source = source;
        assert!(
            checker
                .contextual_list_sequence(id, changed, true, Span::default())
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(
            format!(
                "{:?}{:?}{:?}{:?}",
                checker.list_inputs,
                checker.sequences,
                checker.endpoints,
                checker.edge_counts()
            ),
            before
        );
    }
    let start = checker.flow.work;
    checker
        .contextual_list_sequence(id, inputs.clone(), true, Span::default())
        .unwrap();
    let work = checker.flow.work - start;
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.contextual_list_sequence(id, inputs.clone(), true, Span::default());
        assert_eq!(result.is_ok(), short == 0);
        if short == 1 {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(
            format!(
                "{:?}{:?}{:?}{:?}",
                checker.list_inputs,
                checker.sequences,
                checker.endpoints,
                checker.edge_counts()
            ),
            before
        );
    }
}
