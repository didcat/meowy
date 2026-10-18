use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;
use crate::flow::TRUE;

#[test]
pub(crate) fn mutation_transfers_reject_late_stale_and_orphan_call_reports_atomically() {
    for fault in 0..9 {
        let (mut checker, program, mut reports, facts) =
            context("f:(p<&!int32>){};a:=1;b:=2;f(&!a);f(&!b)");
        let point = checker.invocations.last_key_value().unwrap().1.point;
        let (owner, effect) = reports.effects.get_mut(&point).unwrap();
        let Effect::Call {
            site,
            function,
            args,
            shape,
            operation,
            result,
            control,
            ..
        } = effect
        else {
            panic!()
        };
        match fault {
            0 => *owner += 1,
            1 => *site = usize::MAX,
            2 => *function = usize::MAX,
            3 => args.clear(),
            4 => *shape = crate::check::dependencies::Invocation::result_shape(&hir::Type::Bool),
            5 => {
                *operation = false;
                *result = false;
            }
            6 => *effect = Effect::Unknown,
            7 => {
                let row = reports.effects[&point].clone();
                reports.effects.insert(usize::MAX, row);
            }
            8 => *control = !*control,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let Err(error) =
            checker.call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        else {
            panic!()
        };
        assert!(
            error.message.contains("identity"),
            "fault {fault}: {error:?}"
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn mutation_transfers_ignore_final_reference_maps_and_preserve_fact_tables() {
    let source = "f:(p<&!boolean>,n<int32>){};a:=false;b:=true;p:=&!a;f(p,{p=&!b;->0});f(p,0)";
    let (mut checker, program, reports, facts) = context(source);
    let first = checker.place_borrows.first_key_value().unwrap().1.storage;
    let last = checker.place_borrows.last_key_value().unwrap().1.storage;
    assert_ne!(first, last);
    checker.pointees.clear();
    checker.reference_cells.clear();
    checker.record_cells.clear();
    checker.record_pointees.clear();
    checker.record_shapes.clear();
    checker.mark_derived(first);
    checker.bools.insert((first, vec![]), TRUE);
    let marks = checker.derived.clone();
    let bools = checker.bools.clone();
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let early = transfers.first_key_value().unwrap().1;
    let late = transfers.last_key_value().unwrap().1;
    assert_eq!(early.roots.keys().copied().collect::<Vec<_>>(), [first]);
    assert_eq!(late.roots.keys().copied().collect::<Vec<_>>(), [last]);
    assert_eq!(checker.derived, marks);
    assert_eq!(checker.bools, bools);
    assert!(checker.pointees.is_empty() && checker.reference_cells.is_empty());
}

#[test]
pub(crate) fn mutation_transfers_preserve_empty_and_nonreturning_call_boundaries() {
    for (source, count, result) in [
        ("empty:(){};empty()", 0, true),
        (
            "d:@\"debug\";stop<never>:(p<&!int32>){d.panic(\"stop\")};n:=1;stop(&!n)",
            1,
            false,
        ),
    ] {
        let (mut checker, program, reports, facts) = context(source);
        let (transfers, _) = checker
            .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let transfer = transfers.first_key_value().unwrap().1;
        assert_eq!(transfer.observed, Some((true, result)));
        assert_eq!(transfer.call.snapshot.unwrap().normal, count);
        assert_eq!(transfer.roots.len(), count);
        assert_eq!(transfer.unknown, Some(FALSE));
    }
}

#[test]
pub(crate) fn mutation_transfers_keep_false_entry_separate_from_operation_observations() {
    let (mut checker, program, reports, mut facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
    let snapshot = facts.call_inputs.first_entry().unwrap().into_mut();
    snapshot.header.entered = FALSE;
    checker.proofs.calls.insert(snapshot.header.site, FALSE);
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let transfer = transfers.first_key_value().unwrap().1;
    assert_eq!(transfer.observed, Some((true, true)));
    assert_eq!(transfer.unknown, Some(FALSE));
    assert_eq!(transfer.roots.len(), 1);
    assert!(transfer.roots.values().all(|guard| *guard == FALSE));
    assert!(
        transfer.call.inputs[0]
            .1
            .roots
            .values()
            .all(|guard| *guard == TRUE)
    );
}
