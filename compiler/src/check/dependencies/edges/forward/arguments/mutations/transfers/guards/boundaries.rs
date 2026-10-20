use super::tests::controlled;
use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;
use crate::flow::{MAX_PROOF_WORK, TRUE};

#[test]
pub(crate) fn call_guard_boundaries_keep_independent_entry_separate_from_unknown_mutations() {
    let (mut checker, program, reports, facts) =
        context("sink:(p<&!int32>){};f:(p<&!int32>){sink(p)}");
    let (mut transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let (&point, transfer) = transfers.first_key_value().unwrap();
    assert_eq!(transfer.entry, Authority::Independent);
    assert_eq!(transfer.unknown, Some(TRUE));
    assert!(transfer.roots.is_empty());
    assert!(transfer.call.inputs[0].1.unknown);
    let transfer = transfers.get_mut(&point).unwrap();
    transfer.unknown = None;
    assert_eq!(
        checker
            .call_entry_authority(point, transfer, Span::default())
            .unwrap(),
        Authority::Unknown
    );
    assert_eq!(transfer.unknown, None);
}

#[test]
pub(crate) fn call_guard_boundaries_preserve_nonreturning_and_unvisited_call_evidence() {
    let (mut checker, program, reports, facts) =
        context("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};stop();stop()");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let mut transfers = transfers.values();
    let reached = transfers.next().unwrap();
    assert_eq!(reached.observed, Some((true, false)));
    assert_eq!(reached.entry, Authority::Independent);
    let later = transfers.next().unwrap();
    assert_eq!(later.observed, None);
    assert_eq!(later.entry, Authority::Unknown);
    let (mut checker, program, reports, facts) = controlled("flag:false;f:(){};|flag|{|false|f()}");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(
        transfers.values().all(
            |transfer| transfer.entry == Authority::Unknown && transfer.call.snapshot.is_none()
        )
    );
}

#[test]
pub(crate) fn call_guard_boundaries_reject_stale_or_cyclic_ancestry_before_publication() {
    for fault in 0..7 {
        let (mut checker, program, reports, facts) = context("f:(){};f();flag:false;|flag|f()");
        let call = checker.invocations.last_key_value().unwrap().1.clone();
        let parent = checker.points[call.point].parent.unwrap();
        match fault {
            0 => checker.points[call.point].parent = Some(call.point),
            1 => checker.points[parent].parent = Some(parent),
            2 => checker.points[parent].owner += 1,
            3 => checker.points[parent].complete = false,
            4 => checker.points[call.point].span.start += 1,
            5 => {
                checker.proofs.calls.insert(call.site, usize::MAX);
            }
            6 => checker.branch_inputs.values_mut().next().unwrap().owner += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = match checker.call_mutation_transfers(
            &program,
            &reports,
            &facts,
            Span::default(),
            MAX_EDGES,
        ) {
            Ok(_) => panic!("fault {fault}"),
            Err(error) => error,
        };
        assert!(error.message.contains("identity"), "{fault}: {error:?}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn call_guard_boundaries_use_captured_controls_and_preserve_exact_work() {
    let (mut checker, program, reports, facts) = context("flag:false;f:(){};|flag|f()");
    let (transfers, _) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let (&point, transfer) = transfers.first_key_value().unwrap();
    checker.control = true;
    checker.derived.insert(0);
    checker.bools.clear();
    checker.tags.clear();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let start = checker.flow.work;
    assert_eq!(
        checker
            .call_entry_authority(point, transfer, Span::default())
            .unwrap(),
        Authority::Independent
    );
    let cost = checker.flow.work - start;
    for extra in [1, 0] {
        checker.flow.work = MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result = checker.call_entry_authority(point, transfer, Span::default());
        if extra == 0 {
            assert_eq!(result.unwrap(), Authority::Independent);
            assert_eq!(checker.flow.work, MAX_PROOF_WORK);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(transfer.entry, Authority::Independent);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
        assert!(checker.control && checker.derived.contains(&0));
        assert!(checker.bools.is_empty() && checker.tags.is_empty());
    }
}

#[test]
pub(crate) fn call_guard_boundaries_reserve_authority_with_transfer_payload() {
    let (mut checker, program, reports, facts) =
        context("f:(p<&!int32>){};a:=1;b:=2;f(&!a);f(&!b)");
    let (_, targets) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (expected, left) = checker
        .call_mutation_transfers(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let cost = checker.flow.work - start;
    let parts = MAX_EDGES - left;
    assert_eq!(
        targets - left,
        expected.len() * 2
            + expected
                .values()
                .map(|transfer| transfer.roots.len())
                .sum::<usize>()
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (limit, extra, pass) in [(parts - 1, 0, false), (parts, 1, false), (parts, 0, true)] {
        checker.flow.work = MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        let result =
            checker.call_mutation_transfers(&program, &reports, &facts, Span::default(), limit);
        assert_eq!(result.is_ok(), pass);
        if let Ok((transfers, left)) = result {
            assert_eq!(left, 0);
            assert_eq!(transfers.len(), expected.len());
            for (point, transfer) in transfers {
                assert_eq!(transfer.entry, expected[&point].entry);
                assert_eq!(transfer.roots, expected[&point].roots);
                assert_eq!(transfer.unknown, expected[&point].unknown);
            }
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
