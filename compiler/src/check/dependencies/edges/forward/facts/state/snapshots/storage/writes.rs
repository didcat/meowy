use super::super::capture::tests::capture;
use super::*;
use crate::check::dependencies::OperationKind;

#[test]
pub(crate) fn scalar_write_capture_replaces_versions_and_preserves_copied_histories() {
    let (mut checker, _, _, snapshots) = capture("x:=1;copy:x;x=2;x;copy;x=2;x;x=x;x");
    let mut writes = 0;
    let mut equal = 0;
    let mut same = 0;
    let mut local = None;
    for (&point, operation) in &checker.operations {
        if operation.kind != OperationKind::Write {
            continue;
        }
        let Some(View::Known(before)) = snapshots.before(Port::Operation(point)) else {
            panic!()
        };
        let Some(View::Known(after)) = snapshots.after(Port::Operation(point)) else {
            panic!()
        };
        let Data::Initialized {
            version: old,
            domain: prior,
            origin: source,
        } = &before.get(operation.storage).unwrap().data
        else {
            panic!()
        };
        let Data::Initialized {
            version,
            domain,
            origin,
        } = &after.get(operation.storage).unwrap().data
        else {
            panic!()
        };
        let rhs = snapshots.expressions[&operation.input.unwrap()]
            .value
            .as_ref()
            .unwrap();
        assert_eq!(
            *version,
            Some(Version {
                owner: operation.owner,
                local: operation.storage,
                definition: Definition::Write(point),
            })
        );
        assert_ne!(version, old);
        assert_eq!(domain.as_ref(), Some(&rhs.domain));
        assert_eq!(*origin, rhs.origin);
        assert_eq!(
            snapshots.after(Port::Normal(point)),
            Some(View::Known(after))
        );
        equal += usize::from(domain == prior);
        same += usize::from(origin == source);
        writes += 1;
        local = Some(operation.storage);
    }
    assert_eq!((writes, equal, same), (3, 2, 1));
    let copies = checker
        .local_reads
        .iter()
        .filter(|(_, read)| Some(read.local) != local);
    let mut found = 0;
    for (&point, _) in copies {
        let value = snapshots.expressions[&point].value.as_ref().unwrap();
        assert_eq!(
            value.domain,
            Domain::integer(1, &mut checker.flow, Span::default()).unwrap()
        );
        assert!(matches!(
            value.version.unwrap().definition,
            Definition::Bind(_)
        ));
        found += 1;
    }
    assert_eq!(found, 1);
}

#[test]
pub(crate) fn scalar_write_capture_replaces_joined_values_without_inventing_guard_authority() {
    let (mut checker, _, _, snapshots) = capture("x:=1;|true|x=2;before:x;x=3;x;before");
    let first = Domain::integer(1, &mut checker.flow, Span::default()).unwrap();
    let second = Domain::integer(2, &mut checker.flow, Span::default()).unwrap();
    let joined = first
        .join(&second, &mut checker.flow, Span::default())
        .unwrap();
    let last = Domain::integer(3, &mut checker.flow, Span::default()).unwrap();
    let values = checker
        .local_reads
        .keys()
        .map(|point| snapshots.expressions[point].value.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].domain, joined);
    assert_eq!(values[0].version, None);
    assert_eq!(values[0].origin, None);
    assert_eq!(values[1].domain, last);
    assert!(matches!(
        values[1].version.unwrap().definition,
        Definition::Write(_)
    ));
    assert_eq!(values[2].domain, joined);
    assert!(matches!(
        values[2].origin,
        Some(Origin::Version(Version {
            definition: Definition::Bind(_),
            ..
        }))
    ));
}

#[test]
pub(crate) fn scalar_write_capture_cannot_restore_state_after_unqualified_effects() {
    for middle in ["g()", "r:={->n:=1};r.n=2"] {
        let (checker, _, _, snapshots) = capture(&format!("g:(){{}};x:=1;x;{middle};x=2;x"));
        let write = checker
            .operations
            .iter()
            .find(|(_, op)| op.kind == OperationKind::Write)
            .unwrap();
        assert_eq!(
            snapshots.before(Port::Operation(*write.0)),
            Some(View::Missing)
        );
        assert_eq!(
            snapshots.after(Port::Operation(*write.0)),
            Some(View::Missing)
        );
        let reads = checker
            .local_reads
            .iter()
            .filter(|(_, read)| read.local == write.1.storage)
            .collect::<Vec<_>>();
        assert_eq!(reads.len(), 2);
        assert!(snapshots.expressions[reads[0].0].value.is_some());
        assert!(snapshots.expressions[reads[1].0].value.is_none());
    }
}
