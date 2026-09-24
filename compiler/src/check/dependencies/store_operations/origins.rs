use super::{tests::check, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn stores_snapshot_pointees_before_rhs_retargets_the_reference_cell() {
    let source = "x:=1;y:=2;p:=&!x;*p={p=&!y;->3};*p=4";
    crate::compile(source).unwrap();
    let checker = check(source);
    let ops = checker.stores.values().collect::<Vec<_>>();
    assert_eq!(ops[0].origins.roots, BTreeSet::from([0]));
    assert_eq!(ops[1].origins.roots, BTreeSet::from([0, 1]));
    assert!(ops.iter().all(|op| op.origins.complete));
    assert!(ops.iter().all(|op| !op.origins.roots.contains(&2)));
    assert_eq!(checker.pointees[&2].roots, BTreeSet::from([0, 1]));
}

#[test]
pub(crate) fn store_origins_keep_owner_granularity_reborrows_and_canonical_slot_aliases() {
    for source in [
        "x:={->n:=1};p:&!(x.n);q:&!*p;*q=2",
        "x:=[[1]];p:&!(x[1][1]);*p=2",
        "x:=1;id<&!int32>:(p<&!int32>){->p};*(id(&!x))=2",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let op = checker.stores.values().next().unwrap();
        assert!(op.origins.complete);
        let root = checker.operations.first_key_value().unwrap().1.storage;
        assert_eq!(op.origins.roots, BTreeSet::from([root]), "{source}");
    }
    let source =
        "f:(flag<boolean>)'out{|flag|{'out->n:=1;p:&!n;*p=2};|!flag|{'out->n:=1;p:&!n;*p=3}}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let ops = checker.stores.values().collect::<Vec<_>>();
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0].origins, ops[1].origins);
    let root = *ops[0].origins.roots.first().unwrap();
    assert_eq!(checker.origin_id(root), root);
}

#[test]
pub(crate) fn stores_keep_unknown_origins_and_nonreturning_operands_without_inventing_writes() {
    let checker = check("x:=1;y:=2;p:=&!x;p={->&!y};*p=3");
    let op = checker.stores.values().next().unwrap();
    assert!(!op.origins.complete);
    assert_eq!(op.origins.roots, BTreeSet::from([0]));
    for source in [
        "flag:=false;x:=1;'out{*({|flag|'out.leave();->&!x})=2}",
        "x:=1;p:&!x;'out{*p={'out.leave();->2}}",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        assert_eq!(checker.stores.len(), 1);
        assert_eq!(checker.scope_exits.len(), 1);
        let (&id, op) = checker.stores.first_key_value().unwrap();
        assert_eq!(
            op.edges
                .iter()
                .filter(|edge| edge.to == Port::Operation(id))
                .count(),
            1
        );
        assert!(op.edges.contains(&Edge::new(
            Port::Normal(op.input),
            Port::Operation(id),
            Route::Next
        )));
    }
    let mut checker = Checker::new();
    checker.derived.insert(0);
    let block = crate::parser::parse("seed:true;x:=false;p:={->&!x};*p=seed").unwrap();
    assert!(
        checker
            .block(&block, None, None)
            .unwrap_err()
            .message
            .contains("indirect store origins")
    );
    assert!(checker.stores.is_empty());
    assert_eq!(
        crate::compile("x:=1;p:{->&!x};*p=false").unwrap_err()[0].code,
        "E207"
    );
}

#[test]
pub(crate) fn store_origin_limits_and_invalid_owners_do_not_publish_partial_operations() {
    let mut checker = check("x:=1;p:&!x;*p=2");
    let (&id, op) = checker.stores.first_key_value().unwrap();
    let op = op.clone();
    checker.stores.clear();
    checker.store_edges = 0;
    for (roots, message) in [
        (BTreeSet::from([checker.locals.len()]), "identity"),
        (
            (0..=super::super::references::MAX_ROOTS).collect(),
            "budget",
        ),
    ] {
        let origins = Origins {
            roots,
            complete: false,
        };
        assert!(
            checker
                .store_operation(id, op.target, op.input, origins, op.span)
                .unwrap_err()
                .message
                .contains(message)
        );
        assert!(checker.stores.is_empty());
        assert_eq!(checker.store_edges, 0);
    }
}
