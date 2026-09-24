use super::aliases::{block, field, local, shape, slot_alias};
use super::fields::{named_record, print};
use super::*;

pub(crate) fn deref(value: Expr) -> Expr {
    let Type::Reference(ty) = &value.ty else {
        unreachable!()
    };
    let ty = *ty.clone();
    expr(ExprKind::Deref(Box::new(value)), ty)
}

pub(crate) fn carrier(number: i128) -> Expr {
    named_record(
        20,
        expr(ExprKind::Null, Type::Null),
        vec![
            ("number", integer(number, 32, true), false),
            ("view", borrow(0, integer(0, 32, true).ty), false),
        ],
    )
}

#[test]
pub(crate) fn emitted_reference_values_preserve_external_pointer_identity_and_reborrows() {
    let int = integer(0, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let row = named_record(
        10,
        expr(ExprKind::Bool(true), Type::Bool),
        vec![
            ("number", integer(77, 32, true), false),
            (
                "stamp",
                expr(ExprKind::String("keep".into()), Type::String),
                false,
            ),
        ],
    );
    for (owner, projected) in [(integer(77, 32, true), false), (row, true)] {
        for widened in [false, true] {
            let reference = Type::Reference(Box::new(owner.ty.clone()));
            let stored = if widened {
                Type::union([Type::Null, reference.clone()])
            } else {
                reference.clone()
            };
            let result = shape("view", stored, false);
            let initializer = block(
                3,
                &reference,
                vec![
                    print(vec![expr(ExprKind::String("init".into()), Type::String)]),
                    Stmt::Emit {
                        id: 0,
                        target: 3,
                        field: None,
                        value: borrow(0, owner.ty.clone()),
                    },
                ],
            );
            let mut inner = slot_alias(1, 1, "view", initializer, false);
            let expected = expr(
                ExprKind::Borrow(Place {
                    root: 0,
                    fields: if projected { vec![0] } else { Vec::new() },
                }),
                pointer.clone(),
            );
            let selected = |value| {
                expr(
                    ExprKind::Reborrow {
                        site: 0,
                        value: Box::new(value),
                        fields: if projected { vec![0] } else { Vec::new() },
                    },
                    pointer.clone(),
                )
            };
            let value = selected(local(1, &reference));
            inner.push(print(separated(vec![
                binary(
                    "==",
                    local(1, &reference),
                    borrow(0, owner.ty.clone()),
                    Type::Bool,
                ),
                binary("==", value.clone(), expected.clone(), Type::Bool),
                deref(value.clone()),
            ])));
            inner.push(Stmt::Emit {
                id: 1,
                target: 2,
                field: None,
                value,
            });
            let outer = block(
                1,
                &result,
                vec![
                    Stmt::Bind {
                        id: 3,
                        value: block(2, &pointer, inner),
                    },
                    print(vec![deref(local(3, &pointer))]),
                ],
            );
            let returned = selected(coerce(field(local(2, &result), 0), &reference));
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: owner.clone(),
                        },
                        Stmt::Bind {
                            id: 2,
                            value: outer,
                        },
                        print(separated(vec![
                            binary("==", returned.clone(), expected, Type::Bool),
                            deref(returned),
                        ])),
                    ],
                },
                functions: Vec::new(),
                locals: vec![owner.ty.clone(), reference, result, pointer.clone()],
            };
            assert_eq!(
                emit_ir(&program)
                    .unwrap()
                    .matches("load ptr, ptr %local1\n")
                    .count(),
                1
            );
            for release in [false, true] {
                let output = native_program(&program, release, false);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, b"init\ntrue|true|77|\n77\ntrue|77|\n");
                assert!(output.stderr.is_empty());
            }
        }
    }
}

#[test]
pub(crate) fn emitted_carrier_scalar_addresses_and_stored_reference_values_stay_distinct() {
    let int = integer(0, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let value = carrier(42);
    for widened in [false, true] {
        let stored = if widened {
            Type::union([Type::Null, value.ty.clone()])
        } else {
            value.ty.clone()
        };
        let result = shape("carrier", stored, false);
        let mut inner = slot_alias(1, 1, "carrier", value.clone(), false);
        inner.push(Stmt::Bind {
            id: 4,
            value: local(1, &value.ty),
        });
        let scalar = expr(
            ExprKind::Borrow(Place {
                root: 1,
                fields: vec![0],
            }),
            pointer.clone(),
        );
        let copied = expr(
            ExprKind::Borrow(Place {
                root: 4,
                fields: vec![0],
            }),
            pointer.clone(),
        );
        let carried = field(local(1, &value.ty), 1);
        inner.push(print(separated(vec![
            binary("==", scalar.clone(), scalar.clone(), Type::Bool),
            binary("==", scalar.clone(), copied, Type::Bool),
            binary("==", carried.clone(), borrow(0, int.clone()), Type::Bool),
            deref(carried),
        ])));
        inner.push(Stmt::Emit {
            id: 1,
            target: 2,
            field: None,
            value: scalar,
        });
        let outer = block(
            1,
            &result,
            vec![
                Stmt::Bind {
                    id: 2,
                    value: block(2, &pointer, inner),
                },
                print(vec![deref(local(2, &pointer))]),
            ],
        );
        let returned = coerce(field(local(3, &result), 0), &value.ty);
        let carried = field(returned.clone(), 1);
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind {
                        id: 0,
                        value: integer(77, 32, true),
                    },
                    Stmt::Bind {
                        id: 3,
                        value: outer,
                    },
                    print(separated(vec![
                        binary("==", carried.clone(), borrow(0, int.clone()), Type::Bool),
                        deref(carried),
                        field(returned, 0),
                    ])),
                ],
            },
            functions: Vec::new(),
            locals: vec![
                int.clone(),
                value.ty.clone(),
                pointer.clone(),
                result,
                value.ty.clone(),
            ],
        };
        assert!(!emit_ir(&program).unwrap().contains(&format!(
            "getelementptr {}, ptr %local1",
            ir_type(&value.ty)
        )));
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"true|false|true|77|\n42\ntrue|77|42|\n");
            assert!(output.stderr.is_empty());
        }
    }
}

pub(crate) fn inspect(value: Expr, reference: &Type, record: &Type) -> Vec<Stmt> {
    let int = integer(0, 32, true).ty;
    let direct = coerce(value.clone(), reference);
    let carrier = coerce(value.clone(), record);
    let carried = field(carrier.clone(), 1);
    vec![
        print(separated(vec![
            type_test(value.clone(), Type::Null),
            type_test(value.clone(), reference.clone()),
            type_test(value.clone(), record.clone()),
        ])),
        Stmt::If {
            point: None,
            condition: type_test(value.clone(), Type::Null),
            then: vec![print(vec![expr(
                ExprKind::String("empty".into()),
                Type::String,
            )])],
            otherwise: Vec::new(),
        },
        Stmt::If {
            point: None,
            condition: type_test(value.clone(), reference.clone()),
            then: vec![print(separated(vec![
                binary("==", direct.clone(), borrow(0, int.clone()), Type::Bool),
                deref(direct),
            ]))],
            otherwise: Vec::new(),
        },
        Stmt::If {
            point: None,
            condition: type_test(value, record.clone()),
            then: vec![print(separated(vec![
                binary("==", carried.clone(), borrow(0, int), Type::Bool),
                deref(carried),
                field(carrier, 0),
            ]))],
            otherwise: Vec::new(),
        },
    ]
}

#[test]
pub(crate) fn emitted_reference_unions_preserve_active_variants_across_widened_cells() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let record = carrier(42);
    let small = Type::union([Type::Null, reference.clone(), record.ty.clone()]);
    for (value, expected) in [
        (
            expr(ExprKind::Null, Type::Null),
            "true|false|false|\nempty\n",
        ),
        (borrow(0, int.clone()), "false|true|false|\ntrue|77|\n"),
        (record.clone(), "false|false|true|\ntrue|77|42|\n"),
    ] {
        for widened in [false, true] {
            let stored = if widened {
                Type::union([small.clone(), Type::String])
            } else {
                small.clone()
            };
            let result = shape("value", stored, false);
            let mut stmts = slot_alias(1, 1, "value", coerce(value.clone(), &small), false);
            stmts.extend(inspect(local(1, &small), &reference, &record.ty));
            let mut body = vec![
                Stmt::Bind {
                    id: 0,
                    value: integer(77, 32, true),
                },
                Stmt::Bind {
                    id: 2,
                    value: block(1, &result, stmts),
                },
            ];
            body.extend(inspect(
                coerce(field(local(2, &result), 0), &small),
                &reference,
                &record.ty,
            ));
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: body,
                },
                functions: Vec::new(),
                locals: vec![int.clone(), small.clone(), result],
            };
            for release in [false, true] {
                let output = native_program(&program, release, false);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, expected.repeat(2).as_bytes());
                assert!(output.stderr.is_empty());
            }
        }
    }
}

#[test]
pub(crate) fn discarded_carrier_aliases_preserve_transient_scalar_and_external_reference_cells() {
    let int = integer(0, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let value = carrier(42);
    let returned = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![
            Field {
                name: "scalar".into(),
                ty: pointer.clone(),
                mutable: false,
            },
            Field {
                name: "view".into(),
                ty: pointer.clone(),
                mutable: false,
            },
        ],
    };
    let mut inner = slot_alias(1, 1, "carrier", value.clone(), false);
    inner.push(Stmt::Emit {
        id: 1,
        target: 2,
        field: Some("scalar".into()),
        value: expr(
            ExprKind::Borrow(Place {
                root: 1,
                fields: vec![0],
            }),
            pointer.clone(),
        ),
    });
    inner.push(Stmt::Emit {
        id: 2,
        target: 2,
        field: Some("view".into()),
        value: field(local(1, &value.ty), 1),
    });
    let outer = block(
        1,
        &Type::Never,
        vec![
            Stmt::Bind {
                id: 2,
                value: block(2, &returned, inner),
            },
            print(separated(vec![
                deref(field(local(2, &returned), 0)),
                binary(
                    "==",
                    field(local(2, &returned), 1),
                    borrow(0, int.clone()),
                    Type::Bool,
                ),
                deref(field(local(2, &returned), 1)),
            ])),
            Stmt::Expr(Expr {
                kind: ExprKind::Panic {
                    parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
                },
                ty: Type::Never,
                span: Span { start: 30, end: 40 },
            }),
        ],
    );
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Never,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: integer(77, 32, true),
                },
                Stmt::Expr(outer),
            ],
        },
        functions: Vec::new(),
        locals: vec![int, value.ty, returned],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"42|true|77|\n");
        assert_eq!(output.stderr, b"panic[P006]: stop at bytes 30..40\n");
    }
}

#[test]
pub(crate) fn reference_alias_restart_reloads_the_current_referent() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let result = shape("view", Type::union([Type::Null, reference.clone()]), false);
    let initializer = block(
        3,
        &reference,
        vec![
            print(vec![expr(ExprKind::String("init".into()), Type::String)]),
            Stmt::If {
                point: None,
                condition: local(3, &Type::Bool),
                then: vec![Stmt::Emit {
                    id: 0,
                    target: 3,
                    field: None,
                    value: borrow(0, int.clone()),
                }],
                otherwise: vec![Stmt::Emit {
                    id: 1,
                    target: 3,
                    field: None,
                    value: borrow(1, int.clone()),
                }],
            },
        ],
    );
    let mut stmts = slot_alias(2, 1, "view", initializer, false);
    stmts.push(print(vec![deref(local(2, &reference))]));
    stmts.push(Stmt::If {
        point: None,
        condition: local(3, &Type::Bool),
        then: vec![
            Stmt::Assign {
                id: 3,
                value: expr(ExprKind::Bool(false), Type::Bool),
            },
            Stmt::Restart { target: 1, site: 0 },
        ],
        otherwise: Vec::new(),
    });
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: integer(77, 32, true),
                },
                Stmt::Bind {
                    id: 1,
                    value: integer(88, 32, true),
                },
                Stmt::Bind {
                    id: 3,
                    value: expr(ExprKind::Bool(true), Type::Bool),
                },
                Stmt::Bind {
                    id: 4,
                    value: block(1, &result, stmts),
                },
                print(vec![deref(coerce(field(local(4, &result), 0), &reference))]),
            ],
        },
        functions: Vec::new(),
        locals: vec![int.clone(), int, reference, Type::Bool, result],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"init\n77\ninit\n88\n88\n");
        assert!(output.stderr.is_empty());
    }
}
