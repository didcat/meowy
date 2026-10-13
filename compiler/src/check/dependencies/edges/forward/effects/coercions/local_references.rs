use super::{super::tests::checked, *};
use crate::check::dependencies::{BinaryClass, ScalarKind};
use std::collections::BTreeSet;

#[test]
pub(crate) fn local_reference_coercions_retain_projection_stages_and_typed_binding_evidence() {
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
        for init in ["{->p;->tag:true}", "p.{->$;->tag:true}"] {
            let source = format!(
                "n<{ty}>:{value};p:&n;r:{init};copy:((r));a<&{ty}>:copy;b<&{ty}><null>:r;flag:=true;|flag|inner:{{c<&{ty}>:(r~<{{-><&{ty}>;tag<boolean>}}> )}};cmp:p!=r;f<&{ty}>:(p<&{ty}>){{r:{init};->r}}"
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
            assert_eq!(
                ops.iter().map(|(_, op)| op.owner).collect::<BTreeSet<_>>(),
                BTreeSet::from([0, 1])
            );
            assert!(reports.slot_uses.is_empty());
            assert!(checker.binaries.values().any(|op| {
                op.plan.primary == [false, true]
                    && op.types.inputs[1] == BinaryClass::SharedScalar(kind)
            }));
            for (&local, row) in &reports.shared_initializers {
                assert_eq!(row.record.primary, kind);
                assert_eq!(row.record.fields, 1);
                assert!(!reports.eligible.contains(&local));
                assert!(!reports.initializers.contains_key(&local));
            }
            for (id, op) in ops {
                let (_, Effect::Coercion(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(op.source, Some(Shape::SharedScalar(kind)));
                assert_eq!((observed.op, observed.source), (op.kind, op.source));
                assert!(observed.primary && observed.projected && observed.result);
                assert_eq!(observed.operation, op.kind == CoercionKind::Convert);
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
        }
    }
}

#[test]
pub(crate) fn local_reference_coercions_preserve_stopped_inputs_and_unobserved_suffixes() {
    for (source, captured, observed) in [
        (
            "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true;d.panic(\"stop\")};copy<&int32>:r",
            0,
            0,
        ),
        (
            "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};a<&int32>:r;d.panic(\"stop\");b<&int32>:r",
            2,
            1,
        ),
        (
            "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};d.panic(\"stop\");b<&int32>:r",
            1,
            0,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let ops: Vec<_> = checker
            .coercions
            .iter()
            .filter(|(_, op)| matches!(op.source, Some(Shape::SharedScalar(_))))
            .collect();
        assert_eq!(ops.len(), captured);
        assert_eq!(
            ops.iter()
                .filter(|(id, _)| reports.effects.contains_key(id))
                .count(),
            observed
        );
        for (&id, op) in ops {
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

#[test]
pub(crate) fn local_reference_coercions_keep_mutable_alias_load_and_call_origins_separate() {
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};copy<&int32>:r",
        "n:1;p:&n;r:{->p;->tag:=true};copy<&int32>:r",
        "n:1;p:&n;r:{->p;->other:p};copy<&int32>:r",
        "n:1;p:&n;r:{->p;->tag:true};out:{->named:r;copy<&int32>:named}",
        "n:1;p:&n;r:{->p;->tag:true};copy<&int32>:*(&r)",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->p;->tag:true};n:1;r:f(&n);copy<&int32>:r",
    ] {
        crate::compile(source).unwrap();
        let (_, reports) = checked(source, false);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
