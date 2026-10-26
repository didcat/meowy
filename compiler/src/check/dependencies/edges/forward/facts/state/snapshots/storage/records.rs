use super::super::capture::tests::{capture, capture_with};
use super::*;
use crate::check::dependencies::edges::forward::effects::Effect;

#[test]
pub(crate) fn record_storage_preserves_saved_primaries_versions_and_later_scalar_writes() {
    for source in [
        "x:=7;r:{->x;->tag:false;x=9};x=11;x",
        "x:=7;r:(({->x;->tag:false;x=9}));x=11;x",
        "x:=7;r:({->x;->tag:false;x=9}~<{-><int32>;tag<boolean>}>);x=11;x",
    ] {
        let (mut checker, program, reports, snapshots) = capture(source);
        let (&point, (_, Effect::Storage { local, input: Some(input), .. })) = reports.effects.iter().find(|(_, (_, effect))| matches!(effect, Effect::Storage { local, .. } if matches!(program.locals[*local], hir::Type::Record { .. }))).unwrap() else { panic!() };
        let Some(View::Known(state)) = snapshots.after(Port::Operation(point)) else {
            panic!("{source}")
        };
        let Data::Initialized {
            version: Some(version),
            domain: Some(domain),
            origin,
        } = &state.get(*local).unwrap().data
        else {
            panic!()
        };
        assert_eq!(version.definition, Definition::Bind(point));
        assert_eq!(version.local, *local);
        assert_eq!(
            *domain,
            Domain::integer(7, &mut checker.flow, Span::default()).unwrap()
        );
        assert_eq!(
            *origin,
            snapshots.records[input].1.value.as_ref().unwrap().origin
        );
        assert!(!snapshots.expressions.contains_key(input));
        let read = checker.local_reads.keys().next_back().unwrap();
        assert_eq!(
            snapshots.expressions[read].value.as_ref().unwrap().domain,
            Domain::integer(11, &mut checker.flow, Span::default()).unwrap()
        );
    }
}

#[test]
pub(crate) fn record_storage_keeps_unknown_primary_values_without_declared_domain_fallback() {
    let (mut checker, mut snapshots, definitions, _) =
        super::tests::setup("x:=7;r:{->x;->tag:false}");
    let (&point, record) = definitions.records.iter().next().unwrap();
    let storage = &record.storage;
    let before = snapshots.rows[&Port::Operation(point)].before;
    snapshots.records.get_mut(&storage.input).unwrap().1.value = None;
    let Some(Value::Known(id)) = snapshots
        .storage(
            point,
            before,
            &definitions,
            &mut checker.flow,
            Span::default(),
        )
        .unwrap()
    else {
        panic!()
    };
    let cell = snapshots.states[id].get(storage.version.local).unwrap();
    assert!(matches!(
        cell.data,
        Data::Initialized {
            domain: None,
            origin: None,
            ..
        }
    ));
    assert!(cell.control);
}

#[test]
pub(crate) fn record_storage_does_not_recover_absent_record_facts_or_unknown_state() {
    for missing in [false, true] {
        let source = if missing {
            "r:{->7;->tag:false}"
        } else {
            "r:{->7;->tag:false;1+2}"
        };
        let (_, program, reports, snapshots) = capture_with(source, |obs| {
            if missing {
                obs.records.clear();
            }
        });
        for (&point, (_, effect)) in &reports.effects {
            let Effect::Storage { local, .. } = effect else {
                continue;
            };
            if matches!(program.locals[*local], hir::Type::Record { .. }) {
                assert_eq!(snapshots.after(Port::Operation(point)), Some(View::Missing));
            }
        }
    }
}
