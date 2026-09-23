use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use crate::hir::{Expr, ExprKind};

pub(super) fn alias(checker: &Checker, name: &str) -> usize {
    *checker
        .proofs
        .aliases
        .iter()
        .find(|(_, alias)| alias.field == name)
        .unwrap()
        .0
}

pub(super) fn roots(checker: &Checker, id: usize) -> BTreeSet<usize> {
    checker
        .shaped(id)
        .unwrap()
        .snapshots(&[])
        .flat_map(|value| value.origins.roots.iter().copied())
        .collect()
}

#[test]
pub(crate) fn mutable_named_union_retargets_preserve_copies_and_later_marks() {
    for write in [
        "item={->r:&y}",
        "|c|item={->r:&y}",
        "item=item;item={->r:&y}",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;c:=false;row:{{->item<A><B>:={{->r:&x}};->before:item;{write};->after:item}}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let y = id(&checker, "y");
        let before = alias(&checker, "before");
        let after = alias(&checker, "after");
        assert_eq!(roots(&checker, before), BTreeSet::from([x]));
        assert_eq!(roots(&checker, after), BTreeSet::from([x, y]));
        checker.mark_derived(y);
        assert!(!checker.derived_local(before));
        assert!(checker.derived_local(after));
        assert!(checker.derived_local(alias(&checker, "item")));
    }
}

#[test]
pub(crate) fn sibling_mutable_union_aliases_share_retargets_and_keep_prior_copies() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;z:=false;c:=false;row:'out{|c|{'out->item<A><B>:={->r:&x};->before:item;item={->r:&y}};|!c|{'out->item<A><B>:={->r:&z};item=item;->after:item}}";
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
    assert_eq!(aliases[0].1, aliases[1].1);
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let z = id(&checker, "z");
    assert!(
        aliases
            .iter()
            .all(|(id, _)| roots(&checker, *id) == BTreeSet::from([x, y, z]))
    );
    assert!(!checker.record_shapes.contains_key(&aliases[1].0));
    assert_eq!(
        roots(&checker, alias(&checker, "before")),
        BTreeSet::from([x])
    );
    assert_eq!(
        roots(&checker, alias(&checker, "after")),
        BTreeSet::from([x, y, z])
    );
}

#[test]
pub(crate) fn mutable_named_union_carriers_merge_locations_for_lexical_reads() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{other<boolean>}>;x:=false;y:=true;a:&x;b:&y;row:{->item<A><B>:={->r:&a};->before:item;item={->r:&b};->after:item;|after<A>|copy:*(after.r)}";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    assert!(
        checker
            .pointees
            .values()
            .any(|value| value.complete && value.roots == BTreeSet::from([x, y]))
    );
    checker.mark_derived(y);
    assert!(!checker.derived_local(alias(&checker, "before")));
    assert!(checker.derived_local(alias(&checker, "after")));
}

#[test]
pub(crate) fn mutable_named_union_retargets_keep_null_and_unknown_alternatives() {
    for (next, complete) in [("null", true), ("unknown", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;unknown<A>:{{->r:{{->&y}}}};row:{{->item<A><B><null>:={{->r:&x}};item={next};->after:item}}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let after = alias(&checker, "after");
        let snapshot = checker
            .shaped(after)
            .unwrap()
            .snapshots(&[])
            .find(|value| !value.origins.roots.is_empty())
            .unwrap();
        assert_eq!(snapshot.origins.complete, complete);
        assert_eq!(snapshot.origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn mutable_alias_replacement_failures_preserve_canonical_snapshots_and_loans() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;c:=false;next<A><B>:{->r:&y};row:'out{|c|{'out->item<A><B>:={->r:&x}};|!c|{'out->item<A><B>:={->r:&y}}}";
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let ids = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "item")
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    let next = id(&checker, "next");
    let limit = crate::check::dependencies::references::MAX_ROOTS;
    let value = checker
        .record_shapes
        .get_mut(&next)
        .unwrap()
        .entries
        .values_mut()
        .find(|value| !value.origins.roots.is_empty())
        .unwrap();
    value.origins.roots = (1000..1000 + limit).collect();
    let rhs = Expr {
        kind: ExprKind::Local(next),
        ty: checker.locals[next].clone(),
        span: Span::new(1, 2),
    };
    let before = roots(&checker, ids[0]);
    assert_eq!(
        checker
            .track_record_shapes(ids[1], &rhs, true)
            .unwrap_err()
            .code,
        "B001"
    );
    assert_eq!(roots(&checker, ids[0]), before);
    assert_eq!(roots(&checker, ids[1]), before);
    assert!(!checker.record_shapes.contains_key(&ids[1]));
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.track_record_shapes(ids[1], &rhs, true).is_err());
    assert_eq!(roots(&checker, ids[0]), before);
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;row:{->item<A><B>:={->r:&x};old:item;item={->r:&y};x=true;|old<A>|out:old.r}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}

#[test]
pub(crate) fn mutable_named_union_retargets_control_queries_without_tainting_old_copies() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;seed:false;x:=false;y:=seed;p:@\"proof\";row:{->item<A><B>:={->r:&x};->before:item;|before<A>|first:p.can_copy<uint32>();item={->r:&y};|item<A>|second:p.can_copy<uint32>()}";
    let checker = crate::check::dependencies::slots::check(source);
    assert_eq!(checker.queries.len(), 2);
    assert!(!checker.queries[0].control);
    assert!(checker.queries[1].control);
    assert!(!checker.derived_local(alias(&checker, "before")));
    assert!(checker.derived_local(alias(&checker, "item")));
}
