use super::*;

#[test]
pub(crate) fn element_assignments_preserve_copies_neighbors_and_aggregate_layouts() {
    let a = record(
        1,
        expr(ExprKind::Bool(false), Type::Bool),
        expr(ExprKind::String("a".into()), Type::String),
    );
    let b = record(
        2,
        expr(ExprKind::Bool(false), Type::Bool),
        expr(ExprKind::String("b".into()), Type::String),
    );
    let c = record(
        3,
        expr(ExprKind::Bool(true), Type::Bool),
        expr(ExprKind::String("changed".into()), Type::String),
    );
    let union = Type::union([Type::Null, a.ty.clone()]);
    let int = integer(0, 32, true).ty;
    let cases = [
        (
            integer(11, 32, true),
            integer(22, 32, true),
            integer(99, 32, true),
        ),
        (
            expr(ExprKind::Bool(false), Type::Bool),
            expr(ExprKind::Bool(false), Type::Bool),
            expr(ExprKind::Bool(true), Type::Bool),
        ),
        (a.clone(), b, c.clone()),
        (
            coerce(a, &union),
            coerce(expr(ExprKind::Null, Type::Null), &union),
            coerce(c, &union),
        ),
        (
            list(
                int.clone(),
                4,
                vec![integer(4, 32, true), integer(5, 32, true)],
            ),
            list(
                int.clone(),
                4,
                vec![integer(6, 32, true), integer(7, 32, true)],
            ),
            list(int, 4, Vec::new()),
        ),
    ];
    for release in [false, true] {
        for (first, previous, replacement) in &cases {
            let initial = list(first.ty.clone(), 4, vec![first.clone(), previous.clone()]);
            let expected = list(
                first.ty.clone(),
                4,
                vec![first.clone(), replacement.clone()],
            );
            let current = expr(ExprKind::Local(0), initial.ty.clone());
            let copied = expr(ExprKind::Local(1), initial.ty.clone());
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
                            value: current.clone(),
                        },
                        Stmt::SetPath {
                            id: 0,
                            path: vec![WriteStep::Index(IndexStep {
                                index: integer(2, 64, false),
                                span: Span { start: 10, end: 20 },
                            })],
                            value: replacement.clone(),
                            span: Span { start: 10, end: 20 },
                        },
                        Stmt::Expr(expr(
                            ExprKind::Print {
                                parts: separated(vec![
                                    binary("==", current.clone(), expected, Type::Bool),
                                    binary("==", copied, initial.clone(), Type::Bool),
                                    size(current),
                                ]),
                                newline: true,
                            },
                            Type::Null,
                        )),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty.clone(), initial.ty.clone()],
            };
            let stores = emit_ir(&program)
                .unwrap()
                .lines()
                .filter(|line| {
                    line.trim_start()
                        .starts_with(&format!("store {} ", ir_type(&initial.ty)))
                        && line.ends_with(", ptr %local0")
                })
                .count();
            assert_eq!(
                stores, 1,
                "element assignment must not write back the aggregate"
            );
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"true|true|2|\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn element_assignment_bounds_keep_widths_and_precede_rhs_effects() {
    let print = |text: &str| {
        Stmt::Expr(expr(
            ExprKind::Print {
                parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                newline: true,
            },
            Type::Null,
        ))
    };
    let mut positions = vec![integer(0, 32, true), integer(2, 32, true)];
    for bits in [8, 16, 32, 64] {
        for signed in [false, true] {
            let value = if signed {
                -(1i128 << (bits - 1))
            } else {
                (1i128 << bits) - 1
            };
            positions.push(integer(value, bits, signed));
        }
    }
    for release in [false, true] {
        for position in &positions {
            let ExprKind::Int(number) = position.kind else {
                unreachable!()
            };
            let initial = list(Type::Bool, 4, vec![expr(ExprKind::Bool(false), Type::Bool)]);
            let index = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: position.ty.clone(),
                    stmts: vec![
                        print("index"),
                        Stmt::Emit {
                            id: 0,
                            target: 1,
                            field: None,
                            value: position.clone(),
                        },
                    ],
                }),
                position.ty.clone(),
            );
            let value = expr(
                ExprKind::Block(Block {
                    id: 2,
                    ty: Type::Bool,
                    stmts: vec![
                        print("rhs"),
                        Stmt::Emit {
                            id: 1,
                            target: 2,
                            field: None,
                            value: expr(ExprKind::Bool(true), Type::Bool),
                        },
                    ],
                }),
                Type::Bool,
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
                        Stmt::SetPath {
                            id: 0,
                            path: vec![WriteStep::Index(IndexStep {
                                index,
                                span: Span { start: 12, end: 34 },
                            })],
                            value,
                            span: Span { start: 12, end: 34 },
                        },
                        print("done"),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty],
            };
            let output = native_program(&program, release, false);
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(output.stdout, b"index\n");
            assert_eq!(
                output.stderr,
                format!(
                    "panic[P001]: index {number} is outside initialized length 1 at bytes 12..34\n"
                )
                .as_bytes()
            );
        }
    }
}

#[test]
pub(crate) fn element_assignments_keep_selected_index_and_skip_stores_after_leave() {
    let print = |text: &str| {
        Stmt::Expr(expr(
            ExprKind::Print {
                parts: vec![expr(ExprKind::String(text.into()), Type::String)],
                newline: true,
            },
            Type::Null,
        ))
    };
    let int = integer(0, 32, true).ty;
    for release in [false, true] {
        for leave in 0..3 {
            let initial = list(
                int.clone(),
                4,
                vec![integer(11, 32, true), integer(22, 32, true)],
            );
            let current = expr(ExprKind::Local(0), initial.ty.clone());
            let index_ty = if leave == 1 { Type::Never } else { int.clone() };
            let position = expr(
                ExprKind::Block(Block {
                    id: 2,
                    ty: index_ty.clone(),
                    stmts: vec![
                        print("index"),
                        if leave == 1 {
                            Stmt::Leave {
                                target: 1,
                                point: None,
                            }
                        } else {
                            Stmt::Emit {
                                id: 0,
                                target: 2,
                                field: None,
                                value: expr(ExprKind::Local(1), int.clone()),
                            }
                        },
                    ],
                }),
                index_ty,
            );
            let value_ty = if leave == 2 { Type::Never } else { int.clone() };
            let value = expr(
                ExprKind::Block(Block {
                    id: 3,
                    ty: value_ty.clone(),
                    stmts: vec![
                        print("rhs"),
                        Stmt::Assign {
                            id: 1,
                            value: integer(2, 32, true),
                        },
                        if leave == 2 {
                            Stmt::Leave {
                                target: 1,
                                point: None,
                            }
                        } else {
                            Stmt::Emit {
                                id: 1,
                                target: 3,
                                field: None,
                                value: integer(99, 32, true),
                            }
                        },
                    ],
                }),
                value_ty,
            );
            let assignment = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: Type::Null,
                    stmts: vec![Stmt::SetPath {
                        id: 0,
                        path: vec![WriteStep::Index(IndexStep {
                            index: position,
                            span: Span { start: 10, end: 20 },
                        })],
                        value,
                        span: Span { start: 10, end: 20 },
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
                        Stmt::Bind {
                            id: 1,
                            value: integer(1, 32, true),
                        },
                        Stmt::Expr(assignment),
                        Stmt::Expr(expr(
                            ExprKind::Print {
                                parts: separated(vec![
                                    index(current.clone(), integer(1, 32, true)),
                                    index(current.clone(), integer(2, 32, true)),
                                    expr(ExprKind::Local(1), int.clone()),
                                    size(current),
                                ]),
                                newline: true,
                            },
                            Type::Null,
                        )),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty, int.clone()],
            };
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                match leave {
                    0 => b"index\nrhs\n99|22|2|2|\n".as_slice(),
                    1 => b"index\n11|22|1|2|\n".as_slice(),
                    _ => b"index\nrhs\n11|22|2|2|\n".as_slice(),
                }
            );
            assert!(output.stderr.is_empty());
        }
    }
}
