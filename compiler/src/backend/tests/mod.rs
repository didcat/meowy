mod aggregates;
mod aliases;
mod assignments;
mod bridge;
mod emitted_borrows;
mod fields;
mod foundation;
mod generated_cleanup;
mod generated_owned;
mod generated_strings;
mod immutable_aliases;
mod lists;
mod mapped_checks;
mod mixed_assignments;
mod mutable_references;
mod nested_assignments;
mod panic_outcomes;
mod reference_aliases;
mod reference_temporaries;
mod references;
mod runtime_sites;
mod scalars;
mod sites;
mod temporaries;

use super::*;
use crate::ast::Span;
use crate::hir::{Field, IndexStep, Place, WriteStep};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) static NEXT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn expr(kind: ExprKind, ty: Type) -> Expr {
    Expr {
        kind,
        ty,
        span: Span::default(),
    }
}

pub(crate) fn integer(value: i128, bits: u32, signed: bool) -> Expr {
    expr(ExprKind::Int(value), Type::Int { bits, signed })
}

pub(crate) fn binary(op: &str, left: Expr, right: Expr, ty: Type) -> Expr {
    expr(
        ExprKind::Binary {
            point: None,
            op: op.into(),
            left: Box::new(left),
            right: Box::new(right),
        },
        ty,
    )
}

pub(crate) fn list(element: Type, capacity: usize, values: Vec<Expr>) -> Expr {
    let ty = Type::List {
        element: Box::new(element),
        capacity,
    };
    expr(
        ExprKind::List {
            values,
            list: ty.clone(),
        },
        ty,
    )
}

pub(crate) fn index(value: Expr, position: Expr) -> Expr {
    let Type::List { element, .. } = &value.ty else {
        unreachable!()
    };
    let ty = *element.clone();
    expr(
        ExprKind::ListIndex {
            value: Box::new(value),
            index: Box::new(position),
        },
        ty,
    )
}

pub(crate) fn borrow(root: usize, ty: Type) -> Expr {
    expr(
        ExprKind::Borrow(Place {
            root,
            fields: Vec::new(),
        }),
        Type::Reference(Box::new(ty)),
    )
}

pub(crate) fn element_ref(value: Expr, index: Expr) -> Expr {
    let Type::Reference(list) = &value.ty else {
        unreachable!()
    };
    let Type::List { element, .. } = list.as_ref() else {
        unreachable!()
    };
    let ty = if index.ty == Type::Never {
        Type::Never
    } else {
        Type::Reference(element.clone())
    };
    expr(
        ExprKind::ElementBorrow {
            site: 0,
            value: Box::new(value),
            index: Box::new(index),
        },
        ty,
    )
}

pub(crate) fn size(value: Expr) -> Expr {
    expr(
        ExprKind::ListSize(Box::new(value)),
        Type::Int {
            bits: 64,
            signed: false,
        },
    )
}

pub(crate) fn separated(parts: Vec<Expr>) -> Vec<Expr> {
    parts
        .into_iter()
        .flat_map(|part| [part, expr(ExprKind::String("|".into()), Type::String)])
        .collect()
}

pub(crate) fn native(parts: Vec<Expr>, release: bool, full: bool) -> Output {
    let program = Program {
        body: Block {
            id: 0,
            ty: Type::Null,
            stmts: vec![Stmt::Expr(expr(
                ExprKind::Print {
                    parts,
                    newline: true,
                },
                Type::Null,
            ))],
        },
        functions: Vec::new(),
        locals: Vec::new(),
    };
    native_program(&program, release, full)
}

pub(crate) fn native_program(program: &Program, release: bool, full: bool) -> Output {
    native_ir(&emit_ir(program).unwrap(), release, full)
}

pub(crate) fn native_ir(ir: &str, release: bool, full: bool) -> Output {
    let dir = std::env::temp_dir().join(format!(
        "meowy-native-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let object = dir.join("main.o");
    emit_object(ir, &object, release).unwrap_or_else(|error| panic!("{error}\n{ir}"));
    let runtime = dir.join("runtime.a");
    std::fs::write(&runtime, RUNTIME_ARCHIVE).unwrap();
    let executable = dir.join("main");
    let link = Command::new(CLANG)
        .arg(format!("-fuse-ld={LLD}"))
        .arg("-Wl,--gc-sections")
        .arg(&object)
        .arg(&runtime)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        link.status.success(),
        "{}",
        String::from_utf8_lossy(&link.stderr)
    );
    let mut command = Command::new(&executable);
    if full {
        command.stdout(Stdio::from(
            std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap(),
        ));
    }
    let output = command.output().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    output
}

pub(crate) fn coerce(value: Expr, ty: &Type) -> Expr {
    expr(
        ExprKind::Coerce {
            value: Box::new(value),
        },
        ty.clone(),
    )
}

pub(crate) fn type_test(value: Expr, ty: Type) -> Expr {
    expr(
        ExprKind::TypeTest {
            value: Box::new(value),
            ty,
        },
        Type::Bool,
    )
}

pub(crate) fn record(id: usize, primary: Expr, field: Expr) -> Expr {
    let ty = Type::Record {
        primary: Box::new(primary.ty.clone()),
        fields: vec![Field {
            name: "field".into(),
            ty: field.ty.clone(),
            mutable: false,
        }],
    };
    expr(
        ExprKind::Block(Block {
            id,
            ty: ty.clone(),
            stmts: vec![
                Stmt::Emit {
                    id: 0,
                    target: id,
                    field: None,
                    value: primary,
                },
                Stmt::Emit {
                    id: 0,
                    target: id,
                    field: Some("field".into()),
                    value: field,
                },
            ],
        }),
        ty,
    )
}
