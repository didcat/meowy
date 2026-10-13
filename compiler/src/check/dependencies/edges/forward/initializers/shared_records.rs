use super::{tests::checked, *};
use crate::check::dependencies::{OperationKind, ScalarKind};
use std::collections::BTreeSet;

#[test]
pub(crate) fn shared_local_records_retain_types_binds_and_storage_without_ordinary_eligibility() {
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
                "n<{ty}>:{value};p:&n;r:{init};copy:((r));out:copy.tag;f<boolean>:(p<&{ty}>,flag<boolean>){{r:{init};|flag|copy:r;box:{{->named:r;copy:named}};->r.tag}}"
            );
            let (mut checker, program, reports) = checked(&source);
            assert!(checker.locals.is_empty());
            let mut owners = BTreeSet::new();
            let mut aliases = 0;
            let mut binds = 0;
            for (&id, read) in &checker.local_reads.clone() {
                if Checker::shared_receiver_type(
                    &program.locals[read.local],
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap()
                    != Some(kind)
                {
                    continue;
                }
                assert!(!reports.eligible.contains(&read.local));
                assert!(!reports.initializers.contains_key(&read.local));
                assert_eq!(
                    read.storage,
                    checker
                        .proofs
                        .aliases
                        .get(&read.local)
                        .map_or(read.local, |alias| alias.root)
                );
                aliases += usize::from(checker.proofs.aliases.contains_key(&read.local));
                owners.insert(read.owner);
                assert_eq!(
                    checker
                        .read_initializer_input(&reports, id, read.owner, Span::default(),)
                        .unwrap(),
                    None
                );
            }
            for (&id, (owner, effect)) in &reports.effects {
                let Effect::Storage {
                    kind: OperationKind::Bind,
                    local,
                    ..
                } = effect
                else {
                    continue;
                };
                if Checker::shared_receiver_type(
                    &program.locals[*local],
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap()
                    != Some(kind)
                {
                    continue;
                }
                let binding = checker
                    .binding_effect(&reports, id, *owner, Span::default())
                    .unwrap()
                    .unwrap();
                assert_eq!(binding.local, *local);
                assert_eq!(binding.storage, *local);
                let input = binding.input.unwrap();
                assert_eq!(checker.points[input].parent, Some(id));
                assert_eq!(checker.points[input].owner, *owner);
                binds += 1;
            }
            assert_eq!(owners, BTreeSet::from([0, 1]));
            assert!(
                aliases > 0 && binds >= 5,
                "aliases {aliases}, binds {binds}: {source}"
            );
        }
    }
}

#[test]
pub(crate) fn shared_local_records_keep_mutability_reference_fields_and_other_referents_opaque() {
    for init in [
        "{->p;->tag:=true}",
        "{->p;->other:p;->tag:true}",
        "{->p;->other:{->p:p};->tag:true}",
        "{->p;->other:{->tag:=true};->tag:true}",
        "{->&p;->tag:true}",
    ] {
        let source = format!("n:1;p:&n;r:{init};copy:r;out:copy.tag");
        let (mut checker, program, reports) = checked(&source);
        for (&id, read) in &checker.local_reads.clone() {
            if !matches!(program.locals[read.local], hir::Type::Record { .. }) {
                continue;
            }
            assert_eq!(
                Checker::shared_receiver_type(
                    &program.locals[read.local],
                    &mut checker.flow,
                    Span::default(),
                )
                .unwrap(),
                None
            );
            assert!(!reports.eligible.contains(&read.local));
            assert_eq!(
                checker
                    .read_initializer_input(&reports, id, read.owner, Span::default(),)
                    .unwrap(),
                None
            );
        }
        assert!(reports.slot_uses.is_empty(), "{source}");
    }
    let (mut checker, program, reports) = checked("n:1;p:&n;r:={->p;->tag:true};out:r.tag");
    let read = checker
        .local_reads
        .values()
        .find(|read| matches!(program.locals[read.local], hir::Type::Record { .. }))
        .unwrap();
    assert!(checker.proofs.variable(read.local));
    assert!(
        Checker::shared_receiver_type(
            &program.locals[read.local],
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
        .is_some()
    );
    assert!(!reports.eligible.contains(&read.local));
    assert!(reports.slot_uses.is_empty());
}

#[test]
pub(crate) fn shared_local_records_preserve_stopped_binds_and_unknown_call_sources() {
    for source in [
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true;d.panic(\"stop\")};later:1",
        "d:@\"debug\";n:1;p:&n;r:{->p;->tag:true};d.panic(\"stop\");later:r.tag",
        "f<{-><&int32>;tag<boolean>}>:(p<&int32>){->{->p;->tag:true}};n:1;r:f(&n);out:r.tag",
    ] {
        let (mut checker, program, reports) = checked(source);
        let ops: Vec<_> = checker
            .operations
            .iter()
            .map(|(&id, op)| (id, op.owner, op.local))
            .collect();
        for (id, owner, local) in ops {
            if !reports.effects.contains_key(&id) {
                assert!(
                    checker
                        .binding_effect(&reports, id, owner, Span::default())
                        .unwrap()
                        .is_none()
                );
                assert!(!reports.initializers.contains_key(&local));
            }
        }
        for (local, ty) in program.locals.iter().enumerate() {
            if ty.has_reference() {
                assert!(!reports.eligible.contains(&local));
                assert!(!reports.initializers.contains_key(&local));
            }
        }
        for id in checker.fields.keys() {
            assert!(
                !reports.slot_uses.contains_key(&Port::Operation(*id)),
                "{source}"
            );
        }
    }
}
