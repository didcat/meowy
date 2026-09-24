use super::aliases::{block, local};
use super::fields::print;
use super::reference_aliases::deref;
use super::*;

pub(crate) fn temporary(id: usize, value: Expr) -> Expr {
    let ty = if value.ty == Type::Never {
        Type::Never
    } else {
        Type::Reference(Box::new(value.ty.clone()))
    };
    expr(
        ExprKind::TemporaryBorrow {
            id,
            statement: 0,
            value: Box::new(value),
        },
        ty,
    )
}

pub(crate) fn message(text: &str) -> Stmt {
    print(vec![expr(ExprKind::String(text.into()), Type::String)])
}

pub(crate) fn effect(id: usize, text: &str, mut value: Expr) -> Expr {
    if matches!(value.ty, Type::Record { .. }) {
        let ExprKind::Block(block) = &mut value.kind else {
            panic!("record effect requires a constructor block")
        };
        block.stmts.insert(0, message(text));
        return value;
    }
    block(
        id,
        &value.ty.clone(),
        vec![
            message(text),
            Stmt::Emit {
                id: 0,
                target: id,
                field: None,
                value,
            },
        ],
    )
}

pub(crate) fn entry_slots(ir: &str) {
    for body in ir.split("\nentry:\n").skip(1) {
        let entry = body.split("  br label ").next().unwrap();
        assert_eq!(
            entry.matches(" = alloca ").count(),
            body.matches(" = alloca ").count()
        );
    }
}

#[test]
pub(crate) fn temporary_calls_keep_distinct_cells_and_argument_order() {
    let int = integer(7, 32, true);
    let union = Type::union([int.ty.clone(), Type::String]);
    for value in [
        int.clone(),
        expr(ExprKind::Null, Type::Null),
        expr(ExprKind::Bool(true), Type::Bool),
        expr(ExprKind::String("same".into()), Type::String),
        list(int.ty.clone(), 2, vec![int.clone()]),
        record(4, integer(1, 8, false), int.clone()),
        coerce(int, &union),
    ] {
        let owner = value.ty.clone();
        let reference = Type::Reference(Box::new(owner.clone()));
        let call = expr(
            ExprKind::Call {
                id: 0,
                site: 0,
                args: vec![
                    temporary(0, effect(1, "left", value.clone())),
                    temporary(1, effect(2, "right", value)),
                ],
            },
            Type::Bool,
        );
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![Stmt::Statement {
                    id: 0,
                    stmts: vec![print(vec![call])],
                }],
            },
            functions: vec![crate::hir::Function {
                id: 0,
                name: "same".into(),
                params: vec![2, 3],
                result: Type::Bool,
                body: Block {
                    id: 3,
                    ty: Type::Bool,
                    stmts: vec![Stmt::Emit {
                        id: 1,
                        target: 3,
                        field: None,
                        value: binary("==", local(2, &reference), local(3, &reference), Type::Bool),
                    }],
                },
            }],
            locals: vec![owner.clone(), owner, reference.clone(), reference],
        };
        let ir = emit_ir(&program).unwrap();
        entry_slots(&ir);
        assert_eq!(ir.matches("%local0 = alloca").count(), 1);
        assert_eq!(ir.matches("%local1 = alloca").count(), 1);
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert!(output.status.success());
            assert_eq!(output.stdout, b"left\nright\nfalse\n");
            assert!(output.stderr.is_empty());
        }
    }
}

#[test]
pub(crate) fn temporary_projections_keep_nested_layout_and_materialization_identity() {
    let int = integer(91, 32, true);
    let union = Type::union([int.ty.clone(), Type::String]);
    let items = list(
        union.clone(),
        3,
        vec![
            coerce(expr(ExprKind::String("first".into()), Type::String), &union),
            coerce(int.clone(), &union),
        ],
    );
    let row = record(4, integer(5, 8, false), items.clone());
    let rows = list(row.ty.clone(), 2, vec![row.clone()]);
    let owner = record(3, expr(ExprKind::Bool(true), Type::Bool), rows.clone());
    let project = |value: Expr, ty: &Type| {
        expr(
            ExprKind::Reborrow {
                site: 0,
                value: Box::new(value),
                fields: vec![0],
            },
            Type::Reference(Box::new(ty.clone())),
        )
    };
    let selected = |value, effects| {
        let row_index = if effects {
            effect(5, "row", integer(1, 8, false))
        } else {
            integer(1, 8, false)
        };
        let item_index = if effects {
            effect(6, "column", integer(2, 64, false))
        } else {
            integer(2, 64, false)
        };
        let row = element_ref(project(value, &rows.ty), row_index);
        element_ref(project(row, &items.ty), item_index)
    };
    let address = selected(temporary(0, effect(2, "owner", owner.clone())), true);
    let expected = selected(borrow(0, owner.ty.clone()), false);
    let copied = temporary(1, deref(expected.clone()));
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![Stmt::Statement {
                id: 0,
                stmts: vec![print(separated(vec![
                    binary("==", address, expected.clone(), Type::Bool),
                    coerce(deref(copied), &int.ty),
                    binary("==", borrow(1, union.clone()), expected, Type::Bool),
                ]))],
            }],
        },
        functions: Vec::new(),
        locals: vec![owner.ty, union],
    };
    entry_slots(&emit_ir(&program).unwrap());
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"owner\nrow\ncolumn\ntrue|91|false|\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
pub(crate) fn temporary_bounds_and_exits_skip_later_evaluation() {
    let values = list(Type::Bool, 0, Vec::new());
    for mode in 0..4 {
        let panic = || {
            let mut value = expr(
                ExprKind::Panic {
                    parts: vec![expr(ExprKind::String("stop".into()), Type::String)],
                },
                Type::Never,
            );
            value.span = Span { start: 30, end: 40 };
            value
        };
        let owner = match mode {
            0 => block(1, &Type::Never, vec![message("owner"), Stmt::Expr(panic())]),
            3 => block(
                1,
                &Type::Never,
                vec![
                    message("owner"),
                    Stmt::Leave {
                        target: 0,
                        point: None,
                    },
                ],
            ),
            _ => effect(1, "owner", values.clone()),
        };
        let owner = temporary(0, owner);
        let index = if mode == 1 {
            block(2, &Type::Never, vec![message("index"), Stmt::Expr(panic())])
        } else {
            effect(2, "index", integer(u64::MAX.into(), 64, false))
        };
        let mut access = expr(
            ExprKind::ElementBorrow {
                site: 0,
                value: Box::new(owner),
                index: Box::new(index),
            },
            if mode == 2 {
                Type::Reference(Box::new(Type::Bool))
            } else {
                Type::Never
            },
        );
        access.span = Span { start: 12, end: 34 };
        let program = Program {
            body: Block {
                id: 0,
                ty: Type::Null,
                stmts: vec![Stmt::Statement {
                    id: 0,
                    stmts: vec![Stmt::Expr(access), message("after")],
                }],
            },
            functions: Vec::new(),
            locals: vec![values.ty.clone()],
        };
        let ir = emit_ir(&program).unwrap();
        entry_slots(&ir);
        if mode != 2 {
            assert!(!ir.contains("call void @meowy_index_capture_v0"));
        }
        if mode == 0 || mode == 3 {
            assert!(!ir.contains("%local0 = alloca"));
        }
        for release in [false, true] {
            let output = native_program(&program, release, false);
            assert_eq!(output.status.code(), Some(if mode == 3 { 0 } else { 1 }));
            assert_eq!(
                output.stdout,
                if mode == 0 || mode == 3 {
                    b"owner\n".as_slice()
                } else {
                    b"owner\nindex\n".as_slice()
                }
            );
            assert_eq!(output.stderr, match mode {
                0 | 1 => b"panic[P006]: stop at bytes 30..40\n".as_slice(),
                2 => b"panic[P001]: index 18446744073709551615 is outside initialized length 0 at bytes 12..34\n".as_slice(),
                _ => b"".as_slice(),
            });
        }
    }
}

#[test]
pub(crate) fn restart_reinitializes_temporary_values_in_entry_allocated_cells() {
    let int = integer(0, 32, true).ty;
    let body = block(
        1,
        &Type::Null,
        vec![Stmt::Statement {
            id: 1,
            stmts: vec![
                print(vec![deref(temporary(0, effect(2, "init", local(1, &int))))]),
                Stmt::Assign {
                    id: 1,
                    value: binary("+", local(1, &int), integer(1, 32, true), int.clone()),
                },
                Stmt::If {
                    point: None,
                    condition: binary("<", local(1, &int), integer(3, 32, true), Type::Bool),
                    then: vec![Stmt::Restart { target: 1, site: 0 }],
                    otherwise: vec![Stmt::Leave {
                        target: 1,
                        point: None,
                    }],
                },
                message("unreachable"),
            ],
        }],
    );
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![
                Stmt::Bind {
                    id: 1,
                    value: integer(0, 32, true),
                },
                Stmt::Expr(body),
                message("done"),
            ],
        },
        functions: Vec::new(),
        locals: vec![int.clone(), int],
    };
    let ir = emit_ir(&program).unwrap();
    entry_slots(&ir);
    assert_eq!(ir.matches("%local0 = alloca").count(), 1);
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"init\n0\ninit\n1\ninit\n2\ndone\n");
        assert!(output.stderr.is_empty());
    }
}
