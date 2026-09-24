use super::aliases::{block, field, local, shape, slot_alias};
use super::fields::{named_record, print};
use super::*;

pub(crate) fn readonly(id: usize, target: usize, value: Expr) -> Vec<Stmt> {
    slot_alias(id, target, "value", value, false)
}

#[test]
pub(crate) fn immutable_alias_borrows_keep_target_identity_after_name_scope() {
    for initial in [
        integer(42, 32, true),
        expr(ExprKind::String("stable".into()), Type::String),
    ] {
        for widened in [false, true] {
            let lexical = initial.ty.clone();
            let reference = Type::Reference(Box::new(lexical.clone()));
            let stored = if widened {
                Type::union([Type::Null, lexical.clone()])
            } else {
                lexical.clone()
            };
            let result = shape("value", stored, false);
            let value = block(
                3,
                &lexical,
                vec![
                    print(vec![expr(ExprKind::String("init".into()), Type::String)]),
                    Stmt::Emit {
                        id: 0,
                        target: 3,
                        field: None,
                        value: initial.clone(),
                    },
                ],
            );
            let mut inner = readonly(0, 1, value);
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
            let program = Program {
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
                locals: vec![lexical.clone(), reference, result, lexical.clone()],
            };
            assert!(!emit_ir(&program).unwrap().contains("store ptr %local0,"));
            for release in [false, true] {
                let output = native_program(&program, release, false);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    output.stdout,
                    if lexical == Type::String {
                        b"init\ntrue|false|\nstable\nstable\n".as_slice()
                    } else {
                        b"init\ntrue|false|\n42\n42\n".as_slice()
                    }
                );
                assert!(output.stderr.is_empty());
            }
        }
    }
}

#[test]
pub(crate) fn immutable_aggregate_aliases_borrow_exact_and_widened_payloads() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let record = named_record(
        20,
        expr(ExprKind::Bool(true), Type::Bool),
        vec![
            ("cell", integer(42, 32, true), false),
            (
                "stamp",
                expr(ExprKind::String("keep".into()), Type::String),
                false,
            ),
        ],
    );
    let values = list(
        int.clone(),
        4,
        vec![integer(7, 32, true), integer(42, 32, true)],
    );
    for (initial, indexed) in [(record, false), (values, true)] {
        for widened in [false, true] {
            let lexical = initial.ty.clone();
            let stored = if widened {
                Type::union([Type::Null, lexical.clone()])
            } else {
                lexical.clone()
            };
            let result = shape("value", stored, false);
            let mut inner = readonly(0, 1, initial.clone());
            inner.push(Stmt::Bind {
                id: 3,
                value: local(0, &lexical),
            });
            let direct = if indexed {
                element_ref(borrow(0, lexical.clone()), integer(2, 8, true))
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
                            id: 1,
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
                id: 2,
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
pub(crate) fn immutable_subunion_reads_preserve_variant_views_without_borrow_retagging() {
    let small = Type::union([Type::Bool, Type::String]);
    let stored = Type::union([Type::Null, small.clone()]);
    let result = shape("value", stored, false);
    let mut stmts = readonly(
        0,
        1,
        coerce(
            expr(ExprKind::String("stable".into()), Type::String),
            &small,
        ),
    );
    stmts.push(print(separated(vec![
        type_test(local(0, &small), Type::String),
        local(0, &Type::String),
        coerce(local(0, &small), &Type::String),
    ])));
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 1,
                    value: block(1, &result, stmts),
                },
                print(vec![coerce(field(local(1, &result), 0), &Type::String)]),
            ],
        },
        functions: Vec::new(),
        locals: vec![small.clone(), result],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"true|stable|stable|\nstable\n");
        assert!(output.stderr.is_empty());
    }
    let mut invalid = program;
    let Stmt::Bind { value, .. } = &mut invalid.body.stmts[0] else {
        unreachable!()
    };
    let ExprKind::Block(body) = &mut value.kind else {
        unreachable!()
    };
    body.stmts.push(Stmt::Expr(borrow(0, small)));
    assert!(
        emit_ir(&invalid)
            .unwrap_err()
            .contains("exact union member")
    );
}

#[test]
pub(crate) fn immutable_alias_defaults_and_restart_preserve_initializer_snapshots() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let result = shape("value", Type::union([Type::Null, int.clone()]), false);
    for present in [false, true] {
        let mut then = readonly(1, 1, integer(42, 32, true));
        then.push(print(vec![expr(
            ExprKind::Deref(Box::new(borrow(1, int.clone()))),
            int.clone(),
        )]));
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
                            &result,
                            vec![Stmt::If {
                                point: None,
                                condition: local(0, &Type::Bool),
                                then,
                                otherwise: Vec::new(),
                            }],
                        ),
                    },
                    print(vec![field(local(2, &result), 0)]),
                ],
            },
            functions: Vec::new(),
            locals: vec![Type::Bool, int.clone(), result.clone()],
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
                    b"42\n42\n".as_slice()
                } else {
                    b"null\n".as_slice()
                }
            );
            assert!(output.stderr.is_empty());
        }
    }
    let mut stmts = readonly(0, 1, local(1, &int));
    stmts.push(Stmt::Bind {
        id: 3,
        value: borrow(0, int.clone()),
    });
    let read = || {
        print(vec![expr(
            ExprKind::Deref(Box::new(local(3, &reference))),
            int.clone(),
        )])
    };
    stmts.push(read());
    stmts.push(Stmt::If {
        point: None,
        condition: local(2, &Type::Bool),
        then: vec![
            Stmt::Assign {
                id: 1,
                value: integer(42, 32, true),
            },
            read(),
            Stmt::Assign {
                id: 2,
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
                    value: integer(7, 32, true),
                },
                Stmt::Bind {
                    id: 2,
                    value: expr(ExprKind::Bool(true), Type::Bool),
                },
                Stmt::Bind {
                    id: 4,
                    value: block(1, &result, stmts),
                },
                print(vec![field(local(4, &result), 0)]),
            ],
        },
        functions: Vec::new(),
        locals: vec![int.clone(), int.clone(), Type::Bool, reference, result],
    };
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"7\n7\n42\n42\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn immutable_discarded_aliases_keep_mutability_matched_transient_storage() {
    let int = integer(0, 32, true).ty;
    let reference = Type::Reference(Box::new(int.clone()));
    let result = shape("value", int.clone(), true);
    for discard in [false, true] {
        let mut inner = readonly(0, 1, integer(42, 32, true));
        inner.push(Stmt::Emit {
            id: 1,
            target: 2,
            field: None,
            value: borrow(0, int.clone()),
        });
        let then = vec![
            Stmt::Bind {
                id: 1,
                value: block(2, &reference, inner),
            },
            print(vec![expr(
                ExprKind::Deref(Box::new(local(1, &reference))),
                int.clone(),
            )]),
            Stmt::Leave {
                target: 0,
                point: None,
            },
        ];
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind {
                        id: 3,
                        value: expr(ExprKind::Bool(discard), Type::Bool),
                    },
                    Stmt::Bind {
                        id: 2,
                        value: block(
                            1,
                            &result,
                            vec![Stmt::If {
                                point: None,
                                condition: local(3, &Type::Bool),
                                then,
                                otherwise: vec![Stmt::Emit {
                                    id: 2,
                                    target: 1,
                                    field: Some("value".into()),
                                    value: integer(7, 32, true),
                                }],
                            }],
                        ),
                    },
                    print(vec![field(local(2, &result), 0)]),
                ],
            },
            functions: Vec::new(),
            locals: vec![int.clone(), reference.clone(), result.clone(), Type::Bool],
        };
        assert!(emit_ir(&program).unwrap().contains("store ptr %local0,"));
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                output.stdout,
                if discard {
                    b"42\n".as_slice()
                } else {
                    b"7\n".as_slice()
                }
            );
            assert!(output.stderr.is_empty());
        }
    }
    let mut inner = readonly(0, 1, integer(42, 32, true));
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
