use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_event_boundaries_share_exact_copy_event_and_domain_budgets() {
    let (mut checker, program, reports) = context("a:7;copy:a;x:=1;x=2;f:(n<uint8>){v:=n;v=n}");
    let start = checker.flow.work;
    let (_, _, left) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = MAX_EDGES - left;
    let before = format!("{reports:?}{:?}{:?}", checker.operations, checker.derived);
    for (parts, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.scalar_event_context(&program, &reports, Span::default(), parts);
        assert_eq!(result.is_ok(), pass);
        if let Ok((_, _, left)) = result {
            assert_eq!(left, 0);
        }
        assert_eq!(
            before,
            format!("{reports:?}{:?}{:?}", checker.operations, checker.derived)
        );
    }
}

#[test]
pub(crate) fn scalar_event_boundaries_separate_missing_operations_and_unknown_rhs() {
    for fault in 0..3 {
        let (mut checker, program, mut reports) =
            context("x:=1;x=2;d:@\"debug\";x=d.panic(\"stop\");x=3");
        let point = *checker
            .operations
            .iter()
            .find(|(_, op)| op.kind == OperationKind::Write)
            .unwrap()
            .0;
        match fault {
            0 => {}
            1 => {
                reports.effects.remove(&point);
            }
            2 => {
                let op = checker.operations.get_mut(&point).unwrap();
                op.input = None;
                op.edges.drain(..2);
                let (_, Effect::Storage { input, .. }) = reports.effects.get_mut(&point).unwrap()
                else {
                    panic!()
                };
                *input = None;
            }
            _ => unreachable!(),
        }
        let (_, events, _) = checker
            .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(events.len(), if fault == 1 { 1 } else { 2 });
        if fault == 2 {
            assert!(matches!(
                events[&point].source.1,
                Effect::Storage { input: None, .. }
            ));
            assert_eq!(
                events[&point].version.unwrap().definition,
                Definition::Write(point)
            );
            assert_ne!(events[&point].declared, Some(Domain::BOTTOM));
        }
        for (&id, op) in &checker.operations {
            if op.kind == OperationKind::Write && id != point {
                assert!(!events.contains_key(&id));
            }
        }
    }
}

#[test]
pub(crate) fn scalar_event_boundaries_preserve_control_and_ignore_frontend_caches() {
    let source = "seed:false;x:=1;|seed|x=2;copy:x";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let program = hir::Program {
        body,
        functions: Vec::new(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let marks = checker.derived.clone();
    let before = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(before.1.values().any(|event| matches!(
        event.source.1,
        Effect::Storage {
            kind: OperationKind::Write,
            control: true,
            ..
        }
    )));
    checker.constants.clear();
    checker.inputs.clear();
    checker.bool_inputs.clear();
    checker.record_inputs.clear();
    checker.bools.clear();
    checker.tags.clear();
    checker.pointees.clear();
    let after = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(format!("{before:?}"), format!("{after:?}"));
    assert_eq!(checker.derived, marks);
}

#[test]
pub(crate) fn scalar_event_boundaries_preserve_different_alias_and_storage_declarations() {
    let (mut checker, program, reports) =
        context("flag:=false;row:'out{|flag|{'out->n:=1;n=2};|!flag|{'out->n:=null;n=null}}");
    let (_, events, _) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(events.values().any(|event| event.ty != event.storage_ty));
    for event in events.values().filter(|event| {
        matches!(
            event.source.1,
            Effect::Storage {
                kind: OperationKind::Write,
                ..
            }
        )
    }) {
        assert!(event.version.is_none());
        assert_eq!(
            event.declared,
            Domain::declared(event.ty, &mut checker.flow, Span::default()).unwrap()
        );
    }
}

#[test]
pub(crate) fn scalar_event_boundaries_reject_late_orphan_reports_and_foreign_owners_atomically() {
    for fault in 0..3 {
        let (mut checker, program, mut reports) = context("x:=1;x=2;x=3");
        let point = *checker.operations.last_key_value().unwrap().0;
        match fault {
            0 => {
                checker.operations.remove(&point);
            }
            1 => reports.effects.get_mut(&point).unwrap().0 = 1,
            2 => checker.points[point].owner = 1,
            _ => unreachable!(),
        }
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.operations,
            checker.edge_counts()
        );
        let error = checker
            .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(
            before,
            format!(
                "{reports:?}{:?}{:?}",
                checker.operations,
                checker.edge_counts()
            )
        );
    }
}

#[test]
pub(crate) fn scalar_event_boundaries_keep_calls_paths_and_indirect_stores_as_separate_families() {
    let (mut checker, program, reports) =
        context("x:=1;p:&!x;*p=2;f:(v<&!int32>){*v=3};f(&!x);row:{->n:=1};row.n=2");
    let (_, events, _) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        !checker.paths.is_empty() && !checker.stores.is_empty() && !checker.invocations.is_empty()
    );
    for point in checker
        .paths
        .keys()
        .chain(checker.stores.keys())
        .copied()
        .chain(checker.invocations.values().map(|call| call.point))
    {
        assert!(!events.contains_key(&point));
    }
}
