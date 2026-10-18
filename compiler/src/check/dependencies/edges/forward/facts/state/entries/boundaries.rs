use super::super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_entry_boundaries_share_exact_work_and_peak_payload_allowances() {
    for source in [
        "a:7;copy:a;x:=1;x=2;f:(n<uint8>){v:=n;v=n}",
        "f<never>:(a<int32>,b<boolean>,n<never>){->n}",
    ] {
        let (mut checker, program, reports) = context(source);
        let start = checker.flow.work;
        let (_, _, _, left) = checker
            .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let cost = checker.flow.work - start;
        let used = MAX_EDGES - left;
        let before = format!(
            "{reports:?}{:?}{:?}",
            checker.edge_counts(),
            checker.derived
        );
        for (parts, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
            checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
            checker.flow.full = false;
            let result = checker.scalar_state_context(&program, &reports, Span::default(), parts);
            assert_eq!(result.is_ok(), pass);
            if let Ok((_, _, _, left)) = result {
                assert_eq!(left, 0);
            }
            assert_eq!(
                format!(
                    "{reports:?}{:?}{:?}",
                    checker.edge_counts(),
                    checker.derived
                ),
                before
            );
        }
    }
}

#[test]
pub(crate) fn scalar_entry_boundaries_use_declarations_without_cached_or_candidate_values() {
    let (mut checker, program, reports) =
        context("a:7;copy:a;x:=1;x=2;f:(n<int32>){v:=n;v=2};f(9)");
    let (mut versions, events, parts) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let before = checker
        .scalar_entry_states(&reports, &versions, &events, Span::default(), parts)
        .unwrap();
    for slot in versions.values_mut() {
        slot.value = None;
    }
    checker.derived.insert(0);
    let marks = checker.derived.clone();
    checker.constants.clear();
    checker.inputs.clear();
    checker.bool_inputs.clear();
    checker.record_inputs.clear();
    checker.bools.clear();
    checker.tags.clear();
    checker.pointees.clear();
    let after = checker
        .scalar_entry_states(&reports, &versions, &events, Span::default(), parts)
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(checker.derived, marks);
}

#[test]
pub(crate) fn scalar_entry_boundaries_reject_missing_foreign_and_duplicate_declarations() {
    for fault in 0..3 {
        let (mut checker, program, mut reports) = context("f:(n<never>){v:=1;v=2}");
        if fault == 2 {
            reports.entries.remove(&1);
            let before = format!("{reports:?}");
            assert_eq!(
                checker
                    .scalar_state_context(&program, &reports, Span::default(), MAX_EDGES)
                    .unwrap_err()
                    .code,
                "B001"
            );
            assert_eq!(format!("{reports:?}"), before);
            continue;
        }
        let (mut versions, events, parts) = checker
            .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        if fault == 0 {
            let point = *events.first_key_value().unwrap().0;
            let event = &events[&point];
            let version = event.version.unwrap();
            versions.insert(
                version.local,
                versions::Slot {
                    version,
                    ty: event.ty,
                    declared: event.declared.clone(),
                    value: None,
                    source: Some(event.source),
                },
            );
        } else if fault == 1 {
            versions
                .get_mut(&program.functions[0].params[0])
                .unwrap()
                .version
                .owner += 1;
        }
        let before = format!("{reports:?}{versions:?}{events:?}");
        assert!(
            checker
                .scalar_entry_states(&reports, &versions, &events, Span::default(), parts)
                .unwrap_err()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{versions:?}{events:?}"), before);
    }
}
