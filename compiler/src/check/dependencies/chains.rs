use super::{Cells, carriers::id, references::MAX_CELL_DEPTH, tests::statements};
use crate::{
    ast::Span,
    check::Checker,
    hir::{Expr, ExprKind, Type},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn deeper_named_and_temporary_carriers_recover_original_pointees() {
    for source in [
        "x:=false;r:&x;cell:&r;outer:&cell;copy:**outer",
        "x:=false;r:&x;cell:&r;outer:&cell;middle:*outer;copy:*middle",
        "x:=false;copy:**(&(&(&x)))",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let copy = id(&checker, "copy");
        assert_eq!(checker.pointees[&copy].roots, BTreeSet::from([0]));
        assert!(checker.pointees[&copy].complete);
        checker.mark_derived(0);
        assert!(checker.derived_local(copy));
        for cells in checker.reference_cells.keys() {
            assert!(checker.derived_local(*cells));
        }
    }
}

#[test]
pub(crate) fn deeper_retargets_preserve_snapshots_and_unknown_alternatives() {
    for (second, complete) in [("b:&s", true), ("b:{->&s}", false)] {
        let source = format!(
            "x:=false;y:=true;r:&x;s:&y;a:&r;{second};outer:=&a;old:outer;outer=&b;copy:**outer;prior:**old"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "copy")];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            if complete {
                BTreeSet::from([0, 1])
            } else {
                BTreeSet::from([0])
            }
        );
        assert_eq!(
            checker.pointees[&id(&checker, "prior")].roots,
            BTreeSet::from([0])
        );
    }
}

#[test]
pub(crate) fn cell_expansion_bounds_depth_and_dependency_cycles() {
    let mut checker = Checker::new();
    for id in 0..=MAX_CELL_DEPTH {
        checker.reference_cells.insert(
            id,
            Cells {
                places: BTreeSet::from([(id + 1, vec![])]),
                complete: true,
            },
        );
    }
    let mut value = Expr {
        kind: ExprKind::Local(0),
        ty: Type::Bool,
        span: Span::new(1, 2),
    };
    for _ in 0..MAX_CELL_DEPTH {
        value = Expr {
            kind: ExprKind::Deref(Box::new(value)),
            ty: Type::Bool,
            span: Span::new(1, 2),
        };
    }
    assert_eq!(
        checker.reference_cell(&value).unwrap().places,
        BTreeSet::from([(MAX_CELL_DEPTH + 1, vec![])])
    );
    value = Expr {
        kind: ExprKind::Deref(Box::new(value)),
        ty: Type::Bool,
        span: Span::new(1, 2),
    };
    assert_eq!(checker.reference_cell(&value).err().unwrap().code, "B001");
    checker.reference_cells.insert(
        MAX_CELL_DEPTH + 1,
        Cells {
            places: BTreeSet::from([(0, vec![])]),
            complete: true,
        },
    );
    assert!(!checker.derived_local(0));
    checker.mark_derived(MAX_CELL_DEPTH);
    assert!(checker.derived_local(0));
}

#[test]
pub(crate) fn cell_layer_fanout_preserves_capacity_boundaries() {
    use super::references::MAX_ROOTS;
    let mut checker = Checker::new();
    checker.reference_cells.insert(
        0,
        Cells {
            places: BTreeSet::from([(1, vec![]), (2, vec![])]),
            complete: true,
        },
    );
    checker.reference_cells.insert(
        1,
        Cells {
            places: (10..10 + MAX_ROOTS).map(|id| (id, vec![])).collect(),
            complete: true,
        },
    );
    checker.reference_cells.insert(
        2,
        Cells {
            places: BTreeSet::from([(10 + MAX_ROOTS, vec![])]),
            complete: true,
        },
    );
    let inner = Expr {
        kind: ExprKind::Local(0),
        ty: Type::Bool,
        span: Span::new(1, 2),
    };
    let value = Expr {
        kind: ExprKind::Deref(Box::new(inner)),
        ty: Type::Bool,
        span: Span::new(1, 3),
    };
    let error = checker.reference_cell(&value).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("cell capacity"));
    assert_eq!(error.span, value.span);
    assert_eq!(checker.reference_cells[&0].places.len(), 2);
}

#[test]
pub(crate) fn deeper_carriers_do_not_hide_storage_mutation_or_expiry_errors() {
    for (source, code) in [
        (
            "x:=false;y:=true;r:=&x;cell:&r;outer:&cell;r=&y;copy:**outer",
            "E302",
        ),
        ("outer:&(&(&false));copy:**outer", "E303"),
    ] {
        assert_eq!(crate::compile(source).unwrap_err()[0].code, code);
    }
}
