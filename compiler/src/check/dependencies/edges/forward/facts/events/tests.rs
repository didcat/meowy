use super::super::versions::tests::context;
use super::*;

#[test]
pub(crate) fn scalar_events_borrow_full_types_and_distinguish_bind_write_versions() {
    let (mut checker, program, reports) =
        context("x:=1;before:x;x=2;after:x;x=3;f<int32>:(n<int32>){value:=n;value=n;->value}");
    let (versions, events, _) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(events.len(), 5);
    let mut writes = Vec::new();
    for (&point, event) in &events {
        let (
            owner,
            Effect::Storage {
                kind,
                local,
                storage,
                input,
                ..
            },
        ) = event.source
        else {
            panic!()
        };
        let version = event.version.unwrap();
        assert_eq!((version.owner, version.local), (*owner, *storage));
        assert_eq!(
            version.definition,
            match kind {
                OperationKind::Bind => Definition::Bind(point),
                OperationKind::Write => Definition::Write(point),
            }
        );
        assert!(input.is_some());
        assert!(std::ptr::eq(event.source, &reports.effects[&point]));
        assert!(std::ptr::eq(event.ty, &program.locals[*local]));
        assert!(std::ptr::eq(event.storage_ty, &program.locals[*storage]));
        assert_eq!(
            event.declared,
            Domain::declared(event.ty, &mut checker.flow, Span::default()).unwrap()
        );
        assert!(!versions.contains_key(local));
        assert!(!reports.initializers.contains_key(local));
        if *kind == OperationKind::Write && *owner == 0 {
            writes.push(version);
        }
    }
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].local, writes[1].local);
    assert_ne!(writes[0], writes[1]);
    assert!(versions[&1].value.is_none() && versions[&2].value.is_none());
}

#[test]
pub(crate) fn scalar_events_retain_opaque_and_nullable_declarations_without_inventing_values() {
    let (mut checker, program, reports) = context(
        "a:1;b:2;r:=&a;r=&b;text:=\"cat\";text=\"dog\";n:=1.5;n=2.5;row:={->x:1};row={->x:2};maybe<int32><null>:=1;maybe=null",
    );
    let (_, events, _) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(events.len(), 10);
    assert_eq!(
        events
            .values()
            .filter(|event| event.version.is_some())
            .count(),
        2
    );
    for event in events.values() {
        if matches!(event.ty, hir::Type::Union(_)) {
            assert!(event.declared.as_ref().unwrap().null);
            assert!(event.declared.as_ref().unwrap().values.is_some());
            assert!(event.version.is_some());
        } else {
            assert!(event.declared.is_none() && event.version.is_none());
        }
    }
}

#[test]
pub(crate) fn scalar_events_retain_alias_writes_without_treating_them_as_plain_versions() {
    let (mut checker, program, reports) =
        context("flag:=false;row:'out{|flag|{'out->n:=1;n=2};|!flag|{'out->n:=3;n=4}};row.n=5");
    let (_, events, _) = checker
        .scalar_event_context(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(events.len(), 3);
    let mut roots = Vec::new();
    for event in events.values() {
        let (
            _,
            Effect::Storage {
                kind,
                local,
                storage,
                ..
            },
        ) = event.source
        else {
            panic!()
        };
        if *kind == OperationKind::Write {
            assert!(checker.proofs.aliases.contains_key(local));
            assert!(event.version.is_none());
            assert!(event.declared.is_some());
            roots.push(*storage);
        }
    }
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0], roots[1]);
    assert!(!checker.paths.is_empty());
    assert!(
        checker
            .paths
            .keys()
            .all(|point| !events.contains_key(point))
    );
}
