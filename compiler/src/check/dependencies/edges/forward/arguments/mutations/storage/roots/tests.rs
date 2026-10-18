use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;

#[test]
pub(crate) fn call_fact_roots_merge_field_guards_without_losing_original_sources() {
    let source =
        "sink:(p<&!int32>){};f:(flag<boolean>){r:{->a:=1;->b:=2};p:=r.&!a;|flag|p=r.&!b;sink(p)}";
    let (mut checker, program, reports, facts) = context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let target = &calls.first_key_value().unwrap().1.inputs[0].1;
    assert!(!target.unknown);
    assert_eq!(target.origins.len(), 2);
    assert_ne!(target.origins[0].0.source, target.origins[1].0.source);
    let root = checker.place_borrows[&target.origins[0].1].storage;
    let guard = checker
        .flow
        .or(target.origins[0].0.guard, target.origins[1].0.guard);
    assert_eq!(target.roots, Roots::from([(root, guard)]));
    assert_eq!(guard, crate::flow::TRUE);
}

#[test]
pub(crate) fn call_fact_roots_share_alias_storage_but_keep_guarded_lexical_views() {
    let source = "sink:(p<&!int32>){};f:(flag<boolean>)'out{|flag|{'out->n:=1;sink(&!n)};|!flag|{'out->n:=2;sink(&!n)}}";
    let (mut checker, program, reports, facts) = context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let roots = calls
        .values()
        .map(|call| {
            let target = &call.inputs[0].1;
            assert_eq!(target.roots.len(), 1);
            let (root, guard) = target.roots.first_key_value().unwrap();
            assert_eq!(*guard, target.origins[0].0.guard);
            (*root, &target.origins[0].0.source)
        })
        .collect::<Vec<_>>();
    assert_eq!(roots[0].0, roots[1].0);
    assert_ne!(roots[0].1, roots[1].1);
}

#[test]
pub(crate) fn call_fact_roots_bound_distinct_roots_and_exact_work_atomically() {
    let (mut checker, program, reports, facts) =
        context("f:(a<&!int32>,b<&!int32>){};a:=1;b:=2;f(&!a,&!b)");
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let origins = calls
        .first_key_value()
        .unwrap()
        .1
        .inputs
        .iter()
        .flat_map(|(_, target)| target.origins.iter().copied())
        .collect::<Vec<_>>();
    let start = checker.flow.work;
    let (roots, left) = checker
        .mutation_fact_roots(&reports, 0, &origins, Span::default(), 2)
        .unwrap();
    let cost = checker.flow.work - start;
    assert_eq!(left, 0);
    assert_eq!(roots.len(), 2);
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (parts, extra, pass) in [(1, 0, false), (2, 1, false), (2, 0, true)] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - cost + extra;
        checker.flow.full = false;
        assert_eq!(
            checker
                .mutation_fact_roots(&reports, 0, &origins, Span::default(), parts)
                .is_ok(),
            pass
        );
        assert_eq!(before, format!("{reports:?}{:?}", checker.edge_counts()));
    }
}

#[test]
pub(crate) fn call_fact_roots_reject_stale_storage_sources_owners_and_guards() {
    for fault in 0..7 {
        let (mut checker, program, reports, facts) = context("f:(p<&!int32>){};n:=1;f(&!n)");
        let (calls, _) = checker
            .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let target = &calls.first_key_value().unwrap().1.inputs[0].1;
        let (origin, point) = target.origins[0];
        let mut origin = origin.clone();
        let op = checker.place_borrows.get_mut(&point).unwrap();
        match fault {
            0 => op.owner += 1,
            1 => op.storage = usize::MAX,
            2 => {
                checker.proofs.bindings.remove(&op.storage);
            }
            3 => op.place.root = usize::MAX,
            4 => origin.component.push(crate::borrow_value::Step::Deref),
            5 => origin.guard = usize::MAX,
            6 => op.mode = ReferenceMode::Shared,
            _ => unreachable!(),
        }
        assert!(
            checker
                .mutation_fact_roots(&reports, 0, &[(&origin, point)], Span::default(), 1)
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}
