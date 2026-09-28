use super::*;

pub(super) fn checked(source: &str) -> (Checker, crate::hir::Program) {
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    let body = checker.block(&ast, None, None).unwrap();
    let program = crate::hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    (checker, program)
}

#[test]
pub(crate) fn entry_reports_retain_unused_recursive_functions_and_resolvable_positions() {
    let source =
        "f<int32>:(n<int32>) 'r {|n==0|{'r->0;'r.leave()};->f(n-1)};g<int32>:(n<int32>){->n+1};x:1";
    crate::compile(source).unwrap();
    let (mut checker, mut program) = checked(source);
    program.functions.reverse();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(reports.entries.len(), 3);
    assert_eq!(reports.entries[&0].0, program.body.id);
    let mut positions = std::collections::BTreeSet::new();
    for (&owner, (block, walk)) in &reports.entries {
        assert_eq!(walk.ports[0], Port::BlockEntry(*block));
        assert!(!walk.forward.is_empty());
        for &port in &walk.ports {
            assert_eq!(checker.port_owner(port, Span::default()).unwrap(), owner);
        }
        for &position in walk.forward.iter().chain(&walk.backedges) {
            assert!(positions.insert(position));
            let (_, edge) = reports.index.edges[position];
            assert!(walk.ports.contains(&edge.from));
            assert_eq!(
                edge.route == Route::Backedge,
                walk.backedges.contains(&position)
            );
        }
    }
    for function in &program.functions {
        assert_eq!(reports.entries[&(function.id + 1)].0, function.body.id);
        assert!(
            !reports.entries[&0]
                .1
                .ports
                .contains(&Port::BlockEntry(function.body.id))
        );
    }
    assert_eq!(
        reports.items,
        reports.entries.values().map(|(_, walk)| walk.len()).sum()
    );
}

#[test]
pub(crate) fn entry_reports_reject_duplicate_wrong_and_overflowing_function_owners() {
    let (mut checker, program) = checked("f<int32>:(){->1};g<int32>:(){->2}");
    for id in [program.functions[0].id, 99, usize::MAX] {
        let mut invalid = program.clone();
        invalid.functions[1].id = id;
        assert!(
            checker
                .entry_reports(&invalid, Span::default())
                .unwrap_err()
                .message
                .contains("owner mismatch")
        );
    }
    let mut invalid = program.clone();
    invalid.functions[1].body.id = program.body.id;
    assert!(
        checker
            .entry_reports(&invalid, Span::default())
            .unwrap_err()
            .message
            .contains("owner mismatch")
    );
    assert_eq!(
        checker
            .entry_reports(&program, Span::default())
            .unwrap()
            .entries
            .len(),
        3
    );
}

#[test]
pub(crate) fn entry_reports_keep_empty_programs_and_reject_missing_or_wrong_program_entries() {
    let (mut checker, program) = checked("");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(reports.entries.len(), 1);
    assert_eq!(reports.entries[&0].0, program.body.id);
    assert_eq!(
        reports.entries[&0].1.ports[0],
        Port::BlockEntry(program.body.id)
    );
    let mut missing = program;
    missing.body.id = usize::MAX;
    assert!(
        checker
            .entry_reports(&missing, Span::default())
            .unwrap_err()
            .message
            .contains("identity mismatch")
    );
    let (mut checker, mut wrong) = checked("f<int32>:(){->1}");
    wrong.body.id = wrong.functions[0].body.id;
    assert!(
        checker
            .entry_reports(&wrong, Span::default())
            .unwrap_err()
            .message
            .contains("owner mismatch")
    );
}

#[test]
pub(crate) fn entry_reports_bound_roots_aggregate_items_and_late_work_atomically() {
    let (mut checker, program) = checked("f<int32>:(){->1};g<int32>:(){->2}");
    let counts = checker.edge_counts();
    let before = checker.flow.work;
    let expected = checker.entry_reports(&program, Span::default()).unwrap();
    let work = checker.flow.work - before;
    for roots in [0, 2] {
        assert!(
            checker
                .entry_reports_limited(&program, Span::default(), roots, MAX_REPORT_ITEMS)
                .unwrap_err()
                .message
                .contains("entry-report budget")
        );
    }
    for items in [0, expected.items - 1] {
        assert!(
            checker
                .entry_reports_limited(&program, Span::default(), 3, items)
                .unwrap_err()
                .message
                .contains("structural-walk budget")
        );
    }
    let exact = checker
        .entry_reports_limited(&program, Span::default(), 3, expected.items)
        .unwrap();
    assert_eq!(exact.entries, expected.entries);
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(checker.entry_reports(&program, Span::default()).is_err());
    assert!(checker.flow.exceeded());
    assert_eq!(checker.edge_counts(), counts);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    let retry = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
    assert_eq!(retry.entries, expected.entries);
    assert_eq!(retry.index.edges, expected.index.edges);
}
