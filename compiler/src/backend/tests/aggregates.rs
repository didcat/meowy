use super::*;

#[test]
pub(crate) fn union_scalars_retag_and_extract_in_each_profile() {
    let small = Type::union([Type::Null, Type::String]);
    let wide = Type::union([
        Type::Null,
        Type::Bool,
        Type::Int {
            bits: 8,
            signed: true,
        },
        Type::Int {
            bits: 64,
            signed: false,
        },
        Type::Float { bits: 32 },
        Type::Float { bits: 64 },
        Type::String,
    ]);
    let text = expr(ExprKind::String("猫\0".into()), Type::String);
    let value = coerce(coerce(text, &small), &wide);
    for release in [false, true] {
        let output = native(
            vec![
                coerce(expr(ExprKind::Null, Type::Null), &wide),
                coerce(expr(ExprKind::Bool(true), Type::Bool), &wide),
                coerce(integer(-128, 8, true), &wide),
                coerce(integer(u64::MAX.into(), 64, false), &wide),
                coerce(expr(ExprKind::Float(1.25), Type::Float { bits: 32 }), &wide),
                coerce(expr(ExprKind::Float(-0.0), Type::Float { bits: 64 }), &wide),
                type_test(value.clone(), Type::String),
                type_test(value.clone(), Type::Null),
                coerce(coerce(value.clone(), &small), &Type::String),
            ],
            release,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.stdout,
            "nulltrue-128184467440737095516151.25-0truefalse猫\0\n".as_bytes()
        );
    }
}

#[test]
pub(crate) fn union_records_preserve_padding_and_compare_active_payloads() {
    let field = Type::union([Type::Null, Type::String]);
    let first = record(
        1,
        integer(7, 8, false),
        coerce(expr(ExprKind::String("one".into()), Type::String), &field),
    );
    let other = record(
        2,
        integer(7, 8, false),
        coerce(expr(ExprKind::String("two".into()), Type::String), &field),
    );
    let nested = record(3, integer(9, 8, false), first.clone());
    let ty = Type::union([
        Type::Null,
        Type::String,
        first.ty.clone(),
        nested.ty.clone(),
    ]);
    let a = coerce(first.clone(), &ty);
    let b = coerce(other, &ty);
    let none = coerce(expr(ExprKind::Null, Type::Null), &ty);
    let text = coerce(expr(ExprKind::String("hello".into()), Type::String), &ty);
    for release in [false, true] {
        let output = native(
            vec![
                a.clone(),
                coerce(nested.clone(), &ty),
                type_test(a.clone(), Type::Null),
                type_test(a.clone(), first.ty.clone()),
                binary("==", a.clone(), a.clone(), Type::Bool),
                binary("==", a.clone(), b.clone(), Type::Bool),
                binary("==", none.clone(), a.clone(), Type::Bool),
                binary("==", none.clone(), text.clone(), Type::Bool),
                binary("==", none.clone(), none.clone(), Type::Bool),
                binary("==", text.clone(), text.clone(), Type::Bool),
            ],
            release,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"79falsetruetruefalsefalsefalsetruetrue\n");
    }
}

#[test]
pub(crate) fn inferred_union_emissions_and_nullable_defaults_execute() {
    let ty = Type::union([Type::Null, Type::String]);
    let shape = Type::Record {
        primary: Box::new(ty.clone()),
        fields: vec![Field {
            name: "name".into(),
            ty: ty.clone(),
            mutable: false,
        }],
    };
    let value = |id, present| {
        expr(
            ExprKind::Block(Block {
                id,
                ty: shape.clone(),
                stmts: vec![Stmt::If {
                    point: None,
                    condition: expr(ExprKind::Bool(present), Type::Bool),
                    then: vec![Stmt::Emit {
                        id: 0,
                        target: id,
                        field: Some("name".into()),
                        value: expr(ExprKind::String("hello".into()), Type::String),
                    }],
                    otherwise: Vec::new(),
                }],
            }),
            shape.clone(),
        )
    };
    for release in [false, true] {
        let output = native(
            vec![
                expr(
                    ExprKind::Field {
                        value: Box::new(value(1, true)),
                        index: 0,
                    },
                    ty.clone(),
                ),
                expr(
                    ExprKind::Field {
                        value: Box::new(value(2, false)),
                        index: 0,
                    },
                    ty.clone(),
                ),
                value(3, true),
                expr(
                    ExprKind::Block(Block {
                        id: 4,
                        ty: ty.clone(),
                        stmts: vec![Stmt::Emit {
                            id: 0,
                            target: 4,
                            field: None,
                            value: expr(ExprKind::String("primary".into()), Type::String),
                        }],
                    }),
                    ty.clone(),
                ),
            ],
            release,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"hellonullnullprimary\n");
    }
}

#[test]
pub(crate) fn union_storage_and_restart_defaults_execute() {
    let ty = Type::union([Type::Null, Type::String]);
    let shape = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![Field {
            name: "name".into(),
            ty: ty.clone(),
            mutable: false,
        }],
    };
    let block = expr(
        ExprKind::Block(Block {
            id: 1,
            ty: shape.clone(),
            stmts: vec![Stmt::If {
                point: None,
                condition: expr(
                    ExprKind::Unary {
                        op: "!".into(),
                        value: Box::new(expr(ExprKind::Local(1), Type::Bool)),
                    },
                    Type::Bool,
                ),
                then: vec![
                    Stmt::Emit {
                        id: 0,
                        target: 1,
                        field: Some("name".into()),
                        value: expr(ExprKind::String("discarded".into()), Type::String),
                    },
                    Stmt::Assign {
                        id: 1,
                        value: expr(ExprKind::Bool(true), Type::Bool),
                    },
                    Stmt::Restart { target: 1, site: 0 },
                ],
                otherwise: Vec::new(),
            }],
        }),
        shape,
    );
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: expr(ExprKind::String("stored".into()), Type::String),
                },
                Stmt::Bind {
                    id: 1,
                    value: expr(ExprKind::Bool(false), Type::Bool),
                },
                Stmt::Expr(expr(
                    ExprKind::Print {
                        parts: vec![
                            expr(ExprKind::Local(0), Type::String),
                            expr(ExprKind::Local(0), ty.clone()),
                            expr(
                                ExprKind::Field {
                                    value: Box::new(block),
                                    index: 0,
                                },
                                ty.clone(),
                            ),
                        ],
                        newline: true,
                    },
                    Type::Null,
                )),
                Stmt::Assign {
                    id: 0,
                    value: expr(ExprKind::Null, Type::Null),
                },
                Stmt::Expr(expr(
                    ExprKind::Print {
                        parts: vec![expr(ExprKind::Local(0), ty.clone())],
                        newline: true,
                    },
                    Type::Null,
                )),
            ],
        },
        functions: Vec::new(),
        locals: vec![ty, Type::Bool],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"storedstorednull\nnull\n");
    }
}

#[test]
pub(crate) fn coercions_and_type_tests_evaluate_operands_once() {
    let ty = Type::union([Type::Null, Type::String]);
    let value = expr(
        ExprKind::Block(Block {
            id: 1,
            ty: Type::String,
            stmts: vec![
                Stmt::Expr(expr(
                    ExprKind::Print {
                        parts: vec![expr(ExprKind::String("once:".into()), Type::String)],
                        newline: false,
                    },
                    Type::Null,
                )),
                Stmt::Emit {
                    id: 0,
                    target: 1,
                    field: None,
                    value: expr(ExprKind::String("value".into()), Type::String),
                },
            ],
        }),
        Type::String,
    );
    for release in [false, true] {
        let output = native(
            vec![
                type_test(coerce(value.clone(), &ty), Type::String),
                type_test(value.clone(), Type::Null),
            ],
            release,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"once:trueonce:false\n");
    }
}

#[test]
pub(crate) fn union_function_arguments_and_results_execute() {
    let small = Type::union([Type::Null, Type::String]);
    let wide = Type::union([Type::Null, Type::Bool, Type::String]);
    let call = |value| {
        expr(
            ExprKind::Call {
                id: 0,
                site: 0,
                args: vec![coerce(value, &small)],
            },
            wide.clone(),
        )
    };
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![Stmt::Expr(expr(
                ExprKind::Print {
                    parts: vec![
                        call(expr(ExprKind::String("argument".into()), Type::String)),
                        call(expr(ExprKind::Null, Type::Null)),
                        coerce(
                            call(expr(ExprKind::String("result".into()), Type::String)),
                            &Type::String,
                        ),
                    ],
                    newline: true,
                },
                Type::Null,
            ))],
        },
        functions: vec![crate::hir::Function {
            id: 0,
            name: "widen".into(),
            params: vec![0],
            result: wide.clone(),
            body: Block {
                id: 1,
                ty: wide,
                stmts: vec![Stmt::Emit {
                    id: 0,
                    target: 1,
                    field: None,
                    value: expr(ExprKind::Local(0), small.clone()),
                }],
            },
        }],
        locals: vec![small],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"argumentnullresult\n");
    }
}

#[test]
pub(crate) fn discarded_emission_slots_preserve_operand_effects() {
    let effect = |id, text: &str| {
        expr(
            ExprKind::Block(Block {
                id,
                ty: Type::String,
                stmts: vec![
                    Stmt::Expr(expr(
                        ExprKind::Print {
                            parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                            newline: false,
                        },
                        Type::Null,
                    )),
                    Stmt::Emit {
                        id: 0,
                        target: id,
                        field: None,
                        value: expr(ExprKind::String("discarded".into()), Type::String),
                    },
                ],
            }),
            Type::String,
        )
    };
    let ty = Type::Int {
        bits: 32,
        signed: true,
    };
    let value = expr(
        ExprKind::Block(Block {
            id: 1,
            ty: ty.clone(),
            stmts: vec![
                Stmt::If {
                    point: None,
                    condition: expr(
                        ExprKind::Unary {
                            op: "!".into(),
                            value: Box::new(expr(ExprKind::Local(0), Type::Bool)),
                        },
                        Type::Bool,
                    ),
                    then: vec![
                        Stmt::Emit {
                            id: 0,
                            target: 1,
                            field: None,
                            value: coerce(
                                effect(2, "primary:"),
                                &Type::union([Type::Bool, Type::String]),
                            ),
                        },
                        Stmt::Emit {
                            id: 0,
                            target: 1,
                            field: Some("absent".into()),
                            value: effect(3, "field:"),
                        },
                        Stmt::Assign {
                            id: 0,
                            value: expr(ExprKind::Bool(true), Type::Bool),
                        },
                        Stmt::Restart { target: 1, site: 0 },
                    ],
                    otherwise: Vec::new(),
                },
                Stmt::Emit {
                    id: 0,
                    target: 1,
                    field: None,
                    value: integer(7, 32, true),
                },
            ],
        }),
        ty,
    );
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: expr(ExprKind::Bool(false), Type::Bool),
                },
                Stmt::Expr(expr(
                    ExprKind::Print {
                        parts: vec![value],
                        newline: true,
                    },
                    Type::Null,
                )),
            ],
        },
        functions: Vec::new(),
        locals: vec![Type::Bool],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"primary:field:7\n");
    }
}
