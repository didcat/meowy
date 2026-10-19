use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_copy_lookup_indexes_preserve_ordered_values_and_deterministic_work() {
    let source = format!(
        "{}tail:n63;copy:tail",
        (0..64)
            .map(|index| format!("n{index}:{index};"))
            .collect::<String>()
    );
    let (mut checker, program, reports) = context(&source);
    let mut expected = None;
    for _ in 0..4 {
        let start = checker.flow.work;
        let (versions, left) = checker
            .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let cost = checker.flow.work - start;
        let values = versions
            .into_iter()
            .map(|(local, slot)| (local, (slot.version, slot.value)))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(values[&63].1, values[&64].1);
        assert_eq!(values[&64].1, values[&65].1);
        let actual = (values, left, cost);
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
    }
}

#[test]
pub(crate) fn scalar_copies_preserve_origins_without_merging_equal_literals_or_storage_versions() {
    let (mut checker, program, reports) =
        context("a:7;same:7;copy:((a));again:copy~<int32>;yes:true;flag:yes;none:null;empty:none");
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let slots = versions.values().collect::<Vec<_>>();
    assert_eq!(slots.len(), 8);
    let values = slots
        .iter()
        .map(|slot| slot.value.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values[0].domain, values[1].domain);
    assert_ne!(values[0].origin, values[1].origin);
    for (first, copy) in [(0, 2), (2, 3), (4, 5), (6, 7)] {
        assert_eq!(values[first], values[copy]);
        assert_ne!(slots[first].version, slots[copy].version);
    }
    assert!(
        slots[..4]
            .iter()
            .all(|slot| slot.declared.as_ref() != Some(&slot.value.as_ref().unwrap().domain))
    );
}

#[test]
pub(crate) fn scalar_copies_keep_declared_parameter_values_and_leave_computation_unavailable() {
    let (mut checker, program, reports) =
        context("f<int32>:(n<int32>,other<int32>){a:n;b:a;sum:n+0;->b};answer:f(7,7)");
    let (versions, _) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let slots = versions.values().collect::<Vec<_>>();
    assert_eq!(slots.len(), 6);
    let param = slots[0].value.as_ref().unwrap();
    assert_eq!(Some(&param.domain), slots[0].declared.as_ref());
    assert_ne!(param.origin, slots[1].value.as_ref().unwrap().origin);
    assert_eq!(slots[0].value, slots[2].value);
    assert_eq!(slots[2].value, slots[3].value);
    assert!(slots[4].value.is_none() && slots[5].value.is_none());
}

#[test]
pub(crate) fn scalar_copies_resolve_dependencies_without_numeric_local_order() {
    let (mut checker, program, reports) = context("a:7;b:a;c:b");
    let (literals, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (versions, parts) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    let (inputs, parts) = checker
        .scalar_copy_inputs(&reports, &versions, Span::default(), parts)
        .unwrap();
    let versions = versions
        .into_iter()
        .map(|(local, mut slot)| {
            slot.version.local = 2 - local;
            (2 - local, slot)
        })
        .collect();
    let inputs = inputs
        .into_iter()
        .map(|(local, mut input)| {
            if let Producer::Read(_, version) = &mut input.producer {
                version.local = 2 - version.local;
            }
            (2 - local, input)
        })
        .collect();
    let (versions, _) = checker
        .scalar_copies(versions, &inputs, &literals, Span::default(), parts)
        .unwrap();
    assert!(versions.values().all(|slot| slot.value.is_some()));
    assert_eq!(versions[&0].value, versions[&2].value);
    assert_eq!(versions[&1].value, versions[&2].value);
}

#[test]
pub(crate) fn scalar_copies_reject_cycles_stale_versions_and_foreign_frames_atomically() {
    for fault in 0..3 {
        let (mut checker, program, reports) = context("a:7;b:a");
        let (literals, declared, parts) = checker
            .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
            .unwrap();
        let (versions, parts) = checker
            .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
            .unwrap();
        let (mut inputs, parts) = checker
            .scalar_copy_inputs(&reports, &versions, Span::default(), parts)
            .unwrap();
        let Producer::Read(_, version) = &mut inputs.get_mut(&1).unwrap().producer else {
            panic!()
        };
        match fault {
            0 => *version = versions[&1].version,
            1 => version.definition = Definition::Bind(usize::MAX),
            2 => version.owner += 1,
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let error = checker
            .scalar_copies(versions, &inputs, &literals, Span::default(), parts)
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn scalar_copies_share_exact_seed_source_version_and_value_budgets() {
    let (mut checker, program, reports) = context("a:7;b:a;c:b;f:(n<uint8>){copy:n}");
    let start = checker.flow.work;
    let (_, left) = checker
        .scalar_copy_context(&program, &reports, Span::default(), MAX_EDGES)
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
        let result = checker.scalar_copy_context(&program, &reports, Span::default(), parts);
        assert_eq!(result.is_ok(), pass);
        if let Ok((_, left)) = result {
            assert_eq!(left, 0);
        }
        assert_eq!(
            before,
            format!(
                "{reports:?}{:?}{:?}",
                checker.edge_counts(),
                checker.derived
            )
        );
    }
}
