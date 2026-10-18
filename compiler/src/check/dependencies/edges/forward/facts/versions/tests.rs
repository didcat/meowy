use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

pub(in super::super) fn context(source: &str) -> (Checker, hir::Program, Reports) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports)
}

#[test]
pub(crate) fn scalar_versions_keep_equal_parameter_domains_in_distinct_frames_and_origins() {
    let (mut checker, mut program, reports) =
        context("f<int32>:(a<int32>,b<int32>){->a};g<int32>:(n<int32>){->n};f(7,7);g(7)");
    program.functions.reverse();
    let (_, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (versions, _) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    let values = versions.values().collect::<Vec<_>>();
    assert_eq!(values.len(), 3);
    for (local, (function, position, seed)) in &declared.params {
        let slot = &versions[local];
        assert_eq!(
            slot.version,
            Version {
                owner: function + 1,
                local: *local,
                definition: Definition::Parameter(*position)
            }
        );
        assert!(std::ptr::eq(slot.ty, seed.ty));
        assert_eq!(slot.declared, seed.domain);
        assert_eq!(
            slot.value.as_ref().unwrap().domain,
            *seed.domain.as_ref().unwrap()
        );
        assert_eq!(
            slot.value.as_ref().unwrap().origin,
            Origin::Parameter(*function, *position)
        );
        assert!(slot.source.is_none());
    }
    for pair in values.windows(2) {
        assert_eq!(pair[0].declared, pair[1].declared);
        assert_ne!(pair[0].version, pair[1].version);
        assert_ne!(
            pair[0].value.as_ref().unwrap().origin,
            pair[1].value.as_ref().unwrap().origin
        );
    }
}

#[test]
pub(crate) fn scalar_versions_keep_unavailable_values_separate_from_bottom_and_stopped_storage() {
    let (mut checker, program, reports) = context(
        "x:=1;a:7;text:\"cat\";f<never>:(n<never>){->n};d:@\"debug\";stop:d.panic(\"stop\");late:9",
    );
    let (_, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (versions, _) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    assert_eq!(versions.len(), 3);
    let param = &versions[&program.functions[0].params[0]];
    assert_eq!(param.value.as_ref().unwrap().domain, Domain::BOTTOM);
    for (local, init) in &reports.initializers {
        let slot = &versions[local];
        assert_eq!(slot.version.definition, Definition::Bind(init.statement));
        assert!(std::ptr::eq(
            slot.source.unwrap(),
            &reports.effects[&init.statement]
        ));
        assert!(slot.value.is_none());
        assert_eq!(slot.declared.is_none(), *slot.ty == hir::Type::String);
    }
    assert!(
        checker
            .proofs
            .mutable
            .iter()
            .all(|local| !versions.contains_key(local))
    );
}

#[test]
pub(crate) fn scalar_versions_bound_combined_payload_and_work_without_mutating_reports() {
    let (mut checker, program, reports) = context("a:7;b:a;f:(n<uint8>,v<boolean><null>){}");
    let (_, declared, parts) = checker
        .scalar_seed_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (_, left) = checker
        .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
        .unwrap();
    let cost = checker.flow.work - start;
    let used = parts - left;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (parts, extra, pass) in [(used - 1, 0, false), (used, 1, false), (used, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .scalar_version_seeds(&program, &reports, &declared, Span::default(), parts)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}
