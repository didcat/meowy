use super::*;

pub(super) fn checked(source: &str) -> (Checker, hir::Program) {
    let mut checker = Checker::new();
    let body = checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let program = hir::Program {
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
pub(crate) fn receiver_index_keeps_nested_and_independent_owners_after_hir_transfer() {
    let source = "v:3.{inner:4.{->$};nested:{->$};->$};f<int32>:(n<int32>){->n.{->$}}";
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    assert!(checker.locals.is_empty());
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    assert_eq!(reports.receivers.len(), 3);
    assert!(reports.receivers.values().any(|(owner, _)| *owner != 0));
    for (&local, &(owner, id)) in &reports.receivers {
        let op = &checker.dispatch_ops[&id];
        assert_eq!((op.local, op.owner), (local, owner));
        assert!(!reports.initializers.contains_key(&local));
        assert_eq!(checker.bodies[&op.block].sources[0], None);
    }
    for source in [
        "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};v:stop().{}",
        "d:@\"debug\";v:3.{d.panic(\"stop\")}",
        "r:{->n:1};v:r.{->$.n}",
    ] {
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        assert_eq!(reports.receivers.len(), 1);
    }
}

#[test]
pub(crate) fn receiver_index_rejects_corrupt_local_producer_and_body_identities() {
    for fault in 0..13 {
        let (mut checker, mut program) = checked("a:3.{->$};b:4.{->$}");
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let (&id, op) = checker.dispatch_ops.last_key_value().unwrap();
        let (local, block, input) = (op.local, op.block, op.input);
        match fault {
            0 => program.locals[local] = hir::Type::Bool,
            1 => checker.dispatch_ops.get_mut(&id).unwrap().owner += 1,
            2 => checker.dispatch_ops.get_mut(&id).unwrap().local = usize::MAX,
            3 => checker.points[input].parent = None,
            4 => checker.points[input].span.end = usize::MAX,
            5 => checker.bodies.get_mut(&block).unwrap().parent = None,
            6 => checker.bodies.get_mut(&block).unwrap().facts[0].0 = Fact::Bind(usize::MAX),
            7 => checker.bodies.get_mut(&block).unwrap().sources[0] = Some(id),
            8 => {
                checker.proofs.receivers.remove(&local);
            }
            9 => {
                checker.proofs.dispatches.remove(&block);
            }
            10 => checker.dispatch_ops.get_mut(&id).unwrap().input_normal = false,
            11 => checker.points[id].block = Some(block),
            12 => {
                let prior = checker.dispatch_ops.values().next().unwrap().local;
                checker.dispatch_ops.get_mut(&id).unwrap().local = prior;
                let body = checker.bodies.get_mut(&block).unwrap();
                body.facts[0].0 = Fact::Bind(prior);
                body.storage[0] = Some(prior);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .receiver_index(&program, &reports, Span::default(), MAX_EDGES, 2)
                .unwrap_err()
                .message
                .contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn receiver_index_shares_exact_capacity_payload_and_work_atomically() {
    let (mut checker, program) = checked("a:3.{->$};b:4.{->$}");
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .receiver_index(&program, &reports, Span::default(), 2, 2)
            .unwrap(),
        (reports.receivers.clone(), 0)
    );
    let work = checker.flow.work - start;
    for (limit, parts) in [(1, 2), (2, 1)] {
        assert!(
            checker
                .receiver_index(&program, &reports, Span::default(), limit, parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
    }
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.receiver_index(&program, &reports, Span::default(), 2, 2);
        assert_eq!(result.is_ok(), short == 0);
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}
