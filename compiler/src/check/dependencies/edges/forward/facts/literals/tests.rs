use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

pub(super) fn context(source: &str) -> (Checker, Reports) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn scalar_literal_seeds_borrow_reports_and_keep_full_domains_separate() {
    let (mut checker, reports) = context(
        "f<int64>:(){->-9223372036854775808};n<uint8>:7;yes:true;none:null;text:\"hello\";real:1.5;p:@\"proof\";version<uint32>:p.revision;copy:n",
    );
    let (seeds, _) = checker
        .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(seeds.len(), 7);
    assert!(seeds.values().any(|seed| seed.source.0 != 0));
    for (point, seed) in seeds {
        assert!(std::ptr::eq(seed.source, &reports.effects[&point]));
        let (_, Effect::Scalar(observed)) = seed.source else {
            panic!()
        };
        assert_eq!(seed.value.is_some(), observed.value.is_some());
        if let Some(value) = observed.value {
            assert_eq!(
                seed.value,
                Some(Domain::literal(value, &mut checker.flow, Span::default()).unwrap())
            );
        }
        if matches!(
            observed.ty,
            crate::check::dependencies::ScalarKind::Int { .. }
        ) {
            assert_ne!(seed.value, seed.declared);
        }
    }
}

#[test]
pub(crate) fn scalar_literal_seeds_require_results_without_inventing_operations_or_bottom() {
    let (mut checker, mut reports) = context("7");
    let point = *checker.scalar_leaves.first_key_value().unwrap().0;
    for (operation, result) in [(true, false), (false, true), (true, true)] {
        let (_, Effect::Scalar(observed)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        observed.operation = operation;
        observed.result = result;
        let (seeds, _) = checker
            .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert!(seeds[&point].declared.is_some());
        assert_eq!(seeds[&point].value.is_some(), result);
    }
    reports.effects.remove(&point);
    assert!(
        checker
            .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
            .unwrap()
            .0
            .is_empty()
    );
    let (mut checker, reports) = context("p:@\"proof\";version:p.revision");
    assert!(
        checker
            .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
            .unwrap()
            .0
            .is_empty()
    );
}

#[test]
pub(crate) fn scalar_literal_seeds_reject_late_stale_reports_and_preserve_inputs() {
    for fault in 0..5 {
        let (mut checker, mut reports) = context("1;2");
        let point = *checker.scalar_leaves.last_key_value().unwrap().0;
        let (owner, Effect::Scalar(observed)) = reports.effects.get_mut(&point).unwrap() else {
            panic!()
        };
        match fault {
            0 => observed.value = Some(crate::check::dependencies::ScalarLiteral::Int(3)),
            1 => *owner += 1,
            2 => observed.control = !observed.control,
            3 => {
                observed.operation = false;
                observed.result = false;
            }
            4 => {
                let row = reports.effects[&point].clone();
                reports.effects.insert(usize::MAX, row);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn scalar_literal_seeds_bound_combined_rows_payload_and_exact_work() {
    let (mut checker, reports) = context("n<uint8>:7;yes:true;text:\"hello\"");
    let start = checker.flow.work;
    let (_, left) = checker
        .scalar_literal_seeds(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let parts = MAX_EDGES - left;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, extra, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .scalar_literal_seeds(&reports, Span::default(), limit)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}
