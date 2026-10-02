use super::*;

#[test]
pub(crate) fn operation_payload_remainder_preserves_shared_costs_and_duplicate_visits() {
    let (mut checker, mut reports) = checked("row:{->x:1;->y:2};xs:[row.x]", false);
    assert_eq!(reports.parts, MAX_EDGES - 9);
    let expected = reports.effects.clone();
    for (_, walk) in reports.entries.values_mut() {
        walk.ports.extend(walk.ports.clone());
    }
    for (parts, room) in [(9, 0), (12, 3)] {
        let (effects, remaining) = checker
            .operation_effects_with_room(&reports, Span::default(), expected.len(), parts, 0)
            .unwrap();
        assert_eq!(effects, expected);
        assert_eq!(remaining, room);
    }
    assert!(
        checker
            .operation_effects_with_room(&reports, Span::default(), expected.len(), 8, 0)
            .is_err()
    );
    assert_eq!(reports.effects, expected);
    assert_eq!(reports.parts, MAX_EDGES - 9);
}

pub(super) fn checked(source: &str, marked: bool) -> (Checker, Reports) {
    let ast = crate::parser::parse(source).unwrap();
    let mut checker = Checker::new();
    if marked {
        checker.derived.insert(0);
    }
    let body = checker.block(&ast, None, None).unwrap();
    let program = crate::hir::Program {
        body,
        functions: std::mem::take(&mut checker.functions)
            .into_iter()
            .flatten()
            .collect(),
        locals: std::mem::take(&mut checker.locals),
    };
    let reports = checker.entry_reports(&program, ast.span).unwrap();
    (checker, reports)
}

#[test]
pub(crate) fn operation_effects_preserve_alias_storage_reference_cells_and_control() {
    for source in [
        "c:=false;row:'out{|c|{'out->value:=1;value=2};|!c|{'out->value:=1;value=3}}",
        "a:1;b:2;r:=&a;r=&b;copy:*r",
        "flag:false;x:=1;|flag|x=2;f:(){y:3}",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let mut writes = 0;
        for (&id, op) in &checker.operations {
            if let Some((owner, effect)) = reports.effects.get(&id) {
                assert_eq!(*owner, op.owner);
                assert_eq!(
                    *effect,
                    Effect::Storage {
                        kind: op.kind,
                        local: op.local,
                        storage: op.storage,
                        input: op.input,
                        control: op.control
                    }
                );
                writes += usize::from(op.kind == OperationKind::Write);
            }
        }
        assert!(writes > 0);
        assert!(
            reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Scalar(_)))
        );
    }
    let (_, reports) = checked("flag:false;x:=1;|flag|x=2", true);
    assert!(reports.effects.values().any(|(_, effect)| matches!(
        effect,
        Effect::Storage {
            kind: OperationKind::Write,
            control: true,
            ..
        }
    )));
}

#[test]
pub(crate) fn operation_effects_exclude_stopped_writes_and_distinguish_store_kinds() {
    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};x:=1;x=stop()";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let writes: Vec<_> = checker
        .operations
        .iter()
        .filter(|(_, op)| op.kind == OperationKind::Write)
        .collect();
    assert_eq!(writes.len(), 1);
    assert!(!reports.effects.contains_key(writes[0].0));
    let (checker, reports) = checked(
        "row:{->x:=1};row.x=2;xs<int32[2]>:=[1,2];xs[1]=3;x:=1;r:&!x;*r=2",
        false,
    );
    assert!(!checker.paths.is_empty() && !checker.stores.is_empty());
    for id in checker.paths.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Path { .. }));
    }
    for id in checker.stores.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Indirect { .. }));
    }
    assert!(!reports.effects.values().any(|(_, effect)| matches!(
        effect,
        Effect::Storage {
            kind: OperationKind::Write,
            ..
        }
    )));
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::List(_)))
    );
}

#[test]
pub(crate) fn operation_effects_deduplicate_ports_and_reject_foreign_or_missing_owners() {
    let (mut checker, mut reports) = checked("x:=1;x=2", false);
    let expected = reports.effects.clone();
    let id = *checker.operations.first_key_value().unwrap().0;
    reports
        .entries
        .get_mut(&0)
        .unwrap()
        .1
        .ports
        .extend([Port::Operation(id); 2]);
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        expected
    );
    reports.index.operations.remove(&id);
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap_err()
            .message
            .contains("owner mismatch")
    );
    reports.index.operations.insert(id, 1);
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .is_err()
    );
    reports.index.operations.insert(id, 0);
    checker.operations.get_mut(&id).unwrap().owner = 1;
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .is_err()
    );
    assert_eq!(reports.effects, expected);
    let op = checker.operations.get_mut(&id).unwrap();
    op.owner = 0;
    op.input = None;
    let effects = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    assert!(matches!(
        effects[&id].1,
        Effect::Storage { input: None, .. }
    ));
}

#[test]
pub(crate) fn operation_effects_fail_atomically_at_capacity_and_late_work_limits() {
    let (mut checker, reports) = checked("x:=1;x=2;f:(){y:3}", true);
    let counts = checker.edge_counts();
    let marks = checker.derived.clone();
    assert!(!marks.is_empty());
    let before = checker.flow.work;
    let expected = checker
        .operation_effects(&reports, Span::default())
        .unwrap();
    let work = checker.flow.work - before;
    for limit in [0, expected.len() - 1] {
        assert!(
            checker
                .operation_effects_limited(&reports, Span::default(), limit, MAX_EDGES, MAX_EDGES)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    assert_eq!(
        checker
            .operation_effects_limited(
                &reports,
                Span::default(),
                expected.len(),
                MAX_EDGES,
                MAX_EDGES
            )
            .unwrap(),
        expected
    );
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work + 1;
    assert!(
        checker
            .operation_effects(&reports, Span::default())
            .is_err()
    );
    assert!(checker.flow.exceeded());
    assert_eq!(reports.effects, expected);
    assert_eq!(checker.edge_counts(), counts);
    assert_eq!(checker.derived, marks);
    checker.flow = crate::flow::Flow::new();
    checker.flow.work = crate::flow::MAX_PROOF_WORK - work;
    assert_eq!(
        checker
            .operation_effects(&reports, Span::default())
            .unwrap(),
        expected
    );
    assert_eq!(checker.flow.work, crate::flow::MAX_PROOF_WORK);
}
