use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;
use crate::flow::MAX_PROOF_WORK;
use crate::hir::Stmt;

#[test]
pub(crate) fn hir_call_index_keeps_wrappers_nested_calls_and_unreached_arguments() {
    for source in [
        "f<int32>:(n<int32>){->n};g:(){f(1)};out<int32><null>:((f((f(2)))))",
        "d:@\"debug\";f:(a<int32>,b<int32>){};g<int32>:(){->1};f(d.panic(\"stop\"),g())",
        "f<int32>:(n<int32>){->n};r:{->n:f(1)};x:r.{->f($.n)};|false|f(2)",
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let calls = checker
            .program_calls(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(calls.len(), checker.invocations.len());
        for (site, (owner, body, expr)) in calls {
            assert_eq!(checker.invocations[&site].owner, owner);
            assert_eq!(reports.entries[&owner].0, body);
            assert!(matches!(expr.kind, ExprKind::Call { site: found, .. } if found == site));
        }
    }
}

#[test]
pub(crate) fn hir_call_index_rejects_duplicate_foreign_and_mismatched_calls() {
    for fault in 0..5 {
        let (mut checker, mut program) = checked("f<int32>:(n<int32>){->n};f(1)");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let Stmt::Expr(expr) = &mut program.body.stmts[0] else {
            panic!()
        };
        let ExprKind::Call { id, site, args } = &mut expr.kind else {
            panic!()
        };
        match fault {
            0 => *site = usize::MAX,
            1 => *id = usize::MAX,
            2 => args.clear(),
            3 => expr.ty = hir::Type::Bool,
            4 => {
                let copy = program.body.stmts[0].clone();
                program.body.stmts.push(copy);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .program_calls(&program, &reports, Span::default(), MAX_EDGES)
                .is_err()
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn hir_call_index_bounds_rows_and_exact_work() {
    let (mut checker, program) = checked("f:(){};f();f()");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert!(
        checker
            .program_calls(&program, &reports, Span::default(), 1)
            .is_err()
    );
    let before = checker.flow.work;
    assert_eq!(
        checker
            .program_calls(&program, &reports, Span::default(), 2)
            .unwrap()
            .len(),
        2
    );
    let work = checker.flow.work - before;
    for extra in [0, 1] {
        checker.flow.work = MAX_PROOF_WORK - work + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .program_calls(&program, &reports, Span::default(), 2)
                .is_ok(),
            extra == 0
        );
    }
}
