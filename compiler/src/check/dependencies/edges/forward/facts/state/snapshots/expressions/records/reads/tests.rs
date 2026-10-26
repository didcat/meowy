use super::super::super::super::capture::tests::{capture, capture_with};
use super::*;

#[test]
pub(crate) fn record_reads_and_copies_keep_saved_primary_origins_and_distinct_versions() {
    let (mut checker, program, _, snapshots) =
        capture("x:=7;r:{->x;->tag:false;x=9};x=11;copy:((r));x=13;r;copy;x");
    let mut versions = Vec::new();
    let mut origins = Vec::new();
    for (&point, read) in &checker.local_reads {
        if !matches!(program.locals[read.local], hir::Type::Record { .. }) {
            continue;
        }
        assert!(!snapshots.expressions.contains_key(&point));
        let value = snapshots.records[&point].1.value.as_ref().unwrap();
        assert_eq!(
            value.domain,
            Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
        );
        assert_eq!(value.local, Some(read.local));
        assert_eq!(value.version.unwrap().local, read.local);
        assert!(!value.control);
        versions.push(value.version.unwrap().local);
        origins.push(value.origin.unwrap());
    }
    versions.sort_unstable();
    versions.dedup();
    assert_eq!(versions.len(), 2);
    assert!(origins.len() >= 3 && origins.iter().all(|origin| *origin == origins[0]));
    let read = checker.local_reads.keys().next_back().unwrap();
    assert_eq!(
        snapshots.expressions[read].value.as_ref().unwrap().domain,
        Domain::integer(13, &mut checker.flow, Span::default()).unwrap()
    );
}

#[test]
pub(crate) fn record_reads_preserve_exact_parameter_domains_groups_and_ascribed_copies() {
    let (checker, program, _, snapshots) =
        capture("f:(n<uint64>){r:{->n;->tag:true};copy:((r)~<{-><uint64>;tag<boolean>}>);copy}");
    for (&point, read) in &checker.local_reads {
        if !matches!(program.locals[read.local], hir::Type::Record { .. }) {
            continue;
        }
        let value = snapshots.records[&point].1.value.as_ref().unwrap();
        assert_eq!(value.origin, Some(Origin::Parameter(0, 0)));
        assert_eq!(
            value.domain,
            snapshots
                .records
                .values()
                .find(|(_, fact)| fact
                    .value
                    .as_ref()
                    .is_some_and(|value| value.local.is_none()))
                .unwrap()
                .1
                .value
                .as_ref()
                .unwrap()
                .domain
        );
    }
}

#[test]
pub(crate) fn record_reads_keep_unknown_current_history_and_unsupported_sources_opaque() {
    for source in ["r:{->7;->tag:false};1+2;r", "r:{->7;->tag:false};|true|1;r"] {
        let (checker, program, _, snapshots) = capture_with(source, |obs| {
            for guard in obs.guards.values_mut() {
                guard.control = None;
            }
        });
        for (&point, read) in &checker.local_reads {
            if matches!(program.locals[read.local], hir::Type::Record { .. }) {
                assert!(snapshots.records[&point].1.value.is_none());
            }
        }
    }
    for source in [
        "r:={->7;->tag:false};r",
        "r:{->7;->tag:=false};r",
        "f:(r<{-><int32>;tag<boolean>}>){r}",
    ] {
        let (checker, program, _, snapshots) = capture(source);
        for (&point, read) in &checker.local_reads {
            if matches!(program.locals[read.local], hir::Type::Record { .. }) {
                assert!(!snapshots.records.contains_key(&point));
            }
        }
    }
}
