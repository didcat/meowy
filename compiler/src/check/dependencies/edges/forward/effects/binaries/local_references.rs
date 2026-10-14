use super::{super::tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn local_reference_binaries_preserve_operand_ownership_and_full_record_equality() {
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
                "n<{ty}>:{value};p:&n;r:{init};alias:((r));a:r==p;b:p!=((alias));flag:=true;|flag|inner:{{v:(r~<{{-><&{ty}>;tag<boolean>}}>)==p}};same:r==alias;earlier<&{ty}>:r;cmp:earlier==p;f<boolean>:(p<&{ty}>){{r:{init};->p==r}}"
            );
            crate::compile(&source).unwrap();
            let (mut checker, reports) = checked(&source, false);
            let mut steps = BTreeSet::new();
            let mut owners = BTreeSet::new();
            let mut records = 0;
            for (&id, op) in &checker.binaries.clone() {
                let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
                    panic!()
                };
                assert_eq!(observed.projected, op.plan.primary);
                assert!(observed.operation && observed.result);
                let inputs = checker
                    .primary_effect_inputs(
                        &reports,
                        id,
                        op.owner,
                        &reports.effects[&id].1,
                        Span::default(),
                    )
                    .unwrap();
                for (step, primary) in op.plan.primary.into_iter().enumerate() {
                    assert_eq!(inputs[step], primary.then_some(op.inputs[step]));
                    if primary {
                        assert_eq!(op.types.inputs[step], Class::SharedScalar(kind));
                        assert!(
                            reports
                                .slot_uses
                                .contains_key(&Port::Projection { point: id, step })
                        );
                        steps.insert(step);
                        owners.insert(op.owner);
                    }
                }
                if op.types.inputs == [Class::Record { fields: 1 }; 2] {
                    assert_eq!(op.plan.primary, [false; 2]);
                    records += 1;
                }
            }
            assert_eq!(steps, BTreeSet::from([0, 1]));
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert_eq!(records, 1);
            assert!(!reports.slot_uses.is_empty());
            assert!(reports.slot_uses.keys().any(|port| matches!(port,
                Port::Projection { point, step: 0 } if checker.coercions.contains_key(point)
            )));
        }
    }
}

#[test]
pub(crate) fn local_reference_binaries_keep_stopped_operand_prefixes_and_partial_projections() {
    let source = "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};r==d.panic(\"stop\")";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.plan.primary, [true, false]);
    assert_eq!(op.plan.normal, [true, true]);
    let (_, Effect::Binary(observed)) = &reports.effects[&id] else {
        panic!()
    };
    assert_eq!(observed.projected, [true, false]);
    assert!(!observed.operation && !observed.result);
    assert!(!reports.entries[&0].1.ports.contains(&Port::Normal(id)));
    let source = "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};d.panic(\"stop\")==r";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert!(!reports.effects.contains_key(&id));
    assert!(
        !reports.entries[&0]
            .1
            .ports
            .contains(&Port::Normal(op.inputs[0]))
    );
    assert!(
        !reports.entries[&0]
            .1
            .ports
            .contains(&Port::Entry(op.inputs[1]))
    );
}

#[test]
pub(crate) fn local_reference_binaries_keep_mutable_alias_load_call_and_receiver_origins_opaque() {
    for source in [
        "n:1;p:&n;r:={->p;->tag:true};p==r",
        "n:1;p:&n;r:{->p;->tag:=true};r==p",
        "n:1;p:&n;r:{->p;->other:p};p!=r",
        "n:1;p:&n;r:{->p;->tag:true};out:{->named:r;p==named}",
        "n:1;p:&n;r:{->p;->tag:true};(*(&r))==p",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->p;->tag:true};n:1;p:&n;r:f(p);r==p",
    ] {
        crate::compile(source).unwrap();
        let (_, reports) = checked(source, false);
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
}
