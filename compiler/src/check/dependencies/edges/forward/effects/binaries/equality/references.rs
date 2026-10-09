use super::*;

#[test]
pub(crate) fn shared_scalar_equality_reports_keep_exact_kinds_and_fixed_costs() {
    let mut costs = Vec::new();
    let mut types = vec![
        ("null".to_owned(), "null", ScalarKind::Null),
        ("boolean".into(), "true", ScalarKind::Bool),
        ("string".into(), "\"x\"", ScalarKind::String),
    ];
    for bits in [8, 16, 32, 64] {
        for prefix in ["int", "uint"] {
            types.push((
                format!("{prefix}{bits}"),
                "1",
                ScalarKind::Int {
                    bits,
                    signed: prefix == "int",
                },
            ));
        }
    }
    for bits in [32, 64] {
        types.push((format!("float{bits}"), "1.5", ScalarKind::Float { bits }));
    }
    for (ty, value, expected) in types {
        let source = format!("n<{ty}>:{value};p:&n;p==p");
        crate::compile(&source).unwrap();
        let (mut checker, mut reports) = checked(&source, false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let Class::SharedScalar(kind) = op.types.inputs[0] else {
            panic!()
        };
        assert_eq!(kind, expected);
        assert_eq!(op.types.inputs, [Class::SharedScalar(kind); 2]);
        reports.entries.get_mut(&0).unwrap().1.ports =
            vec![Port::Operation(id), Port::Normal(id), Port::Operation(id)];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let start = checker.flow.work;
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        let work = checker.flow.work - start;
        costs.push(work);
        assert_eq!(effects.len(), 1);
        let (_, Effect::Binary(observed)) = &effects[&id] else {
            panic!()
        };
        assert!(observed.operation && observed.result && observed.plan.equality);
        assert_eq!(observed.types.inputs, [Class::SharedScalar(kind); 2]);
        for short in [0, 1] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
            checker.flow.full = false;
            let result = checker.operation_effects_limited(&reports, Span::default(), 1, 0, 0);
            if short == 0 {
                assert_eq!(result.unwrap(), effects);
            } else {
                assert!(result.unwrap_err().message.contains("budget"));
            }
        }
        checker.flow.work = 0;
        checker.flow.full = false;
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), 0, 0, 0)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    assert!(costs.iter().all(|cost| *cost == costs[0]));
}

#[test]
pub(crate) fn shared_scalar_equality_reports_reject_mismatched_and_invalid_referents() {
    for fault in 0..7 {
        let (mut checker, mut reports) = checked("n:1;p:&n;p==p", false);
        let (&id, op) = checker.binaries.first_key_value().unwrap();
        let mut types = op.types;
        match fault {
            0 => types.inputs[1] = Class::SharedScalar(ScalarKind::Bool),
            1 => {
                types.inputs[1] = Class::SharedScalar(ScalarKind::Int {
                    bits: 16,
                    signed: true,
                })
            }
            2 => {
                types.inputs[1] = Class::SharedScalar(ScalarKind::Int {
                    bits: 32,
                    signed: false,
                })
            }
            3 => {
                types.inputs = [Class::SharedScalar(ScalarKind::Int {
                    bits: 7,
                    signed: true,
                }); 2]
            }
            4 => types.inputs = [Class::SharedScalar(ScalarKind::Float { bits: 16 }); 2],
            5 => types.result = types.inputs[0],
            6 => types.inputs[1] = Class::Reference(crate::hir::ReferenceMode::Shared),
            _ => unreachable!(),
        }
        checker.binaries.get_mut(&id).unwrap().types = types;
        reports.entries.get_mut(&0).unwrap().1.ports = vec![Port::Operation(id), Port::Normal(id)];
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .operation_effects(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
