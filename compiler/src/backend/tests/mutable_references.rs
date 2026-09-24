use super::aliases::{block, local};
use super::fields::print;
use super::reference_aliases::deref;
use super::temporaries::{entry_slots, message};
use super::*;

#[test]
pub(crate) fn reference_assignment_loads_new_cells_without_retargeting_prior_values() {
    let int = integer(0, 32, true).ty;
    let pointer = Type::Reference(Box::new(int.clone()));
    let changed = block(
        1,
        &pointer,
        vec![
            Stmt::Assign {
                id: 2,
                value: borrow(1, int.clone()),
            },
            Stmt::Emit {
                id: 0,
                target: 1,
                field: None,
                value: local(2, &pointer),
            },
        ],
    );
    let rhs = block(
        2,
        &pointer,
        vec![
            message("rhs"),
            Stmt::Assign {
                id: 1,
                value: integer(33, 32, true),
            },
            Stmt::Emit {
                id: 1,
                target: 2,
                field: None,
                value: borrow(0, int.clone()),
            },
        ],
    );
    let skipped = block(
        3,
        &Type::Null,
        vec![
            Stmt::Assign {
                id: 2,
                value: block(
                    4,
                    &Type::Never,
                    vec![
                        message("skip"),
                        Stmt::Leave {
                            target: 3,
                            point: None,
                        },
                    ],
                ),
            },
            message("unreachable"),
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
                    value: integer(22, 32, true),
                },
                Stmt::Bind {
                    id: 2,
                    value: borrow(0, int.clone()),
                },
                Stmt::Bind {
                    id: 3,
                    value: local(2, &pointer),
                },
                Stmt::Bind {
                    id: 4,
                    value: deref(borrow(2, pointer.clone())),
                },
                print(separated(vec![
                    binary("==", local(2, &pointer), changed, Type::Bool),
                    deref(local(3, &pointer)),
                    deref(local(4, &pointer)),
                    deref(local(2, &pointer)),
                    binary("==", local(2, &pointer), borrow(1, int.clone()), Type::Bool),
                ])),
                Stmt::Assign { id: 2, value: rhs },
                print(separated(vec![deref(local(2, &pointer)), local(1, &int)])),
                Stmt::Expr(skipped),
                print(vec![deref(local(2, &pointer))]),
            ],
        },
        functions: Vec::new(),
        locals: vec![int.clone(), int, pointer.clone(), pointer.clone(), pointer],
    };
    let ir = emit_ir(&program).unwrap();
    entry_slots(&ir);
    assert_eq!(ir.matches("%local2 = alloca ptr").count(), 1);
    for release in [false, true] {
        let output = native_program(&program, release, false);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            output.stdout,
            b"false|11|11|22|true|\nrhs\n11|33|\nskip\n11\n"
        );
        assert!(output.stderr.is_empty());
    }
}
