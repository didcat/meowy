use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn shared_reference_coercions_capture_exact_sources_before_forwarding_and_conversion() {
    for (ty, value, kind) in [
        (
            "int8",
            "1",
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
        ),
        (
            "uint64",
            "1",
            ScalarKind::Int {
                bits: 64,
                signed: false,
            },
        ),
        ("float32", "1.5", ScalarKind::Float { bits: 32 }),
        ("boolean", "true", ScalarKind::Bool),
        ("string", "\"cat\"", ScalarKind::String),
        ("null", "null", ScalarKind::Null),
    ] {
        let body = format!("(p.{{->$;->tag:true}}~<{{-><&{ty}>;tag<boolean>}}>)");
        let source = format!(
            "n<{ty}>:{value};p:&n;a<&{ty}>:{body};b<&{ty}><null>:{body};f<&{ty}>:(p<&{ty}>){{->{body}}}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        let ops: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| op.primary)
            .map(|(&id, op)| (id, op.clone()))
            .collect();
        assert_eq!(ops.len(), 3, "{source}");
        assert_eq!(
            ops.iter()
                .filter(|(_, op)| op.kind == CoercionKind::Convert)
                .count(),
            1
        );
        assert!(ops.iter().any(|(_, op)| op.owner != 0));
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        for (id, op) in ops {
            assert_eq!(op.source, Some(Shape::SharedScalar(kind)));
            assert!(matches!(
                op.kind,
                CoercionKind::Forward | CoercionKind::Convert
            ));
            let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!(observed.source, op.source);
            assert!(observed.primary && observed.projected && observed.result);
            assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
            assert_eq!(checker.points[op.input].parent, Some(id));
            assert_eq!(
                op.edges[1],
                Edge::new(
                    Port::Normal(op.input),
                    Port::Projection { point: id, step: 0 },
                    Route::Next
                )
            );
            assert_eq!(
                checker
                    .primary_effect_inputs(
                        &reports,
                        id,
                        op.owner,
                        &reports.effects[&id].1,
                        Span::default()
                    )
                    .unwrap(),
                [Some(op.input), None]
            );
            assert_eq!(
                checker
                    .forward_coercion_input(&reports, id, op.owner, Span::default())
                    .unwrap(),
                None
            );
            for port in [Port::Projection { point: id, step: 0 }, Port::Normal(id)] {
                let stage = checker
                    .coercion_effect_stage(&reports, op.owner, port, Span::default())
                    .unwrap()
                    .unwrap();
                assert_eq!(stage.source, op.source);
                assert_eq!(stage.op, op.kind);
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn shared_reference_coercions_keep_stopped_inputs_distinct_from_referent_types() {
    for source in [
        "d:@\"debug\";n:1;p:&n;x<&int32>:(p.{->$;->tag:true;d.panic(\"stop\")}~<{-><&int32>;tag<boolean>}>)",
        "d:@\"debug\";x<&int32>:d.panic(\"stop\")",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(!checker.reborrow_ops.is_empty());
        assert!(checker.coercions.values().all(|op| !op.primary));
        for (&id, op) in &checker.reborrow_ops {
            assert!(op.site.is_none() && op.parent_mode.is_none());
            assert_eq!(
                op.edges,
                [Edge::new(
                    Port::Entry(id),
                    Port::Entry(op.parent),
                    Route::Next
                )]
            );
            assert!(!reports.effects.contains_key(&id));
            assert!(
                !reports.entries[&op.owner]
                    .1
                    .ports
                    .contains(&Port::Normal(id))
            );
        }
    }
    let source = "f<&int32>:(r<{-><never>;tag<boolean>}>){->r}";
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(errors[0].code, "B001");
    assert_eq!(
        errors[0].message,
        "borrow origins outside supported local storage is not supported by this bootstrap compiler"
    );
    let (checker, reports) = checked(source, false);
    let ops: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.primary)
        .collect();
    assert_eq!(ops.len(), 1);
    let (&id, op) = ops[0];
    assert_eq!(
        (op.kind, op.source),
        (CoercionKind::Stopped, Some(Shape::Never))
    );
    let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert!(observed.projected);
    assert!(!observed.operation && !observed.result);
    assert!(
        !reports.entries[&op.owner]
            .1
            .ports
            .contains(&Port::Normal(id))
    );
    assert!(
        !reports.entries[&op.owner]
            .1
            .ports
            .contains(&Port::Operation(id))
    );
    assert!(!CoercionKind::Stopped.valid_source(true, Some(Shape::SharedScalar(ScalarKind::Bool))));
}
