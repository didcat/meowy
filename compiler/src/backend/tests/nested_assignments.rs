use super::*;

#[test]
pub(crate) fn nested_element_assignments_keep_selected_rows_and_payload_layouts() {
    let print = |text: &str| {
        Stmt::Expr(expr(
            ExprKind::Print {
                parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                newline: true,
            },
            Type::Null,
        ))
    };
    let a = record(
        10,
        expr(ExprKind::Bool(false), Type::Bool),
        expr(ExprKind::String("a".into()), Type::String),
    );
    let c = record(
        11,
        expr(ExprKind::Bool(false), Type::Bool),
        expr(ExprKind::String("c".into()), Type::String),
    );
    let changed = record(
        12,
        expr(ExprKind::Bool(true), Type::Bool),
        expr(ExprKind::String("changed".into()), Type::String),
    );
    let union = Type::union([Type::Null, a.ty.clone()]);
    let cases = [
        (
            integer(11, 32, true),
            integer(22, 32, true),
            integer(33, 32, true),
            integer(99, 32, true),
            false,
        ),
        (
            coerce(a, &union),
            coerce(expr(ExprKind::Null, Type::Null), &union),
            coerce(c, &union),
            coerce(changed, &union),
            true,
        ),
    ];
    let int = integer(0, 32, true).ty;
    for release in [false, true] {
        for (a, b, c, changed, deep) in &cases {
            let first = list(a.ty.clone(), 4, vec![a.clone()]);
            let second = list(a.ty.clone(), 4, vec![a.clone(), b.clone(), c.clone()]);
            let expected = list(a.ty.clone(), 4, vec![a.clone(), b.clone(), changed.clone()]);
            let mut initial = list(first.ty.clone(), 3, vec![first.clone(), second]);
            let mut expected = list(first.ty.clone(), 3, vec![first, expected]);
            if *deep {
                initial = list(initial.ty.clone(), 2, vec![initial]);
                expected = list(expected.ty.clone(), 2, vec![expected]);
            }
            let outer = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: int.clone(),
                    stmts: vec![
                        print("outer"),
                        Stmt::Emit {
                            id: 0,
                            target: 1,
                            field: None,
                            value: expr(ExprKind::Local(2), int.clone()),
                        },
                    ],
                }),
                int.clone(),
            );
            let inner = expr(
                ExprKind::Block(Block {
                    id: 2,
                    ty: int.clone(),
                    stmts: vec![
                        print("inner"),
                        Stmt::Assign {
                            id: 2,
                            value: integer(1, 32, true),
                        },
                        Stmt::Emit {
                            id: 1,
                            target: 2,
                            field: None,
                            value: expr(ExprKind::Local(3), int.clone()),
                        },
                    ],
                }),
                int.clone(),
            );
            let value = expr(
                ExprKind::Block(Block {
                    id: 3,
                    ty: changed.ty.clone(),
                    stmts: vec![
                        print("rhs"),
                        Stmt::Assign {
                            id: 2,
                            value: integer(3, 32, true),
                        },
                        Stmt::Assign {
                            id: 3,
                            value: integer(1, 32, true),
                        },
                        Stmt::Emit {
                            id: 2,
                            target: 3,
                            field: None,
                            value: changed.clone(),
                        },
                    ],
                }),
                changed.ty.clone(),
            );
            let mut path = Vec::new();
            if *deep {
                path.push(WriteStep::Index(IndexStep {
                    index: integer(1, 64, false),
                    span: Span { start: 10, end: 15 },
                }));
            }
            path.push(WriteStep::Index(IndexStep {
                index: outer,
                span: Span { start: 10, end: 20 },
            }));
            path.push(WriteStep::Index(IndexStep {
                index: inner,
                span: Span { start: 10, end: 30 },
            }));
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: initial.clone(),
                        },
                        Stmt::Bind {
                            id: 1,
                            value: expr(ExprKind::Local(0), initial.ty.clone()),
                        },
                        Stmt::Bind {
                            id: 2,
                            value: integer(2, 32, true),
                        },
                        Stmt::Bind {
                            id: 3,
                            value: integer(3, 32, true),
                        },
                        Stmt::SetPath {
                            id: 0,
                            path,
                            value,
                            span: Span { start: 10, end: 30 },
                        },
                        Stmt::Expr(expr(
                            ExprKind::Print {
                                parts: separated(vec![
                                    binary(
                                        "==",
                                        expr(ExprKind::Local(0), initial.ty.clone()),
                                        expected,
                                        Type::Bool,
                                    ),
                                    binary(
                                        "==",
                                        expr(ExprKind::Local(1), initial.ty.clone()),
                                        initial.clone(),
                                        Type::Bool,
                                    ),
                                    expr(ExprKind::Local(2), int.clone()),
                                    expr(ExprKind::Local(3), int.clone()),
                                ]),
                                newline: true,
                            },
                            Type::Null,
                        )),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty.clone(), initial.ty, int.clone(), int.clone()],
            };
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"outer\ninner\nrhs\ntrue|true|3|1|\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn nested_assignment_bounds_use_each_header_and_prefix_before_later_effects() {
    let print = |text: &str| {
        Stmt::Expr(expr(
            ExprKind::Print {
                parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                newline: true,
            },
            Type::Null,
        ))
    };
    let effect = |id, text: &str, value: Expr| {
        expr(
            ExprKind::Block(Block {
                id,
                ty: value.ty.clone(),
                stmts: vec![
                    print(text),
                    Stmt::Emit {
                        id,
                        target: id,
                        field: None,
                        value: value.clone(),
                    },
                ],
            }),
            value.ty,
        )
    };
    let cases = [
        (
            integer(0, 8, false),
            integer(1, 32, true),
            false,
            "0",
            2,
            20,
        ),
        (
            integer(i64::MIN.into(), 64, true),
            integer(1, 32, true),
            false,
            "-9223372036854775808",
            2,
            20,
        ),
        (
            integer(3, 32, true),
            integer(1, 32, true),
            false,
            "3",
            2,
            20,
        ),
        (
            integer(1, 32, true),
            integer(2, 32, true),
            false,
            "2",
            1,
            30,
        ),
        (
            integer(2, 32, true),
            integer(-1, 8, true),
            false,
            "-1",
            3,
            30,
        ),
        (
            integer(2, 32, true),
            integer(u64::MAX.into(), 64, false),
            false,
            "18446744073709551615",
            3,
            30,
        ),
        (
            integer(1, 32, true),
            integer(1, 16, false),
            true,
            "1",
            0,
            30,
        ),
    ];
    for release in [false, true] {
        for (outer, inner, empty, number, length, end) in &cases {
            let row = if *empty {
                list(Type::Bool, 0, Vec::new())
            } else {
                list(Type::Bool, 4, vec![expr(ExprKind::Bool(false), Type::Bool)])
            };
            let other = if *empty {
                row.clone()
            } else {
                list(
                    Type::Bool,
                    4,
                    vec![expr(ExprKind::Bool(true), Type::Bool); 3],
                )
            };
            let initial = list(row.ty.clone(), 4, vec![row, other]);
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: initial.clone(),
                        },
                        Stmt::SetPath {
                            id: 0,
                            path: vec![
                                WriteStep::Index(IndexStep {
                                    index: effect(1, "outer", outer.clone()),
                                    span: Span { start: 10, end: 20 },
                                }),
                                WriteStep::Index(IndexStep {
                                    index: effect(2, "inner", inner.clone()),
                                    span: Span { start: 10, end: 30 },
                                }),
                            ],
                            value: effect(3, "rhs", expr(ExprKind::Bool(true), Type::Bool)),
                            span: Span { start: 9, end: 31 },
                        },
                        print("done"),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty],
            };
            let output = native_program(&program, release, false);
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(
                output.stdout,
                if *end == 20 {
                    b"outer\n".as_slice()
                } else {
                    b"outer\ninner\n".as_slice()
                }
            );
            assert_eq!(output.stderr, format!("panic[P001]: index {number} is outside initialized length {length} at bytes 10..{end}\n").as_bytes());
        }
    }
}

#[test]
pub(crate) fn nested_assignment_leaves_skip_remaining_indices_rhs_and_stores() {
    let print = |text: &str| {
        Stmt::Expr(expr(
            ExprKind::Print {
                parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                newline: true,
            },
            Type::Null,
        ))
    };
    for release in [false, true] {
        for leave in 0..3 {
            let row = list(
                Type::Bool,
                3,
                vec![expr(ExprKind::Bool(false), Type::Bool); 2],
            );
            let initial = list(row.ty.clone(), 3, vec![row.clone(), row]);
            let phase = |index, id, text: &str, value: Expr| {
                let ty = if index == leave {
                    Type::Never
                } else {
                    value.ty.clone()
                };
                expr(
                    ExprKind::Block(Block {
                        id,
                        ty: ty.clone(),
                        stmts: vec![
                            print(text),
                            if index == leave {
                                Stmt::Leave {
                                    target: 1,
                                    point: None,
                                }
                            } else {
                                Stmt::Emit {
                                    id,
                                    target: id,
                                    field: None,
                                    value,
                                }
                            },
                        ],
                    }),
                    ty,
                )
            };
            let assignment = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: Type::Null,
                    stmts: vec![Stmt::SetPath {
                        id: 0,
                        path: vec![
                            WriteStep::Index(IndexStep {
                                index: phase(0, 2, "outer", integer(2, 32, true)),
                                span: Span { start: 10, end: 20 },
                            }),
                            WriteStep::Index(IndexStep {
                                index: phase(1, 3, "inner", integer(2, 32, true)),
                                span: Span { start: 10, end: 30 },
                            }),
                        ],
                        value: phase(2, 4, "rhs", expr(ExprKind::Bool(true), Type::Bool)),
                        span: Span { start: 10, end: 30 },
                    }],
                }),
                Type::Null,
            );
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: initial.clone(),
                        },
                        Stmt::Expr(assignment),
                        Stmt::Expr(expr(
                            ExprKind::Print {
                                parts: vec![binary(
                                    "==",
                                    expr(ExprKind::Local(0), initial.ty.clone()),
                                    initial.clone(),
                                    Type::Bool,
                                )],
                                newline: true,
                            },
                            Type::Null,
                        )),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty],
            };
            assert_eq!(
                emit_ir(&program)
                    .unwrap()
                    .matches("call void @meowy_index_capture_v0")
                    .count(),
                leave
            );
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                match leave {
                    0 => b"outer\ntrue\n".as_slice(),
                    1 => b"outer\ninner\ntrue\n".as_slice(),
                    _ => b"outer\ninner\nrhs\ntrue\n".as_slice(),
                }
            );
            assert!(output.stderr.is_empty());
        }
    }
}
