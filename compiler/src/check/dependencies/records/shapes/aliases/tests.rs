use super::*;
use crate::check::dependencies::{
    carriers::id, records::shapes::tests::snapshot, tests::statements,
};
use crate::hir::Type;

#[test]
pub(crate) fn shaped_sibling_aliases_share_canonical_snapshots_but_copies_do_not() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;c:=false;row:'out{|c|{'out->item<A><B>:{->r:&x}};|!c|{'out->item<A><B>:{->r:&y}}}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let aliases = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "item")
        .map(|(id, alias)| (*id, alias.root))
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 2);
    let root = aliases[0].1;
    let sibling = aliases[1].0;
    assert_eq!(root, aliases[1].1);
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    for value in checker
        .record_shapes
        .get_mut(&root)
        .unwrap()
        .entries
        .values_mut()
    {
        *value = snapshot(x);
    }
    let old = checker.local(checker.locals[root].clone());
    checker
        .record_shapes
        .insert(old, checker.record_shapes[&root].clone());
    let mut next = checker.record_shapes[&root].clone();
    for value in next.entries.values_mut() {
        *value = snapshot(y);
    }
    checker.record_shapes.insert(sibling, next);
    checker
        .merge_shape_alias(sibling, root, Span::new(1, 2))
        .unwrap();
    assert!(!checker.record_shapes.contains_key(&sibling));
    for value in checker.shaped(sibling).unwrap().snapshots(&[]) {
        assert!(value.origins.complete && value.cells.complete);
        assert_eq!(value.origins.roots, BTreeSet::from([x, y]));
    }
    checker.mark_derived(y);
    assert!(checker.derived_local(sibling));
    assert!(!checker.derived_local(old));
}

#[test]
pub(crate) fn shaped_alias_merges_keep_missing_keys_incomplete_and_errors_atomic() {
    let mut checker = Checker::new();
    let span = Span::new(3, 4);
    let a = ShapeKey::new(&[0], &[(0, &Type::Bool)], &mut checker.flow, span).unwrap();
    let b = ShapeKey::new(&[0], &[(0, &Type::Null)], &mut checker.flow, span).unwrap();
    let mut prior = Shapes::default();
    prior
        .insert(a.clone(), snapshot(10), &mut checker.flow, span)
        .unwrap();
    let mut next = Shapes::default();
    next.insert(b.clone(), snapshot(11), &mut checker.flow, span)
        .unwrap();
    checker.record_shapes.insert(0, prior);
    checker.record_shapes.insert(1, next);
    checker.merge_shape_alias(1, 0, span).unwrap();
    for (key, root) in [(&a, 10), (&b, 11)] {
        let value = checker.record_shapes[&0].get(key).unwrap();
        assert!(!value.origins.complete && !value.cells.complete);
        assert_eq!(value.origins.roots, BTreeSet::from([root]));
    }
    checker.record_shapes.insert(1, Shapes::default());
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker.merge_shape_alias(1, 0, span).unwrap_err().code,
        "B001"
    );
    assert!(checker.record_shapes.contains_key(&1));
    assert_eq!(checker.record_shapes[&0].entries.len(), 2);
}

#[test]
pub(crate) fn shaped_alias_merges_bound_combined_keys_and_owner_sets() {
    let mut flow = Flow::new();
    let span = Span::new(5, 6);
    let mut prior = Shapes::default();
    for index in 0..MAX_FIELDS {
        let key = ShapeKey::new(&[index], &[(0, &Type::Bool)], &mut flow, span).unwrap();
        prior.insert(key, snapshot(0), &mut flow, span).unwrap();
    }
    assert_eq!(
        prior.merged(&prior, &mut flow, span).unwrap().entries.len(),
        MAX_FIELDS
    );
    let mut next = Shapes::default();
    let key = ShapeKey::new(&[0], &[(0, &Type::Null)], &mut flow, span).unwrap();
    next.insert(key, snapshot(1), &mut flow, span).unwrap();
    assert!(prior.merged(&next, &mut flow, span).is_err());
    let key = prior.entries.keys().next().unwrap().clone();
    let limit = crate::check::dependencies::references::MAX_ROOTS;
    prior.entries.get_mut(&key).unwrap().origins.roots = (0..limit).collect();
    let mut next = Shapes::default();
    next.insert(key.clone(), snapshot(limit), &mut flow, span)
        .unwrap();
    assert!(prior.merged(&next, &mut flow, span).is_err());
    assert_eq!(prior.get(&key).unwrap().origins.roots.len(), limit);
}
