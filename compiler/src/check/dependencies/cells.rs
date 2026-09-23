use super::{carriers::id, tests::statements};
use crate::check::Checker;
use std::collections::BTreeSet;

#[test]
pub(crate) fn direct_and_named_reference_cells_keep_external_pointees() {
    for source in [
        "x:=false;r:&x;copy:*(&r)",
        "x<boolean[1]>:=[false];view:&x;cell:&view;copy:*cell",
        "x:=false;r:&x;cell:&r;copy:*cell",
        "x:=false;r:&x;cell:&r;alias:cell;copy:*alias",
        "x:=false;r:&x;cell:&r;alias:&*cell;copy:*alias",
        "x:=false;row:{->r:&x};cell:&(row.r);copy:*cell",
        "x:=false;row:{->inner:{->r:&x}};cell:&(row.inner.r);copy:*cell",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let owner = id(&checker, "x");
        let copy = id(&checker, "copy");
        assert_eq!(
            checker.pointees[&copy].roots,
            BTreeSet::from([owner]),
            "{source}"
        );
        assert!(checker.pointees[&copy].complete);
        checker.mark_derived(owner);
        assert!(checker.derived_local(copy));
    }
}

#[test]
pub(crate) fn reference_cell_reads_preserve_earlier_value_snapshots() {
    let source = "x:=false;y:=true;r:=&x;old:*(&r);r=&y;cell:&r;copy:*cell";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert_eq!(
        checker.pointees[&id(&checker, "old")].roots,
        BTreeSet::from([0])
    );
    assert_eq!(
        checker.pointees[&id(&checker, "copy")].roots,
        BTreeSet::from([0, 1])
    );
    checker.mark_derived(1);
    assert!(checker.derived_local(id(&checker, "cell")));
    assert!(!checker.derived_local(id(&checker, "old")));
}

#[test]
pub(crate) fn unknown_cell_aliases_and_contents_stay_incomplete() {
    for source in [
        "x:=false;r:&x;cell:{->&r};copy:*cell",
        "f<&boolean>:(v<&boolean>){->v};x:=false;r:f({->&x});cell:&r;copy:*cell",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.pointees[&id(&checker, "copy")].complete);
    }
    let errors = crate::compile("x:=false;y:=true;r:=&x;cell:&r;r=&y;copy:*cell").unwrap_err();
    assert_eq!(errors[0].code, "E302");
}

#[test]
pub(crate) fn cell_location_sets_preserve_snapshots_completeness_and_limits() {
    use crate::ast::Span;
    use crate::check::dependencies::{Cells, references::MAX_ROOTS};
    use crate::hir::{Expr, ExprKind, Type};

    let mut checker = Checker::new();
    let expr = Expr {
        kind: ExprKind::Local(0),
        ty: Type::Reference(Box::new(Type::Reference(Box::new(Type::Bool)))),
        span: Span::new(1, 2),
    };
    checker.reference_cells.insert(
        0,
        Cells {
            places: (0..MAX_ROOTS).map(|id| (id, vec![])).collect(),
            complete: true,
        },
    );
    checker.track_reference_cell(1, &expr, false).unwrap();
    checker.reference_cells.insert(
        0,
        Cells {
            places: BTreeSet::from([(MAX_ROOTS, vec![0])]),
            complete: false,
        },
    );
    let error = checker.track_reference_cell(1, &expr, true).unwrap_err();
    assert_eq!(error.code, "B001");
    assert_eq!(checker.reference_cells[&1].places.len(), MAX_ROOTS);
    assert!(checker.reference_cells[&1].complete);
    checker.track_reference_cell(2, &expr, false).unwrap();
    assert!(!checker.reference_cells[&2].complete);
    assert_eq!(
        checker.reference_cells[&2].places,
        BTreeSet::from([(MAX_ROOTS, vec![0])])
    );
}

#[test]
pub(crate) fn origin_traversal_reports_budget_and_pointee_capacity_failures() {
    use crate::check::dependencies::{Cells, Origins, references::MAX_ROOTS};
    use crate::{
        ast::Span,
        hir::{Expr, ExprKind, Type},
    };
    let value = Expr {
        kind: ExprKind::Local(2),
        ty: Type::Reference(Box::new(Type::Reference(Box::new(Type::Bool)))),
        span: Span::new(1, 2),
    };
    let mut checker = Checker::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK;
    assert_eq!(checker.reference_cell(&value).err().unwrap().code, "B001");
    let mut checker = Checker::new();
    checker.reference_cells.insert(
        2,
        Cells {
            places: BTreeSet::from([(0, vec![]), (1, vec![])]),
            complete: true,
        },
    );
    checker.pointees.insert(
        0,
        Origins {
            roots: (10..10 + MAX_ROOTS).collect(),
            complete: true,
        },
    );
    checker.pointees.insert(
        1,
        Origins {
            roots: BTreeSet::from([10 + MAX_ROOTS]),
            complete: true,
        },
    );
    let expr = Expr {
        kind: ExprKind::Deref(Box::new(value)),
        ty: Type::Reference(Box::new(Type::Bool)),
        span: Span::new(1, 4),
    };
    let error = checker.reference_origins(&expr).err().unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("origin capacity"));
    assert_eq!(error.span, expr.span);
}
