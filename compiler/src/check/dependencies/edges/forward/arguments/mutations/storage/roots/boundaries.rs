use super::*;
use crate::check::dependencies::edges::forward::arguments::mutations::tests::context;
use crate::flow::TRUE;

#[test]
pub(crate) fn call_fact_root_capacity_allows_guard_merges_without_new_records() {
    let mut checker = Checker::new();
    let mut roots = (0..MAX_ROOTS).map(|root| (root, FALSE)).collect::<Roots>();
    let mut parts = 0;
    checker
        .merge_mutation_root(&mut roots, 0, TRUE, Span::default(), &mut parts)
        .unwrap();
    assert_eq!(roots[&0], TRUE);
    assert_eq!(parts, 0);
    let prior = roots.clone();
    for spare in [0, 1] {
        parts = spare;
        assert!(
            checker
                .merge_mutation_root(&mut roots, MAX_ROOTS, TRUE, Span::default(), &mut parts)
                .unwrap_err()
                .message
                .contains("budget")
        );
        assert_eq!(roots, prior);
        assert_eq!(parts, spare);
    }
}

#[test]
pub(crate) fn call_fact_roots_reject_swapped_fields_and_alias_views_even_with_equal_roots() {
    for source in [
        "f:(a<&!int32>,b<&!int32>){};r:{->a:=1;->b:=2};f(r.&!a,r.&!b)",
        "sink:(p<&!int32>){};f:(flag<boolean>)'out{|flag|{'out->n:=1;sink(&!n)};|!flag|{'out->n:=2;sink(&!n)}}",
    ] {
        let (mut checker, program, reports, facts) = context(source);
        let (calls, _) = checker
            .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
            .unwrap();
        let origins = calls
            .values()
            .flat_map(|call| &call.inputs)
            .flat_map(|(_, target)| &target.origins)
            .copied()
            .collect::<Vec<_>>();
        let root = checker.place_borrows[&origins[0].1].storage;
        assert_eq!(checker.place_borrows[&origins[1].1].storage, root);
        let owner = checker.place_borrows[&origins[0].1].owner;
        let mut origin = origins[0].0.clone();
        origin.source = origins[1].0.source.clone();
        assert!(
            checker
                .mutation_fact_roots(
                    &reports,
                    owner,
                    &[(&origin, origins[0].1)],
                    Span::default(),
                    1
                )
                .unwrap_err()
                .message
                .contains("identity")
        );
    }
}
