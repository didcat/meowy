use super::aliases::{block, field, local};
use super::fields::print;
use super::reference_aliases::deref;
use super::temporaries::{effect, entry_slots, message, temporary};
use super::*;

#[test]
pub(crate) fn temporary_reference_cells_keep_distinct_addresses_and_copied_pointers() {
    let int = integer(77, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: integer(77, 32, true),
                },
                Stmt::Statement {
                    id: 0,
                    stmts: vec![
                        print(separated(vec![
                            binary(
                                "==",
                                temporary(1, effect(1, "left", borrow(0, int.clone()))),
                                temporary(2, effect(2, "right", borrow(0, int.clone()))),
                                Type::Bool,
                            ),
                            binary(
                                "==",
                                deref(borrow(1, pointer.clone())),
                                deref(borrow(2, pointer.clone())),
                                Type::Bool,
                            ),
                        ])),
                        Stmt::Bind {
                            id: 3,
                            value: deref(borrow(1, pointer.clone())),
                        },
                        Stmt::Bind {
                            id: 4,
                            value: deref(temporary(
                                5,
                                deref(temporary(6, effect(3, "nested", borrow(0, int.clone())))),
                            )),
                        },
                    ],
                },
                print(separated(vec![
                    binary("==", local(3, &pointer), borrow(0, int.clone()), Type::Bool),
                    deref(local(3, &pointer)),
                    binary("==", local(4, &pointer), borrow(0, int.clone()), Type::Bool),
                    deref(local(4, &pointer)),
                ])),
            ],
        },
        functions: Vec::new(),
        locals: vec![
            int,
            pointer.clone(),
            pointer.clone(),
            pointer.clone(),
            pointer.clone(),
            pointer.clone(),
            pointer,
        ],
    };
    let ir = emit_ir(&program).unwrap();
    entry_slots(&ir);
    assert!(ir.contains("%local1 = alloca ptr"));
    assert!(ir.contains("%local2 = alloca ptr"));
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"left\nright\nfalse|true|\nnested\ntrue|77|true|77|\n"
        );
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn temporary_carrier_and_union_copies_preserve_nested_reference_contents() {
    let int = integer(13, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let optional = Type::union([Type::Null, pointer.clone()]);
    for present in [false, true] {
        for widened in [false, true] {
            let view = if present {
                borrow(0, int.clone())
            } else {
                expr(ExprKind::Null, Type::Null)
            };
            let inner = record(
                4,
                expr(ExprKind::Bool(true), Type::Bool),
                coerce(view, &optional),
            );
            let carrier = record(3, integer(9, 8, false), inner);
            let stored = if widened {
                Type::union([Type::Null, carrier.ty.clone()])
            } else {
                carrier.ty.clone()
            };
            let value = deref(temporary(1, effect(2, "carrier", carrier.clone())));
            let value = if widened {
                deref(temporary(3, coerce(value, &stored)))
            } else {
                value
            };
            let selected = expr(
                ExprKind::Reborrow {
                    site: 0,
                    value: Box::new(borrow(1, carrier.ty.clone())),
                    fields: vec![0, 0],
                },
                Type::Reference(Box::new(optional.clone())),
            );
            let returned = coerce(local(2, &stored), &carrier.ty);
            let returned = field(field(returned, 0), 0);
            let program = Program {
                body: Block {
                    id: 0,
                    ty: Type::Null,
                    stmts: vec![
                        Stmt::Bind {
                            id: 0,
                            value: integer(13, 32, true),
                        },
                        Stmt::Statement {
                            id: 0,
                            stmts: vec![
                                Stmt::Bind { id: 2, value },
                                print(vec![type_test(deref(selected), pointer.clone())]),
                            ],
                        },
                        print(vec![type_test(returned.clone(), pointer.clone())]),
                        Stmt::If {
                            point: None,
                            condition: type_test(returned.clone(), pointer.clone()),
                            then: vec![print(separated(vec![
                                binary(
                                    "==",
                                    coerce(returned.clone(), &pointer),
                                    borrow(0, int.clone()),
                                    Type::Bool,
                                ),
                                deref(coerce(returned, &pointer)),
                            ]))],
                            otherwise: vec![message("null")],
                        },
                    ],
                },
                functions: Vec::new(),
                locals: vec![int.clone(), carrier.ty, stored.clone(), stored],
            };
            entry_slots(&emit_ir(&program).unwrap());
            for release in [false, true] {
                let output = native_program(&program, release, false);
                assert!(output.status.success(), "{output:?}");
                assert_eq!(
                    output.stdout,
                    if present {
                        b"carrier\ntrue\ntrue\ntrue|13|\n".as_slice()
                    } else {
                        b"carrier\nfalse\nfalse\nnull\n".as_slice()
                    }
                );
                assert!(output.stderr.is_empty());
            }
        }
    }
}

#[test]
pub(crate) fn temporary_reference_call_arguments_stop_before_nonreturning_calls() {
    let int = integer(11, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let cell = Type::Reference(Box::new(pointer.clone()));
    for mode in 0..3 {
        let second = if mode == 0 {
            effect(2, "second", integer(0, 32, true))
        } else {
            block(
                2,
                &Type::Never,
                vec![
                    message("second"),
                    if mode == 1 {
                        Stmt::Expr(Expr {
                            kind: ExprKind::Panic {
                                parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
                            },
                            ty: Type::Never,
                            span: Span { start: 30, end: 40 },
                        })
                    } else {
                        Stmt::Leave(0)
                    },
                ],
            )
        };
        let call = expr(
            ExprKind::Call {
                id: 0,
                site: 0,
                args: vec![
                    temporary(1, effect(1, "cell", borrow(0, int.clone()))),
                    second,
                ],
            },
            if mode == 0 {
                pointer.clone()
            } else {
                Type::Never
            },
        );
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![
                    Stmt::Bind {
                        id: 0,
                        value: integer(11, 32, true),
                    },
                    Stmt::Statement {
                        id: 0,
                        stmts: vec![if mode == 0 {
                            print(vec![deref(call)])
                        } else {
                            Stmt::Expr(call)
                        }],
                    },
                ],
            },
            functions: vec![crate::hir::Function {
                id: 0,
                name: "load".into(),
                params: vec![2, 3],
                result: pointer.clone(),
                body: Block {
                    id: 3,
                    ty: pointer.clone(),
                    stmts: vec![Stmt::Emit {
                        id: 0,
                        target: 3,
                        field: None,
                        value: deref(local(2, &cell)),
                    }],
                },
            }],
            locals: vec![int.clone(), pointer.clone(), cell.clone(), int.clone()],
        };
        let ir = emit_ir(&program).unwrap();
        entry_slots(&ir);
        assert_eq!(
            ir.matches("@meowy_fn_0(").count(),
            if mode == 0 { 2 } else { 1 }
        );
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert_eq!(output.status.code(), Some(if mode == 1 { 1 } else { 0 }));
            assert_eq!(
                output.stdout,
                if mode == 0 {
                    b"cell\nsecond\n11\n".as_slice()
                } else {
                    b"cell\nsecond\n".as_slice()
                }
            );
            assert_eq!(
                output.stderr,
                if mode == 1 {
                    b"panic[P006]: stop at bytes 30..40\n".as_slice()
                } else {
                    b"".as_slice()
                }
            );
        }
    }
}

#[test]
pub(crate) fn restarted_reference_temporaries_replace_active_union_payloads() {
    let int = integer(11, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let optional = Type::union([Type::Null, pointer.clone()]);
    let value = block(
        2,
        &optional,
        vec![
            message("init"),
            Stmt::If {
                point: None,
                condition: local(1, &Type::Bool),
                then: vec![Stmt::Emit {
                    id: 0,
                    target: 2,
                    field: None,
                    value: borrow(0, int.clone()),
                }],
                otherwise: Vec::new(),
            },
        ],
    );
    let repeated = block(
        1,
        &Type::Null,
        vec![
            Stmt::Statement {
                id: 0,
                stmts: vec![Stmt::Bind {
                    id: 3,
                    value: deref(temporary(2, value)),
                }],
            },
            Stmt::If {
                point: None,
                condition: type_test(local(3, &optional), pointer.clone()),
                then: vec![print(vec![deref(coerce(local(3, &optional), &pointer))])],
                otherwise: vec![message("null")],
            },
            Stmt::If {
                point: None,
                condition: local(1, &Type::Bool),
                then: vec![
                    Stmt::Assign {
                        id: 1,
                        value: expr(ExprKind::Bool(false), Type::Bool),
                    },
                    Stmt::Restart { target: 1, site: 0 },
                ],
                otherwise: vec![Stmt::Leave(1)],
            },
        ],
    );
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 0,
                    value: integer(11, 32, true),
                },
                Stmt::Bind {
                    id: 1,
                    value: expr(ExprKind::Bool(true), Type::Bool),
                },
                Stmt::Expr(repeated),
                message("done"),
            ],
        },
        functions: Vec::new(),
        locals: vec![int, Type::Bool, optional.clone(), optional],
    };
    let ir = emit_ir(&program).unwrap();
    entry_slots(&ir);
    assert_eq!(ir.matches("%local2 = alloca").count(), 1);
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"init\n11\ninit\nnull\ndone\n");
        assert!(output.stderr.is_empty());
    }
}
