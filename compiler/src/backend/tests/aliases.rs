use super::fields::{named_record, print};
use super::*;

pub(crate) fn local(id: usize, ty: &Type) -> Expr {
    expr(ExprKind::Local(id), ty.clone())
}

pub(crate) fn block(id: usize, ty: &Type, stmts: Vec<Stmt>) -> Expr {
    expr(
        ExprKind::Block(Block {
            id,
            ty: ty.clone(),
            stmts,
        }),
        ty.clone(),
    )
}

pub(crate) fn field(value: Expr, index: usize) -> Expr {
    let Type::Record { fields, .. } = &value.ty else {
        unreachable!()
    };
    let ty = fields[index].ty.clone();
    expr(
        ExprKind::Field {
            value: Box::new(value),
            index,
        },
        ty,
    )
}

pub(crate) fn shape(name: &str, ty: Type, mutable: bool) -> Type {
    Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![Field {
            name: name.into(),
            ty,
            mutable,
        }],
    }
}

pub(crate) fn alias(id: usize, target: usize, name: &str, value: Expr) -> Vec<Stmt> {
    slot_alias(id, target, name, value, true)
}

pub(crate) fn slot_alias(
    id: usize,
    target: usize,
    name: &str,
    value: Expr,
    mutable: bool,
) -> Vec<Stmt> {
    let ty = value.ty.clone();
    vec![
        Stmt::Bind { id, value },
        Stmt::Emit {
            id: 0,
            target,
            field: Some(name.into()),
            value: local(id, &ty),
        },
        Stmt::SlotAlias {
            id,
            target,
            field: name.into(),
            mutable,
        },
    ]
}

pub(crate) fn initial(value: i128) -> Expr {
    let ty = integer(value, 32, true).ty;
    block(
        3,
        &ty,
        vec![
            print(vec![expr(ExprKind::String("init".into()), Type::String)]),
            Stmt::Emit {
                id: 0,
                target: 3,
                field: None,
                value: integer(value, 32, true),
            },
        ],
    )
}

#[test]
pub(crate) fn mutable_emissions_alias_same_outer_and_function_result_cells() {
    let int = integer(0, 32, true).ty;
    let ty = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![
            Field {
                name: "after".into(),
                ty: int.clone(),
                mutable: false,
            },
            Field {
                name: "value".into(),
                ty: int.clone(),
                mutable: true,
            },
        ],
    };
    for context in 0..4 {
        let stored = if context == 3 {
            Type::union([ty.clone(), shape("text", Type::String, false)])
        } else {
            ty.clone()
        };
        let mut stmts = alias(0, 1, "value", initial(10));
        stmts.push(Stmt::Assign {
            id: 0,
            value: binary("+", local(0, &int), integer(1, 32, true), int.clone()),
        });
        stmts.push(print(vec![local(0, &int)]));
        if context == 1 {
            stmts = vec![Stmt::Expr(block(2, &Type::Null, stmts))];
        }
        stmts.push(Stmt::Emit {
            id: 1,
            target: 1,
            field: Some("after".into()),
            value: integer(20, 32, true),
        });
        let result = block(1, &ty, stmts);
        let mut functions = Vec::new();
        let value = if context >= 2 {
            let ExprKind::Block(body) = result.kind else {
                unreachable!()
            };
            functions.push(crate::hir::Function {
                id: 0,
                name: "result".into(),
                params: Vec::new(),
                result: stored.clone(),
                body,
            });
            expr(
                ExprKind::Call {
                    id: 0,
                    site: 0,
                    args: Vec::new(),
                },
                stored.clone(),
            )
        } else {
            result
        };
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind { id: 1, value },
                    Stmt::If {
                        point: None,
                        condition: type_test(local(1, &stored), ty.clone()),
                        then: vec![print(separated(vec![
                            field(coerce(local(1, &stored), &ty), 1),
                            field(coerce(local(1, &stored), &ty), 0),
                        ]))],
                        otherwise: Vec::new(),
                    },
                ],
            },
            functions,
            locals: vec![int.clone(), stored],
        };
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"init\n11\n11|20|\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn widened_result_aliases_retag_values_and_address_actual_aggregate_payloads() {
    let int = integer(0, 32, true).ty;
    let small = Type::union([Type::Bool, Type::String]);
    let record = |id, value| {
        named_record(
            id,
            integer(7, 8, false),
            vec![
                ("cell", integer(value, 32, true), true),
                (
                    "stamp",
                    expr(ExprKind::String("keep".into()), Type::String),
                    false,
                ),
            ],
        )
    };
    let cases = [
        (
            list(
                int.clone(),
                4,
                vec![integer(1, 32, true), integer(2, 32, true)],
            ),
            list(
                int,
                4,
                vec![
                    integer(9, 32, true),
                    integer(2, 32, true),
                    integer(3, 32, true),
                ],
            ),
            0,
        ),
        (record(20, 1), record(21, 9), 1),
        (
            coerce(expr(ExprKind::String("old".into()), Type::String), &small),
            coerce(expr(ExprKind::Bool(true), Type::Bool), &small),
            2,
        ),
    ];
    for (value, expected, kind) in cases {
        let lexical = value.ty.clone();
        let stored = Type::union([Type::Null, lexical.clone()]);
        let ty = shape("value", stored.clone(), true);
        let mut stmts = alias(0, 1, "value", value);
        if kind == 2 {
            stmts.push(Stmt::Assign {
                id: 0,
                value: expected.clone(),
            });
        } else {
            let path = if kind == 0 {
                vec![WriteStep::Index(IndexStep {
                    index: integer(1, 8, true),
                    span: Span { start: 10, end: 20 },
                })]
            } else {
                vec![WriteStep::Field(0)]
            };
            stmts.push(Stmt::SetPath {
                id: 0,
                path,
                value: integer(9, 32, true),
                span: Span { start: 10, end: 20 },
            });
            if kind == 0 {
                stmts.push(Stmt::Assign {
                    id: 0,
                    value: expr(
                        ExprKind::ListAdd {
                            value: Box::new(local(0, &lexical)),
                            item: Box::new(integer(3, 32, true)),
                        },
                        lexical.clone(),
                    ),
                });
            }
        }
        stmts.push(print(vec![binary(
            "==",
            local(0, &lexical),
            expected.clone(),
            Type::Bool,
        )]));
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind {
                        id: 1,
                        value: block(1, &ty, stmts),
                    },
                    print(vec![binary(
                        "==",
                        coerce(field(local(1, &ty), 0), &lexical),
                        expected,
                        Type::Bool,
                    )]),
                ],
            },
            functions: Vec::new(),
            locals: vec![lexical, ty],
        };
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"true\ntrue\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn guarded_alias_initializers_preserve_optional_defaults() {
    let int = integer(0, 32, true).ty;
    let ty = shape("value", Type::union([Type::Null, int.clone()]), true);
    for present in [false, true] {
        let mut then = alias(1, 1, "value", initial(5));
        then.push(Stmt::Assign {
            id: 1,
            value: integer(9, 32, true),
        });
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind {
                        id: 0,
                        value: expr(ExprKind::Bool(present), Type::Bool),
                    },
                    Stmt::Bind {
                        id: 2,
                        value: block(
                            1,
                            &ty,
                            vec![Stmt::If {
                                point: None,
                                condition: local(0, &Type::Bool),
                                then,
                                otherwise: Vec::new(),
                            }],
                        ),
                    },
                    print(vec![field(local(2, &ty), 0)]),
                ],
            },
            functions: Vec::new(),
            locals: vec![Type::Bool, int.clone(), ty.clone()],
        };
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                if present {
                    b"init\n9\n".as_slice()
                } else {
                    b"null\n".as_slice()
                }
            );
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn restart_reinitializes_alias_cells_and_discarded_defaults() {
    let int = integer(0, 32, true).ty;
    let optional = Type::union([Type::Null, Type::String]);
    let ty = Type::Record {
        primary: Box::new(Type::Null),
        fields: vec![
            Field {
                name: "optional".into(),
                ty: optional,
                mutable: false,
            },
            Field {
                name: "value".into(),
                ty: int.clone(),
                mutable: true,
            },
        ],
    };
    let mut stmts = alias(0, 1, "value", initial(5));
    stmts.push(Stmt::Assign {
        id: 0,
        value: binary("+", local(0, &int), integer(1, 32, true), int.clone()),
    });
    stmts.push(Stmt::If {
        point: None,
        condition: local(1, &Type::Bool),
        then: vec![
            Stmt::Emit {
                id: 1,
                target: 1,
                field: Some("optional".into()),
                value: expr(ExprKind::String("discarded".into()), Type::String),
            },
            Stmt::Assign {
                id: 0,
                value: integer(99, 32, true),
            },
            Stmt::Assign {
                id: 1,
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
                    id: 1,
                    value: expr(ExprKind::Bool(true), Type::Bool),
                },
                Stmt::Bind {
                    id: 2,
                    value: block(1, &ty, stmts),
                },
                print(separated(vec![
                    field(local(2, &ty), 1),
                    field(local(2, &ty), 0),
                ])),
            ],
        },
        functions: Vec::new(),
        locals: vec![int, Type::Bool, ty],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"init\ninit\n6|null|\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn discarded_aliases_keep_valid_local_storage_until_exit() {
    let int = integer(0, 32, true).ty;
    let cases = [
        (
            shape("other", int.clone(), false),
            Some("other"),
            integer(7, 32, true),
        ),
        (
            shape("value", int.clone(), true),
            Some("value"),
            integer(7, 32, true),
        ),
        (
            shape("value", Type::String, false),
            Some("value"),
            expr(ExprKind::String("normal".into()), Type::String),
        ),
        (
            Type::union([int.clone(), Type::String]),
            None,
            integer(7, 32, true),
        ),
    ];
    for (ty, name, normal) in cases {
        for discard in [false, true] {
            let mut then = alias(
                0,
                1,
                "value",
                expr(ExprKind::String("start".into()), Type::String),
            );
            then.push(Stmt::Assign {
                id: 0,
                value: expr(ExprKind::String("changed".into()), Type::String),
            });
            then.push(print(vec![local(0, &Type::String)]));
            then.push(Stmt::Leave(0));
            let result = local(1, &ty);
            let result = if name.is_some() {
                field(result, 0)
            } else {
                result
            };
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 2,
                            value: expr(ExprKind::Bool(discard), Type::Bool),
                        },
                        Stmt::Bind {
                            id: 1,
                            value: block(
                                1,
                                &ty,
                                vec![Stmt::If {
                                    point: None,
                                    condition: local(2, &Type::Bool),
                                    then,
                                    otherwise: vec![Stmt::Emit {
                                        id: 1,
                                        target: 1,
                                        field: name.map(str::to_owned),
                                        value: normal.clone(),
                                    }],
                                }],
                            ),
                        },
                        print(vec![result]),
                    ],
                },
                functions: Vec::new(),
                locals: vec![Type::String, ty.clone(), Type::Bool],
            };
            for release in [false, true] {
                let output = native_program(&program, release, false);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let expected = if discard {
                    b"changed\n".as_slice()
                } else if normal.ty == Type::String {
                    b"normal\n".as_slice()
                } else {
                    b"7\n".as_slice()
                };
                assert_eq!(output.stdout, expected);
                assert!(output.stderr.is_empty());
            }
        }
    }
    let value = list(int.clone(), 2, vec![integer(1, 32, true)]);
    let mut stmts = alias(0, 1, "value", value.clone());
    stmts.push(Stmt::SetPath {
        id: 0,
        path: vec![WriteStep::Index(IndexStep {
            index: integer(1, 32, true),
            span: Span { start: 10, end: 20 },
        })],
        value: integer(9, 32, true),
        span: Span { start: 10, end: 20 },
    });
    stmts.push(print(vec![index(
        local(0, &value.ty),
        integer(1, 32, true),
    )]));
    stmts.push(Stmt::Expr(Expr {
        kind: ExprKind::Panic {
            parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
        },
        ty: Type::Never,
        span: Span { start: 30, end: 40 },
    }));
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Never,
            stmts: vec![Stmt::Expr(block(1, &Type::Never, stmts))],
        },
        functions: Vec::new(),
        locals: vec![value.ty],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"9\n");
        assert_eq!(output.stderr, b"panic[P006]: stop at bytes 30..40\n");
    }
}
