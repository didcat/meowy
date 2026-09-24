use super::aliases::{alias, block, field, local, shape};
use super::fields::{named_record, print};
use super::*;

pub(crate) fn scalar_scope(initial: Expr, changed: Expr, stored: Type) -> Program {
    let lexical = initial.ty.clone();
    let reference = Type::Reference(Box::new(lexical.clone()));
    let result = shape("value", stored, true);
    let mut inner = alias(0, 1, "value", initial);
    inner.push(Stmt::Assign {
        id: 0,
        value: changed,
    });
    inner.push(Stmt::Bind {
        id: 3,
        value: local(0, &lexical),
    });
    inner.push(print(separated(vec![
        binary(
            "==",
            borrow(0, lexical.clone()),
            borrow(0, lexical.clone()),
            Type::Bool,
        ),
        binary(
            "==",
            borrow(0, lexical.clone()),
            borrow(3, lexical.clone()),
            Type::Bool,
        ),
    ])));
    inner.push(Stmt::Emit {
        id: 1,
        target: 2,
        field: None,
        value: borrow(0, lexical.clone()),
    });
    let outer = block(
        1,
        &result,
        vec![
            Stmt::Bind {
                id: 1,
                value: block(2, &reference, inner),
            },
            print(vec![expr(
                ExprKind::Deref(Box::new(local(1, &reference))),
                lexical.clone(),
            )]),
        ],
    );
    Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 2,
                    value: outer,
                },
                print(vec![field(local(2, &result), 0)]),
            ],
        },
        functions: Vec::new(),
        locals: vec![lexical.clone(), reference, result, lexical],
    }
}

#[test]
pub(crate) fn emitted_scalar_borrows_use_updated_cells_until_the_target_completes() {
    let int = integer(0, 32, true).ty;
    for stored in [int.clone(), Type::union([Type::Null, int])] {
        let program = scalar_scope(integer(7, 32, true), integer(42, 32, true), stored);
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"true|false|\n42\n42\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn emitted_record_and_list_borrows_preserve_original_projected_addresses() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let record = named_record(
        20,
        expr(ExprKind::Bool(true), Type::Bool),
        vec![
            ("cell", integer(7, 32, true), true),
            (
                "stamp",
                expr(ExprKind::String("keep".into()), Type::String),
                false,
            ),
        ],
    );
    let values = list(
        int.clone(),
        3,
        vec![integer(7, 32, true), integer(8, 32, true)],
    );
    for (initial, indexed) in [(record, false), (values, true)] {
        for widened in [false, true] {
            let lexical = initial.ty.clone();
            let stored = if widened {
                Type::union([Type::Null, lexical.clone()])
            } else {
                lexical.clone()
            };
            let result = shape("value", stored, true);
            let mut inner = alias(0, 1, "value", initial.clone());
            let path = if indexed {
                vec![WriteStep::Index(IndexStep {
                    index: integer(2, 8, true),
                    span: Span { start: 10, end: 20 },
                })]
            } else {
                vec![WriteStep::Field(0)]
            };
            inner.push(Stmt::SetPath {
                id: 0,
                path,
                value: integer(42, 32, true),
                span: Span { start: 10, end: 20 },
            });
            inner.push(Stmt::Bind {
                id: 3,
                value: local(0, &lexical),
            });
            let direct = if indexed {
                element_ref(borrow(0, lexical.clone()), integer(2, 32, true))
            } else {
                expr(
                    ExprKind::Borrow(Place {
                        root: 0,
                        fields: vec![0],
                    }),
                    reference.clone(),
                )
            };
            let repeated = if indexed {
                direct.clone()
            } else {
                expr(
                    ExprKind::Reborrow {
                        site: 0,
                        value: Box::new(borrow(0, lexical.clone())),
                        fields: vec![0],
                    },
                    reference.clone(),
                )
            };
            inner.push(print(separated(vec![
                binary("==", direct.clone(), repeated, Type::Bool),
                binary(
                    "==",
                    borrow(0, lexical.clone()),
                    borrow(3, lexical.clone()),
                    Type::Bool,
                ),
            ])));
            let selected = if indexed {
                let position = block(
                    4,
                    &int,
                    vec![
                        print(vec![expr(ExprKind::String("index".into()), Type::String)]),
                        Stmt::Emit {
                            id: 2,
                            target: 4,
                            field: None,
                            value: integer(2, 32, true),
                        },
                    ],
                );
                element_ref(borrow(0, lexical.clone()), position)
            } else {
                direct
            };
            inner.push(Stmt::Emit {
                id: 1,
                target: 2,
                field: None,
                value: selected,
            });
            let outer = block(
                1,
                &result,
                vec![
                    Stmt::Bind {
                        id: 1,
                        value: block(2, &reference, inner),
                    },
                    print(vec![expr(
                        ExprKind::Deref(Box::new(local(1, &reference))),
                        int.clone(),
                    )]),
                ],
            );
            let final_value = coerce(field(local(2, &result), 0), &lexical);
            let final_value = if indexed {
                index(final_value, integer(2, 32, true))
            } else {
                field(final_value, 0)
            };
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 2,
                            value: outer,
                        },
                        print(vec![final_value]),
                    ],
                },
                functions: Vec::new(),
                locals: vec![lexical.clone(), reference.clone(), result, lexical],
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
                    if indexed {
                        b"true|false|\nindex\n42\n42\n".as_slice()
                    } else {
                        b"true|false|\n42\n42\n".as_slice()
                    }
                );
                assert!(output.stderr.is_empty());
            }
        }
    }
}

#[test]
pub(crate) fn emitted_union_borrows_require_matching_tagged_storage() {
    let int = integer(0, 32, true).ty;
    let union = Type::union([Type::Null, int, Type::String]);
    let program = scalar_scope(
        coerce(expr(ExprKind::String("old".into()), Type::String), &union),
        coerce(integer(42, 32, true), &union),
        union,
    );
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"true|false|\n42\n42\n");
        assert!(output.stderr.is_empty());
    }
    let small = Type::union([Type::Bool, Type::String]);
    let stored = Type::union([Type::Null, small.clone()]);
    let program = scalar_scope(
        coerce(expr(ExprKind::String("old".into()), Type::String), &small),
        coerce(expr(ExprKind::Bool(true), Type::Bool), &small),
        stored,
    );
    assert!(
        emit_ir(&program)
            .unwrap_err()
            .contains("exact union member")
    );
}

#[test]
pub(crate) fn discarded_emitted_borrows_keep_target_owned_transient_storage() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let mut inner = alias(0, 1, "value", integer(7, 32, true));
    inner.push(Stmt::Assign {
        id: 0,
        value: integer(42, 32, true),
    });
    inner.push(Stmt::Emit {
        id: 1,
        target: 2,
        field: None,
        value: borrow(0, int.clone()),
    });
    let outer = block(
        1,
        &Type::Never,
        vec![
            Stmt::Bind {
                id: 1,
                value: block(2, &reference, inner),
            },
            print(vec![expr(
                ExprKind::Deref(Box::new(local(1, &reference))),
                int.clone(),
            )]),
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
            stmts: vec![Stmt::Expr(outer)],
        },
        functions: Vec::new(),
        locals: vec![int, reference],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"42\n");
        assert_eq!(output.stderr, b"panic[P006]: stop at bytes 30..40\n");
    }
}

#[test]
pub(crate) fn target_restart_reinitializes_borrowable_result_and_transient_cells() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    for transient in [false, true] {
        let result = if transient {
            Type::Never
        } else {
            shape("value", Type::union([Type::Null, int.clone()]), true)
        };
        let mut stmts = alias(0, 1, "value", integer(5, 32, true));
        stmts.push(Stmt::Assign {
            id: 0,
            value: binary("+", local(0, &int), integer(1, 32, true), int.clone()),
        });
        stmts.push(Stmt::Bind {
            id: 2,
            value: borrow(0, int.clone()),
        });
        stmts.push(print(vec![expr(
            ExprKind::Deref(Box::new(local(2, &reference))),
            int.clone(),
        )]));
        stmts.push(Stmt::If {
            point: None,
            condition: local(1, &Type::Bool),
            then: vec![
                Stmt::Assign {
                    id: 1,
                    value: expr(ExprKind::Bool(false), Type::Bool),
                },
                Stmt::Restart { target: 1, site: 0 },
            ],
            otherwise: Vec::new(),
        });
        if transient {
            stmts.push(Stmt::Expr(Expr {
                kind: ExprKind::Panic {
                    parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
                },
                ty: Type::Never,
                span: Span { start: 30, end: 40 },
            }));
        }
        let value = block(1, &result, stmts);
        let mut body = vec![Stmt::Bind {
            id: 1,
            value: expr(ExprKind::Bool(true), Type::Bool),
        }];
        if transient {
            body.push(Stmt::Expr(value));
        } else {
            body.push(Stmt::Bind { id: 3, value });
            body.push(print(vec![field(local(3, &result), 0)]));
        }
        let program = Program {
            body: Block {
                id: 0,
                ty: if transient { Type::Never } else { Type::Null },
                stmts: body,
            },
            functions: Vec::new(),
            locals: vec![int.clone(), Type::Bool, reference.clone(), result],
        };
        for release in [false, true] {
            let output = native_program(&program, release, false);
            if transient {
                assert_eq!(output.status.code(), Some(1));
                assert_eq!(output.stdout, b"6\n6\n");
                assert_eq!(output.stderr, b"panic[P006]: stop at bytes 30..40\n");
            } else {
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(output.stdout, b"6\n6\n6\n");
                assert!(output.stderr.is_empty());
            }
        }
    }
}
