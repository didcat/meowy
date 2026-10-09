use super::*;
use crate::{check::dependencies::bodies::layout::MAX_SLOTS, flow::Flow};

impl Checker {
    pub(in crate::check::dependencies) fn shared_receiver_type(
        ty: &hir::Type,
        flow: &mut Flow,
        span: Span,
    ) -> Result<Option<ScalarKind>> {
        let hir::Type::Record { primary, fields } = ty else {
            return Ok(None);
        };
        let hir::Type::Reference(target) = primary.as_ref() else {
            return Ok(None);
        };
        let Some(kind) = ScalarKind::of(target) else {
            return Ok(None);
        };
        if fields.len() >= MAX_SLOTS || !flow.spend(fields.len() + 1) {
            return Err(Diagnostic::unsupported(
                "proof receiver-type budget exhausted",
                span,
            ));
        }
        for field in fields {
            if field.mutable || !Checker::eligible_type(&field.ty, flow, span, MAX_SLOTS)? {
                return Ok(None);
            }
        }
        Ok(Some(kind))
    }
}

#[cfg(test)]
mod tests {
    use super::{super::tests::checked, *};

    #[test]
    pub(crate) fn shared_receiver_index_keeps_exact_types_without_widening_local_eligibility() {
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
                "n<{ty}>:{value};p:&n;x:{{->p;->tag:true;->more:{{->n:1;->items:[true]}}}}.{{->$==p}};f<boolean>:(p<&{ty}>){{->{{->p;->tag:true}}.{{->$==p}}}}"
            );
            crate::compile(&source).unwrap();
            let (mut checker, program) = checked(&source);
            let reports = checker.entry_reports(&program, Span::default()).unwrap();
            assert_eq!(reports.receivers.len(), 2);
            assert!(reports.receivers.values().any(|(owner, _, _)| *owner != 0));
            for (&local, &(_, _, primary)) in &reports.receivers {
                assert_eq!(primary, Some(kind));
                assert!(!reports.eligible.contains(&local));
                assert!(!reports.initializers.contains_key(&local));
            }
            assert_eq!(reports.slot_uses.len(), 2);
            for (local, ty) in program.locals.iter().enumerate() {
                if ty.has_reference() {
                    assert!(!reports.eligible.contains(&local));
                }
            }
        }
    }

    #[test]
    pub(crate) fn shared_receiver_index_excludes_reference_fields_mutability_and_other_referents() {
        for source in [
            "n:1;p:&n;x:{->p;->tag:=true}.{}",
            "n:1;p:&n;x:{->p;->other:p}.{}",
            "n:1;p:&n;x:{->p;->other:{->p:p}}.{}",
            "n:1;p:&n;x:{->p;->other:{->tag:=true}}.{}",
            "n:1;p:&n;x:{->p;->other:@\"memory\".heap}.{}",
            "n:[1];p:&n;x:{->p;->tag:true}.{}",
            "n:1;p:&n;q:&p;x:{->q;->tag:true}.{}",
            "n<int32><null>:1;p:&n;x:{->p;->tag:true}.{}",
            "n:1;p:&n;x:p.{}",
            "x:{->1;->tag:true}.{}",
            "d:@\"debug\";x:d.panic(\"stop\").{}",
        ] {
            crate::compile(source).unwrap();
            let (mut checker, program) = checked(source);
            let reports = checker.entry_reports(&program, Span::default()).unwrap();
            assert_eq!(reports.receivers.len(), 1);
            assert!(
                reports
                    .receivers
                    .values()
                    .all(|(_, _, primary)| primary.is_none()),
                "{source}"
            );
        }
        let (mut checker, program) = checked("n:1;p:&n;x:{->p;->tag:true}.{}");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let local = *reports.receivers.keys().next().unwrap();
        checker.proofs.mutable.insert(local);
        assert!(
            checker
                .receiver_index(&program, &reports, Span::default(), 1, 1)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }

    #[test]
    pub(crate) fn shared_receiver_index_shares_exact_rows_payload_and_type_work() {
        let (mut checker, program) =
            checked("n:1;p:&n;x:{->p;->tag:true;->more:{->items:[true]}}.{}");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let start = checker.flow.work;
        let expected = checker
            .receiver_index(&program, &reports, Span::default(), 1, 1)
            .unwrap();
        let work = checker.flow.work - start;
        assert_eq!(expected, (reports.receivers.clone(), 0));
        for (limit, parts) in [(0, 1), (1, 0)] {
            assert!(
                checker
                    .receiver_index(&program, &reports, Span::default(), limit, parts)
                    .unwrap_err()
                    .message
                    .contains("budget")
            );
        }
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.receiver_index(&program, &reports, Span::default(), 1, 1);
            if short == 0 {
                assert_eq!(result.unwrap(), expected);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        let ty = hir::Type::Record {
            primary: Box::new(hir::Type::Reference(Box::new(hir::Type::Bool))),
            fields: vec![
                hir::Field {
                    name: "x".into(),
                    ty: hir::Type::Bool,
                    mutable: false
                };
                MAX_SLOTS
            ],
        };
        assert!(
            Checker::shared_receiver_type(&ty, &mut Flow::new(), Span::default())
                .unwrap_err()
                .message
                .contains("budget")
        );
    }

    #[test]
    pub(crate) fn shared_receiver_index_rejects_stale_referents_fields_and_canonical_descriptors() {
        for fault in 0..5 {
            let (mut checker, mut program) = checked("n:1;p:&n;x:{->p;->tag:true}.{}");
            let reports = checker.entry_reports(&program, Span::default()).unwrap();
            let (&id, op) = checker.dispatch_ops.first_key_value().unwrap();
            let local = op.local;
            let hir::Type::Record { primary, fields } = &mut program.locals[local] else {
                panic!()
            };
            match fault {
                0 => **primary = hir::Type::Reference(Box::new(hir::Type::Bool)),
                1 => fields[0].mutable = true,
                2 => fields[0].ty = hir::Type::Reference(Box::new(hir::Type::Bool)),
                3 => {
                    checker.dispatch_ops.get_mut(&id).unwrap().shared_primary =
                        Some(ScalarKind::Bool)
                }
                4 => checker.dispatch_ops.get_mut(&id).unwrap().shared_primary = None,
                _ => unreachable!(),
            }
            let before = format!("{reports:?}{:?}", checker.edge_counts());
            assert!(
                checker
                    .receiver_index(&program, &reports, Span::default(), 1, 1)
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "fault {fault}"
            );
            assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        }
    }
}
