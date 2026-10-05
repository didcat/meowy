use super::{tests::checked, *};
use crate::check::dependencies::{
    edges::forward::results::inputs::graph::Visit, grouped::MAX_GROUPS,
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn receiver_sources_link_scalar_aliases_inside_ordinary_nested_blocks() {
    let (mut checker, reports) = checked("r:{->n:1};n:r.n;s:n.{inner:{->(($))}}");
    let (&read, op) = checker
        .local_reads
        .iter()
        .find(|(_, read)| reports.receivers.contains_key(&read.local))
        .unwrap();
    assert!(!reports.initializers.contains_key(&op.local));
    assert_eq!(
        checker
            .read_initializer_input(&reports, read, 0, Span::default())
            .unwrap(),
        None
    );
    assert!(
        reports
            .direct_sources
            .values()
            .any(|(_, direct)| direct.source.is_some())
    );
    assert!(
        reports
            .expanded_walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Field(_, _, _)))
    );
}

#[test]
pub(crate) fn receiver_sources_preserve_field_block_dispatch_and_candidate_identities() {
    let source = "r:{->n:1};a:r.n.{alias<int32>:(($~<int32>));nested:{->alias};->$};b:{->2}.{->$};c:3.{->$}.{->$};out:{->a};f<int32>:(){r:{->n:5};->r.n.{->$}}";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut families = BTreeSet::new();
    for (&key, &(owner, direct)) in &reports.direct_sources {
        if !checker.proofs.dispatches.contains(&key.0) {
            continue;
        }
        let input = reports.candidate_inputs[&key].1;
        assert_eq!(direct.point, input.point);
        assert!(
            reports
                .candidate_walk
                .visits
                .contains(&Visit::Value(key, input))
        );
        let visit = if let Some(source) = direct.source {
            families.insert((owner, "field"));
            assert_eq!(
                reports.field_results[&Port::Normal(source.field)],
                (owner, source.slot)
            );
            Visit::Field(key, input, source)
        } else if let Some(source) = direct.block {
            families.insert((owner, "block"));
            assert_eq!(
                reports.consumers[&source.consumer],
                (owner, source.slot.block)
            );
            Visit::Block(key, input, source)
        } else if let Some(source) = direct.dispatch {
            families.insert((owner, "dispatch"));
            assert_eq!(checker.dispatch_ops[&source.point].block, source.slot.block);
            Visit::Dispatch(key, input, source)
        } else {
            continue;
        };
        assert!(reports.expanded_walk.visits.contains(&visit));
    }
    assert_eq!(
        families,
        BTreeSet::from([(0, "field"), (0, "block"), (0, "dispatch"), (1, "field")])
    );
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
    );
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn receiver_sources_keep_calls_dereferences_and_nonscalar_inputs_opaque() {
    for source in [
        "f<int32>:(){->3};v:f().{->$}",
        "n:3;p:&n;v:(*p).{->$}",
        "v:(1+2).{->$}",
        "r:{->n:3};v:r.{->$}",
        "v:[1,2].{->$}",
        "n<int32><null>:3;v:n.{->$}",
        "n:3;v:(&n).{->$}",
    ] {
        crate::compile(source).unwrap();
        let (_, reports) = checked(source);
        assert!(
            reports
                .direct_sources
                .values()
                .all(|(_, direct)| direct.source.is_none()
                    && direct.block.is_none()
                    && direct.dispatch.is_none()),
            "{source}"
        );
    }
}

#[test]
pub(crate) fn receiver_sources_require_initialization_without_requiring_dispatch_results() {
    let (mut checker, mut reports) = checked("r:{->n:1};v:r.n.{nested:{->$};->$}");
    let (&dispatch, op) = checker.dispatch_ops.first_key_value().unwrap();
    let local = op.local;
    let read = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local == local)
        .unwrap()
        .0;
    let mut ctx = field_results::lookup::Lookup::new(&reports, MAX_EDGES);
    let expected = checker
        .field_narrowing_source(&mut ctx, read, 0, Span::default(), MAX_GROUPS)
        .unwrap();
    assert!(expected.is_some());
    for initialized in [true, false] {
        for result in [true, false] {
            let (_, Effect::Dispatch(op)) = reports.effects.get_mut(&dispatch).unwrap() else {
                panic!()
            };
            op.initialized = initialized;
            op.result = result;
            let mut ctx = field_results::lookup::Lookup::new(&reports, MAX_EDGES);
            assert_eq!(
                checker
                    .field_narrowing_source(&mut ctx, read, 0, Span::default(), MAX_GROUPS)
                    .unwrap(),
                if initialized { expected } else { None }
            );
        }
    }
    let (mut checker, reports) =
        checked("d:@\"debug\";r:{->n:1};v:r.n.{nested:{->$};d.panic(\"stop\")}");
    let local = checker.dispatch_ops.values().next().unwrap().local;
    let read = *checker
        .local_reads
        .iter()
        .find(|(_, read)| read.local == local)
        .unwrap()
        .0;
    let mut ctx = field_results::lookup::Lookup::new(&reports, MAX_EDGES);
    assert!(
        checker
            .field_narrowing_source(&mut ctx, read, 0, Span::default(), MAX_GROUPS)
            .unwrap()
            .is_some()
    );
}
