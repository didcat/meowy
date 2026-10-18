use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;
use crate::hir::{Stmt, Type};

#[test]
pub(crate) fn hir_block_index_borrows_nested_and_unreached_full_target_types() {
    let source = "f:(n<int32>){r:{->inner:{->n:=1}};|false|{x:{->true}}};r:{->inner:{->n:=2}};r.{->$.inner.n}";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let blocks = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(blocks.len() >= 8);
    assert!(std::ptr::eq(blocks[&program.body.id].1, &program.body));
    assert!(std::ptr::eq(
        blocks[&program.functions[0].body.id].1,
        &program.functions[0].body
    ));
    for (&id, (owner, block)) in &blocks {
        assert_eq!(*owner, checker.bodies[&id].owner);
        assert_eq!(id, block.id);
        assert!(checker.bodies[&id].layout.matches(&block.ty));
    }
}

#[test]
pub(crate) fn hir_block_index_rejects_stale_duplicate_and_foreign_targets() {
    for fault in 0..4 {
        let (mut checker, mut program) = checked("r:{->n:=1}");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let Stmt::Bind { value, .. } = &mut program.body.stmts[0] else {
            panic!()
        };
        let ExprKind::Block(block) = &mut value.kind else {
            panic!()
        };
        match fault {
            0 => block.id = usize::MAX,
            1 => block.ty = Type::Bool,
            2 => checker.bodies.get_mut(&block.id).unwrap().owner += 1,
            3 => {
                let copy = program.body.stmts[0].clone();
                program.body.stmts.push(copy);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn hir_block_index_bounds_records_and_exact_traversal_work() {
    let (mut checker, program) = checked("f:(){r:{->n:=1}};r:{->n:=2}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let start = checker.flow.work;
    let count = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap()
        .len();
    let cost = checker.flow.work - start;
    assert!(
        checker
            .program_blocks(&program, &reports, Span::default(), count - 1)
            .is_err()
    );
    for spare in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + spare;
        assert_eq!(
            checker
                .program_blocks(&program, &reports, Span::default(), count)
                .is_ok(),
            spare == 0
        );
    }
}
