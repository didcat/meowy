use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

#[test]
pub(crate) fn call_storage_index_deduplicates_places_and_keeps_alias_views() {
    let source = "f:(p<&!int32>){};n:=1;f(&!n);f(&!n);r:{->n:=2;f(&!n)}";
    let (mut checker, program, reports, facts) = context(source);
    let blocks = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (storage, parts) = checker
        .mutation_storage(&program, &reports, &blocks, Span::default(), 5)
        .unwrap();
    assert_eq!(parts, 0);
    assert_eq!(storage.len(), 2);
    for snapshot in facts.call_inputs.values() {
        let origin = &snapshot.args[0].1.origins[0];
        let kind = ScalarKind::Int {
            bits: 32,
            signed: true,
        };
        assert!(
            checker
                .mutation_origin(
                    (&program, &reports, &blocks, &storage),
                    0,
                    kind,
                    origin,
                    Span::default()
                )
                .unwrap()
                .is_some()
        );
    }
}

#[test]
pub(crate) fn call_storage_index_rejects_stale_witnesses_and_keeps_unproved_origins_unknown() {
    let (mut checker, program, reports, facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
    let blocks = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (mut storage, _) = checker
        .mutation_storage(&program, &reports, &blocks, Span::default(), 1)
        .unwrap();
    let snapshot = facts.call_inputs.first_key_value().unwrap().1;
    let mut origin = snapshot.args[0].1.origins[0].clone();
    let kind = ScalarKind::Int {
        bits: 32,
        signed: true,
    };
    for source in [
        Source::Input {
            id: 0,
            component: vec![],
            fields: vec![],
        },
        Source::Expired { id: 0 },
        Source::Temporary {
            id: 0,
            statement: 0,
            fields: vec![],
        },
        Source::Local {
            id: 0,
            fields: vec![Projection::Element],
        },
    ] {
        origin.source = source;
        assert_eq!(
            checker
                .mutation_origin(
                    (&program, &reports, &blocks, &storage),
                    0,
                    kind,
                    &origin,
                    Span::default()
                )
                .unwrap(),
            None
        );
    }
    origin = snapshot.args[0].1.origins[0].clone();
    origin.component.push(crate::borrow_value::Step::Deref);
    assert_eq!(
        checker
            .mutation_origin(
                (&program, &reports, &blocks, &storage),
                0,
                kind,
                &origin,
                Span::default()
            )
            .unwrap(),
        None
    );
    origin.component.clear();
    storage.first_entry().unwrap().get_mut().0 = usize::MAX;
    assert!(
        checker
            .mutation_origin(
                (&program, &reports, &blocks, &storage),
                0,
                kind,
                &origin,
                Span::default()
            )
            .unwrap_err()
            .message
            .contains("identity")
    );
}

#[test]
pub(crate) fn call_storage_index_bounds_payload_and_late_work_without_publication() {
    let (mut checker, program, reports, _) =
        context("f:(p<&!int32>){};n:=1;f(&!n);r:{->n:=2;f(&!n)}");
    let blocks = checker
        .program_blocks(&program, &reports, Span::default(), MAX_EDGES)
        .unwrap();
    let start = checker.flow.work;
    let (_, left) = checker
        .mutation_storage(&program, &reports, &blocks, Span::default(), 5)
        .unwrap();
    assert_eq!(left, 0);
    let cost = checker.flow.work - start;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (parts, spare, pass) in [(4, 0, false), (5, 1, false), (5, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + spare;
        checker.flow.full = false;
        assert_eq!(
            checker
                .mutation_storage(&program, &reports, &blocks, Span::default(), parts)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}
