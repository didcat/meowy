use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

pub(super) fn roots(checker: &Checker, id: usize) -> BTreeSet<usize> {
    checker
        .shaped(id)
        .unwrap()
        .snapshots(&[])
        .flat_map(|value| value.origins.roots.iter().copied())
        .collect()
}

#[test]
pub(crate) fn immutable_named_unions_keep_lexical_copies_and_carrier_origins() {
    for (ty, init, read) in [
        ("&boolean", "&x", "row.copy.r"),
        ("& &boolean", "&a", "*(row.copy.r)"),
    ] {
        let source = format!(
            "<A>:<{{r<{ty}>}}>;<B>:<{{other<boolean>}}>;x:=false;a:&x;row:{{->item<A><B>:{{->r:{init}}};->copy:item}};|row.copy<A>|out:{read}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = checker.locals.len() - 1;
        assert!(checker.pointees[&out].complete);
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        let alias = *checker
            .proofs
            .aliases
            .iter()
            .find(|(_, alias)| alias.field == "item")
            .unwrap()
            .0;
        assert!(checker.derived_local(alias));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn immutable_sibling_union_emissions_merge_without_changing_prior_copies() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;c:=false;row:'out{|c|{'out->item<A><B>:{->r:&x};old:item};|!c|{'out->item<A><B>:{->r:&y};new:item}}";
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
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let old = aliases[0].0 + 1;
    let new = aliases[1].0 + 1;
    assert_eq!(roots(&checker, old), BTreeSet::from([x]));
    assert_eq!(roots(&checker, new), BTreeSet::from([x, y]));
    assert!(
        aliases
            .iter()
            .all(|(id, _)| roots(&checker, *id) == BTreeSet::from([x, y]))
    );
    assert!(!checker.record_shapes.contains_key(&aliases[1].0));
    checker.mark_derived(y);
    assert!(checker.derived_local(aliases[0].0));
    assert!(checker.derived_local(aliases[1].0));
    assert!(!checker.derived_local(old));
    assert!(checker.derived_local(new));
}

#[test]
pub(crate) fn sibling_union_emissions_preserve_null_and_unknown_alternatives() {
    for (init, complete) in [("null", true), ("unknown", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;c:=false;unknown<A>:{{->r:{{->&y}}}};row:'out{{|c|{{'out->item<A><B><null>:{{->r:&x}}}};|!c|{{'out->item<A><B><null>:{init};copy:item}}}}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let alias = *checker
            .proofs
            .aliases
            .iter()
            .find(|(_, alias)| alias.field == "item")
            .unwrap()
            .0;
        let snapshot = checker
            .shaped(alias)
            .unwrap()
            .snapshots(&[])
            .find(|value| !value.origins.roots.is_empty())
            .unwrap();
        assert_eq!(snapshot.origins.complete, complete);
        assert_eq!(snapshot.origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn named_union_marks_control_queries_without_bypassing_ownership() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;seed:false;x:=seed;p:@\"proof\";row:{->item<A><B>:{->r:&x};|item<A>|q:p.can_copy<uint32>()}";
    let checker = crate::check::dependencies::slots::check(source);
    assert!(checker.queries[0].control);
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;row:{->item<A><B>:{->r:&x};copy:item;x=true;|copy<A>|out:copy.r}";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}

#[test]
pub(crate) fn nested_named_shapes_keep_completed_mutable_record_boundaries() {
    for (body, complete) in [
        ("->item:{->inner<A><B>:{->r:&x}};->copy:item.inner", true),
        ("->item<A><B>:={->r:&x};->copy:item", false),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;row:{{{body}}};|row.copy<A>|out:row.copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            if complete {
                BTreeSet::from([id(&checker, "x")])
            } else {
                BTreeSet::new()
            }
        );
    }
}

#[test]
pub(crate) fn sibling_union_carriers_merge_locations_for_later_lexical_reads() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{other<boolean>}>;x:=false;y:=true;a:&x;b:&y;c:=false;row:'out{|c|{'out->item<A><B>:{->r:&a};old:item};|!c|{'out->item<A><B>:{->r:&b};new:item;|new<A>|copied:*(new.r)}}";
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
    let aliases = checker
        .proofs
        .aliases
        .iter()
        .filter(|(_, alias)| alias.field == "item")
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    checker.mark_derived(y);
    assert!(aliases.iter().all(|id| checker.derived_local(*id)));
    assert!(!checker.derived_local(aliases[0] + 1));
}
