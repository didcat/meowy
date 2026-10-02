use super::{super::tests::checked, *};

#[test]
pub(crate) fn equality_effects_reject_corrupt_categories_counts_and_checked_flags() {
    for result in [false, true] {
        for fault in 0..16 {
            let (mut checker, mut reports) = checked("a<int32[2]>:[1];b<int32[2]>:[1];a==b", false);
            let id = *checker.binaries.first_key_value().unwrap().0;
            let op = checker.binaries.get_mut(&id).unwrap();
            match fault {
                0 => {
                    op.types.inputs = [Class::List {
                        capacity: crate::list::MAX_CAPACITY + 1,
                    }; 2]
                }
                1 => {
                    op.types.inputs = [Class::Record {
                        fields: MAX_COUNT + 1,
                    }; 2]
                }
                2..=4 => {
                    op.types.inputs = [Class::Union {
                        members: [0, 1, MAX_COUNT + 1][fault - 2],
                    }; 2]
                }
                5 => op.types.inputs[1] = Class::Record { fields: 2 },
                6 => {
                    op.types.inputs = [
                        Class::Reference(crate::hir::ReferenceMode::Shared),
                        Class::Reference(crate::hir::ReferenceMode::Exclusive),
                    ]
                }
                7 => op.plan.equality = false,
                8 => op.plan.checked = true,
                9 => op.types.result = Class::List { capacity: 2 },
                10 => op.plan.normal[0] = false,
                11 => op.plan.primary[1] = true,
                12 => {
                    op.op = "<".into();
                    op.plan.equality = false;
                }
                13 => {
                    op.op = "+".into();
                    op.plan.equality = false;
                    op.types.result = op.types.inputs[0];
                }
                14 => op.types.inputs = [Class::Scalar(ScalarKind::Float { bits: 16 }); 2],
                15 => op.types.inputs[1] = Class::List { capacity: 1 },
                _ => unreachable!(),
            }
            let before = reports.effects.clone();
            let ops = checker.binaries.clone();
            let counts = checker.edge_counts();
            reports.entries.get_mut(&0).unwrap().1.ports = vec![if result {
                Port::Normal(id)
            } else {
                Port::Operation(id)
            }];
            assert!(
                checker
                    .operation_effects(&reports, Span::default())
                    .unwrap_err()
                    .message
                    .contains("identity"),
                "fault {fault}"
            );
            assert_eq!(reports.effects, before);
            assert_eq!(checker.binaries, ops);
            assert_eq!(checker.edge_counts(), counts);
        }
    }
}

#[test]
pub(crate) fn equality_effects_keep_composed_order_calls_owner_and_control() {
    let source = "flag:false;make<{n<int32>}>:(){->n:1};|flag|x:make()=={->n:2};f<boolean>:(a<int32><null>,b<int32><null>){->a==b}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.binaries.len(), 2);
    assert!(checker.binaries.values().any(|op| op.control));
    assert!(checker.binaries.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.binaries {
        let (owner, Effect::Binary(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert!(observed.plan.equality);
        let sequence = &checker.sequences[&SequenceSource::Expr(id)];
        assert_eq!(sequence.items, op.inputs.map(Some));
        assert_eq!(
            sequence.edges,
            [Edge::new(
                Port::Normal(op.inputs[0]),
                Port::Entry(op.inputs[1]),
                Route::Next
            )]
        );
    }
    assert_eq!(checker.invocations.len(), 1);
    let call = checker.invocations.values().next().unwrap();
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert!(matches!(
        reports.effects[&call.point].1,
        Effect::Call { .. }
    ));
}

#[test]
pub(crate) fn equality_categories_do_not_replace_complete_type_checks() {
    use crate::hir::{Expr, ExprKind, Field, Type};
    let int = Type::Int {
        bits: 32,
        signed: true,
    };
    for (left, right) in [
        (
            Type::Record {
                primary: Box::new(Type::Null),
                fields: vec![Field {
                    name: "n".into(),
                    ty: int.clone(),
                    mutable: false,
                }],
            },
            Type::Record {
                primary: Box::new(Type::Null),
                fields: vec![Field {
                    name: "n".into(),
                    ty: Type::Bool,
                    mutable: false,
                }],
            },
        ),
        (
            Type::List {
                element: Box::new(int.clone()),
                capacity: 1,
            },
            Type::List {
                element: Box::new(Type::Bool),
                capacity: 1,
            },
        ),
        (
            Type::Reference(Box::new(int.clone())),
            Type::Reference(Box::new(Type::Bool)),
        ),
        (
            Type::union(vec![int, Type::Null]),
            Type::union(vec![Type::Bool, Type::Null]),
        ),
    ] {
        let mut checker = Checker::new();
        let left = Expr {
            kind: ExprKind::Local(0),
            ty: left,
            span: Span::default(),
        };
        let right = Expr {
            kind: ExprKind::Local(1),
            ty: right,
            span: Span::default(),
        };
        assert_eq!(
            checker
                .binary_plan_values("==", left, right, Span::default())
                .unwrap_err()
                .code,
            "E222"
        );
        assert!(checker.binaries.is_empty());
    }
}
