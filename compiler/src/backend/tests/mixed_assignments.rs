use super::fields::{named_record, print};
use super::*;

pub(crate) fn holder(first: Expr, second: Vec<Expr>, capacity: usize) -> Expr {
    let leaf = |value| {
        named_record(
            30,
            integer(5, 16, false),
            vec![
                ("cell", value, true),
                (
                    "stamp",
                    expr(ExprKind::String("leaf".into()), Type::String),
                    false,
                ),
            ],
        )
    };
    let ty = leaf(first.clone()).ty;
    let row = |values| {
        named_record(
            20,
            expr(ExprKind::Bool(true), Type::Bool),
            vec![
                ("items", list(ty.clone(), capacity, values), true),
                (
                    "stamp",
                    expr(ExprKind::String("row".into()), Type::String),
                    false,
                ),
            ],
        )
    };
    let first = row(if capacity == 0 {
        Vec::new()
    } else {
        vec![leaf(first)]
    });
    let second = row(second.into_iter().map(leaf).collect());
    named_record(
        10,
        integer(7, 8, false),
        vec![
            ("rows", list(first.ty.clone(), 4, vec![first, second]), true),
            (
                "stamp",
                expr(ExprKind::String("root".into()), Type::String),
                false,
            ),
        ],
    )
}

pub(crate) fn effect(id: usize, text: &str, changes: Vec<Stmt>, value: Expr, leave: bool) -> Expr {
    let ty = if leave { Type::Never } else { value.ty.clone() };
    let mut stmts = vec![print(vec![expr(
        ExprKind::String(text.into()),
        Type::String,
    )])];
    stmts.extend(changes);
    stmts.push(if leave {
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
    });
    expr(
        ExprKind::Block(Block {
            id,
            ty: ty.clone(),
            stmts,
        }),
        ty,
    )
}

pub(crate) fn path(outer: Expr, inner: Expr) -> Vec<WriteStep> {
    vec![
        WriteStep::Field(0),
        WriteStep::Index(IndexStep {
            index: outer,
            span: Span { start: 10, end: 20 },
        }),
        WriteStep::Field(0),
        WriteStep::Index(IndexStep {
            index: inner,
            span: Span { start: 10, end: 40 },
        }),
        WriteStep::Field(0),
    ]
}

#[test]
pub(crate) fn mixed_writes_preserve_layout_copies_and_captured_indices() {
    let payload = |number| {
        record(
            40,
            expr(ExprKind::Bool(true), Type::Bool),
            integer(number, 64, true),
        )
    };
    let union = Type::union([Type::Null, payload(0).ty]);
    let first = coerce(payload(11), &union);
    let second = coerce(payload(22), &union);
    let third = coerce(payload(33), &union);
    let changed = coerce(payload(99), &union);
    let initial = holder(
        first.clone(),
        vec![
            second.clone(),
            third.clone(),
            coerce(expr(ExprKind::Null, Type::Null), &union),
        ],
        4,
    );
    let expected = holder(first, vec![second, third, changed.clone()], 4);
    let int = integer(0, 32, true).ty;
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
                    value: integer(2, 32, true),
                },
                Stmt::Bind {
                    id: 2,
                    value: integer(3, 32, true),
                },
                Stmt::Bind {
                    id: 3,
                    value: expr(ExprKind::Local(0), initial.ty.clone()),
                },
                Stmt::SetPath {
                    id: 0,
                    path: path(
                        effect(
                            1,
                            "outer",
                            Vec::new(),
                            expr(ExprKind::Local(1), int.clone()),
                            false,
                        ),
                        effect(
                            2,
                            "inner",
                            vec![Stmt::Assign {
                                id: 1,
                                value: integer(1, 32, true),
                            }],
                            expr(ExprKind::Local(2), int.clone()),
                            false,
                        ),
                    ),
                    value: effect(
                        3,
                        "rhs",
                        vec![
                            Stmt::Assign {
                                id: 1,
                                value: integer(1, 32, true),
                            },
                            Stmt::Assign {
                                id: 2,
                                value: integer(1, 32, true),
                            },
                        ],
                        changed,
                        false,
                    ),
                    span: Span { start: 10, end: 50 },
                },
                print(separated(vec![
                    binary(
                        "==",
                        expr(ExprKind::Local(0), initial.ty.clone()),
                        expected,
                        Type::Bool,
                    ),
                    binary(
                        "==",
                        expr(ExprKind::Local(3), initial.ty.clone()),
                        initial.clone(),
                        Type::Bool,
                    ),
                    expr(ExprKind::Local(1), int.clone()),
                    expr(ExprKind::Local(2), int.clone()),
                ])),
            ],
        },
        functions: Vec::new(),
        locals: vec![initial.ty.clone(), int.clone(), int, initial.ty],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"outer\ninner\nrhs\ntrue|true|1|1|\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn mixed_write_bounds_report_index_prefix_and_skip_later_effects() {
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
            40,
        ),
        (
            integer(2, 32, true),
            integer(-1, 8, true),
            false,
            "-1",
            3,
            40,
        ),
        (
            integer(2, 32, true),
            integer(u64::MAX.into(), 64, false),
            false,
            "18446744073709551615",
            3,
            40,
        ),
        (
            integer(1, 32, true),
            integer(1, 16, false),
            true,
            "1",
            0,
            40,
        ),
    ];
    for release in [false, true] {
        for (outer, inner, empty, number, length, end) in &cases {
            let initial = holder(
                expr(ExprKind::Bool(false), Type::Bool),
                if *empty {
                    Vec::new()
                } else {
                    vec![expr(ExprKind::Bool(false), Type::Bool); 3]
                },
                if *empty { 0 } else { 4 },
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
                            path: path(
                                effect(1, "outer", Vec::new(), outer.clone(), false),
                                effect(2, "inner", Vec::new(), inner.clone(), false),
                            ),
                            value: effect(
                                3,
                                "rhs",
                                Vec::new(),
                                expr(ExprKind::Bool(true), Type::Bool),
                                false,
                            ),
                            span: Span { start: 10, end: 50 },
                        },
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
pub(crate) fn mixed_writes_skip_remaining_phases_after_leave() {
    for release in [false, true] {
        for leave in 0..3 {
            let initial = holder(
                expr(ExprKind::Bool(false), Type::Bool),
                vec![expr(ExprKind::Bool(false), Type::Bool); 3],
                4,
            );
            let assignment = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: Type::Null,
                    stmts: vec![Stmt::SetPath {
                        id: 0,
                        path: path(
                            effect(2, "outer", Vec::new(), integer(2, 32, true), leave == 0),
                            effect(3, "inner", Vec::new(), integer(2, 32, true), leave == 1),
                        ),
                        value: effect(
                            4,
                            "rhs",
                            Vec::new(),
                            expr(ExprKind::Bool(true), Type::Bool),
                            leave == 2,
                        ),
                        span: Span { start: 10, end: 50 },
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
                        print(vec![binary(
                            "==",
                            expr(ExprKind::Local(0), initial.ty.clone()),
                            initial.clone(),
                            Type::Bool,
                        )]),
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

#[test]
pub(crate) fn pure_field_paths_preserve_rhs_owner_replacement() {
    let owner = |id, cell, stamp: &str| {
        named_record(
            id,
            integer(7, 8, false),
            vec![
                ("cell", integer(cell, 32, true), true),
                (
                    "stamp",
                    expr(ExprKind::String(stamp.into()), Type::String),
                    true,
                ),
            ],
        )
    };
    let initial = owner(10, 1, "old");
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
                Stmt::SetPath {
                    id: 0,
                    path: vec![WriteStep::Field(0)],
                    value: effect(
                        1,
                        "rhs",
                        vec![Stmt::Assign {
                            id: 0,
                            value: owner(11, 10, "new"),
                        }],
                        integer(99, 32, true),
                        false,
                    ),
                    span: Span { start: 10, end: 20 },
                },
                print(separated(vec![
                    binary(
                        "==",
                        expr(ExprKind::Local(0), initial.ty.clone()),
                        owner(12, 99, "new"),
                        Type::Bool,
                    ),
                    binary(
                        "==",
                        expr(ExprKind::Local(1), initial.ty.clone()),
                        initial.clone(),
                        Type::Bool,
                    ),
                ])),
            ],
        },
        functions: Vec::new(),
        locals: vec![initial.ty.clone(), initial.ty],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"rhs\ntrue|true|\n");
        assert!(output.stderr.is_empty());
    }
}
