use super::{super::tests::checked, *};
use crate::check::dependencies::ScalarKind;

#[test]
pub(crate) fn receiver_reference_coercions_retain_exact_kinds_ports_and_owners() {
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
        let source = format!(
            "n<{ty}>:{value};p:&n;x:p.{{->$;->tag:true}}.{{a<&{ty}>:$;b<&{ty}><null>:$;c:p!=(($))}};f<&{ty}>:(p<&{ty}>){{->{{->p;->tag:true}}.{{->$}}}}"
        );
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        let ops: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| op.primary)
            .map(|(&id, op)| (id, op.clone()))
            .collect();
        assert_eq!(ops.len(), 4, "{source}");
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
            let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                panic!()
            };
            assert_eq!((observed.op, observed.source), (op.kind, op.source));
            assert!(observed.primary && observed.projected && observed.result);
            assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
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
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_reference_coercions_preserve_stopped_receivers_and_unobserved_suffixes() {
    for (source, captured, observed, stopped) in [
        (
            "d:@\"debug\";x:d.panic(\"stop\").{a<&int32>:$}",
            0,
            0,
            false,
        ),
        (
            "d:@\"debug\";n:1;p:&n;x:p.{->$;->tag:true}.{a<&int32>:$;d.panic(\"stop\");b<&int32>:$}",
            2,
            1,
            false,
        ),
        (
            "f<boolean>:(r<{-><never>;tag<boolean>}>){->r.{->$}}",
            0,
            0,
            true,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let shared: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| matches!(op.source, Some(Shape::SharedScalar(_))))
            .collect();
        assert_eq!(shared.len(), captured, "{source}");
        assert_eq!(
            shared
                .iter()
                .filter(|(id, _)| reports.effects.contains_key(id))
                .count(),
            observed
        );
        let stops: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| op.primary && op.kind == CoercionKind::Stopped)
            .collect();
        assert_eq!(!stops.is_empty(), stopped, "{source}");
        for (&id, op) in stops {
            assert_eq!(op.source, Some(Shape::Never));
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
        }
        for (&id, op) in shared {
            if !reports.effects.contains_key(&id) {
                for port in [
                    Port::Projection { point: id, step: 0 },
                    Port::Operation(id),
                    Port::Normal(id),
                ] {
                    assert!(!reports.entries[&op.owner].1.ports.contains(&port));
                }
            }
        }
    }
}
