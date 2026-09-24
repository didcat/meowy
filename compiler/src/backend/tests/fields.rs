use super::*;

pub(crate) fn named_record(id: usize, primary: Expr, fields: Vec<(&str, Expr, bool)>) -> Expr {
    let ty = Type::Record {
        primary: Box::new(primary.ty.clone()),
        fields: fields
            .iter()
            .map(|(name, value, mutable)| Field {
                name: (*name).into(),
                ty: value.ty.clone(),
                mutable: *mutable,
            })
            .collect(),
    };
    let mut stmts = vec![Stmt::Emit {
        id: 0,
        target: id,
        field: None,
        value: primary,
    }];
    stmts.extend(
        fields
            .into_iter()
            .enumerate()
            .map(|(index, (name, value, _))| Stmt::Emit {
                id: index + 1,
                target: id,
                field: Some(name.into()),
                value,
            }),
    );
    expr(
        ExprKind::Block(Block {
            id,
            ty: ty.clone(),
            stmts,
        }),
        ty,
    )
}

pub(crate) fn print(parts: Vec<Expr>) -> Stmt {
    Stmt::Expr(expr(
        ExprKind::Print {
            parts,
            newline: true,
        },
        Type::Null,
    ))
}

#[test]
pub(crate) fn field_mutability_keeps_layout_and_distinguishes_union_members() {
    let value = |id, mutable| {
        named_record(
            id,
            integer(7, 8, false),
            vec![
                ("flag", expr(ExprKind::Bool(true), Type::Bool), mutable),
                ("wide", integer(99, 64, true), false),
            ],
        )
    };
    let immutable = value(1, false);
    let mutable = value(2, true);
    assert_ne!(immutable.ty, mutable.ty);
    assert_eq!(ir_type(&immutable.ty), ir_type(&mutable.ty));
    assert_eq!(immutable.ty.layout(), mutable.ty.layout());
    let union = Type::union([immutable.ty.clone(), mutable.ty.clone()]);
    assert_eq!(union.members().len(), 2);
    for release in [false, true] {
        let output = native(
            separated(vec![
                binary(
                    "==",
                    coerce(immutable.clone(), &union),
                    coerce(mutable.clone(), &union),
                    Type::Bool,
                ),
                binary(
                    "==",
                    coerce(coerce(immutable.clone(), &union), &immutable.ty),
                    immutable.clone(),
                    Type::Bool,
                ),
                binary(
                    "==",
                    coerce(coerce(mutable.clone(), &union), &mutable.ty),
                    mutable.clone(),
                    Type::Bool,
                ),
            ]),
            release,
            false,
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"false|true|true|\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn field_assignments_preserve_copies_neighbors_and_last_use_reads() {
    let union = Type::union([Type::Null, Type::Bool, Type::String]);
    let cases = [
        (integer(7, 32, true), integer(9, 32, true)),
        (
            expr(ExprKind::Bool(false), Type::Bool),
            expr(ExprKind::Bool(true), Type::Bool),
        ),
        (
            record(
                10,
                integer(2, 16, false),
                expr(ExprKind::String("old".into()), Type::String),
            ),
            record(
                11,
                integer(3, 16, false),
                expr(ExprKind::String("new".into()), Type::String),
            ),
        ),
        (
            list(
                integer(0, 8, true).ty,
                4,
                vec![integer(11, 8, true), integer(22, 8, true)],
            ),
            list(integer(0, 8, true).ty, 4, vec![integer(99, 8, true)]),
        ),
        (
            coerce(expr(ExprKind::String("old".into()), Type::String), &union),
            coerce(expr(ExprKind::Bool(true), Type::Bool), &union),
        ),
    ];
    for release in [false, true] {
        for (old, new) in &cases {
            let owner = |id, field: Expr| {
                named_record(
                    id,
                    integer(7, 8, false),
                    vec![
                        ("field", field, true),
                        (
                            "stamp",
                            expr(ExprKind::String("keep".into()), Type::String),
                            false,
                        ),
                    ],
                )
            };
            let initial = owner(1, old.clone());
            let expected = owner(2, new.clone());
            let current = expr(ExprKind::Local(0), initial.ty.clone());
            let place = Place {
                root: 0,
                fields: vec![0],
            };
            let previous = expr(
                ExprKind::Deref(Box::new(expr(
                    ExprKind::Borrow(place.clone()),
                    Type::Reference(Box::new(old.ty.clone())),
                ))),
                old.ty.clone(),
            );
            let read = print(vec![binary("==", previous, old.clone(), Type::Bool)]);
            let value = if let ExprKind::Block(block) = &new.kind
                && matches!(new.ty, Type::Record { .. })
            {
                let mut block = block.clone();
                block.stmts.insert(0, read);
                expr(ExprKind::Block(block), new.ty.clone())
            } else {
                expr(
                    ExprKind::Block(Block {
                        id: 3,
                        ty: new.ty.clone(),
                        stmts: vec![
                            read,
                            Stmt::Emit {
                                id: 0,
                                target: 3,
                                field: None,
                                value: new.clone(),
                            },
                        ],
                    }),
                    new.ty.clone(),
                )
            };
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
                            id: place.root,
                            path: place.fields.into_iter().map(WriteStep::Field).collect(),
                            value,
                            span: Span { start: 10, end: 20 },
                        },
                        print(separated(vec![
                            binary("==", current.clone(), expected, Type::Bool),
                            binary(
                                "==",
                                expr(ExprKind::Local(1), initial.ty.clone()),
                                initial.clone(),
                                Type::Bool,
                            ),
                            expr(
                                ExprKind::Primary(Box::new(current.clone())),
                                integer(0, 8, false).ty,
                            ),
                            expr(
                                ExprKind::Field {
                                    value: Box::new(current),
                                    index: 1,
                                },
                                Type::String,
                            ),
                        ])),
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
                "field writes must not replace their owner aggregate"
            );
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout, b"true\ntrue|true|7|keep|\n",
                "{:?}, release={release}",
                old.ty
            );
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn nested_field_writes_preserve_layout_and_skip_stores_on_leave_or_panic() {
    let owner = |count| {
        named_record(
            12,
            expr(ExprKind::Bool(true), Type::Bool),
            vec![
                (
                    "child",
                    named_record(
                        11,
                        integer(5, 16, false),
                        vec![
                            ("count", integer(count, 64, true), true),
                            (
                                "note",
                                expr(ExprKind::String("inner".into()), Type::String),
                                false,
                            ),
                        ],
                    ),
                    true,
                ),
                (
                    "stamp",
                    expr(ExprKind::String("outer".into()), Type::String),
                    false,
                ),
            ],
        )
    };
    for release in [false, true] {
        for mode in 0..3 {
            let initial = owner(11);
            let value_ty = if mode == 0 {
                integer(0, 64, true).ty
            } else {
                Type::Never
            };
            let end = match mode {
                0 => Stmt::Emit {
                    id: 0,
                    target: 2,
                    field: None,
                    value: integer(99, 64, true),
                },
                1 => Stmt::Leave {
                    target: 1,
                    point: None,
                },
                _ => Stmt::Expr(Expr {
                    kind: ExprKind::Panic {
                        parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
                    },
                    ty: Type::Never,
                    span: Span { start: 30, end: 40 },
                }),
            };
            let value = expr(
                ExprKind::Block(Block {
                    id: 2,
                    ty: value_ty.clone(),
                    stmts: vec![
                        print(vec![expr(ExprKind::String("rhs".into()), Type::String)]),
                        end,
                    ],
                }),
                value_ty,
            );
            let ty = if mode == 2 { Type::Never } else { Type::Null };
            let assignment = expr(
                ExprKind::Block(Block {
                    id: 1,
                    ty: ty.clone(),
                    stmts: vec![Stmt::SetPath {
                        id: 0,
                        path: vec![WriteStep::Field(0), WriteStep::Field(0)],
                        value,
                        span: Span { start: 10, end: 20 },
                    }],
                }),
                ty.clone(),
            );
            let program = Program {
                body: Block {
                    id: 0,
                    ty,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: initial.clone(),
                        },
                        Stmt::Expr(assignment),
                        print(vec![binary(
                            "==",
                            expr(ExprKind::Local(0), initial.ty.clone()),
                            owner(if mode == 0 { 99 } else { 11 }),
                            Type::Bool,
                        )]),
                    ],
                },
                functions: Vec::new(),
                locals: vec![initial.ty],
            };
            let output = native_program(&program, release, false);
            if mode == 2 {
                assert_eq!(output.status.code(), Some(1));
                assert_eq!(output.stdout, b"rhs\n");
                assert_eq!(output.stderr, b"panic[P006]: stop at bytes 30..40\n");
            } else {
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, b"rhs\ntrue\n");
                assert!(output.stderr.is_empty());
            }
        }
    }
}
