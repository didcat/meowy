use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn mutable_union_fields_capture_final_slots_and_keep_prior_copies() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;z:=false;w:=true;row:{->item<A><B>:={->r:&x};item={->r:&y};->other<A><B>:{->r:&z}};old:row;row.item={->r:&w};copy:row.item;prior:old.item;other:row.other;|copy<A>|a:copy.r;|prior<A>|b:prior.r;|other<A>|c:other.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let c = checker.locals.len() - 1;
    let b = c - 1;
    let a = b - 1;
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let z = id(&checker, "z");
    let w = id(&checker, "w");
    assert_eq!(checker.pointees[&a].roots, BTreeSet::from([x, y, w]));
    assert_eq!(checker.pointees[&b].roots, BTreeSet::from([x, y]));
    assert_eq!(checker.pointees[&c].roots, BTreeSet::from([z]));
    assert!(checker.pointees[&a].complete);
    checker.mark_derived(w);
    assert!(checker.derived_local(id(&checker, "row")));
    assert!(!checker.derived_local(id(&checker, "old")));
    assert!(!checker.derived_local(c));
}

#[test]
pub(crate) fn mutable_union_descendants_capture_lexical_writes_and_subrecord_replacements() {
    for tail in [
        "row:{->inner:{->item<A><B>:={->r:&x}};inner.item={->r:&y}}",
        "row:{->inner:={->item<A><B>:={->r:&x}}};row.inner.item={->r:&y}",
        "row:{->inner:={->item<A><B>:={->r:&x}}};row.inner={->item<A><B>:={->r:&y}}",
        "row:{->inner:={->item<A><B>:={->r:&x}}};|condition|row.inner={->item<A><B>:={->r:&y}}",
        "row:{->inner:={->item<A><B>:={->r:&x}}};row.inner=row.inner;row.inner.item={->r:&y}",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;condition:=false;{tail};copy:row.inner.item;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete, "{tail}");
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")]),
            "{tail}"
        );
    }
}

#[test]
pub(crate) fn mutable_union_carrier_fields_retain_final_and_replacement_locations() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{other<boolean>}>;x:=false;y:=true;z:=false;a:&x;b:&y;c:&z;row:{->item<A><B>:={->r:&a};item={->r:&b}};old:row;row.item={->r:&c};copy:row.item;|copy<A>|out:*(copy.r)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y"), id(&checker, "z")])
    );
    checker.mark_derived(id(&checker, "z"));
    assert!(checker.derived_local(id(&checker, "row")));
    assert!(!checker.derived_local(id(&checker, "old")));
}

#[test]
pub(crate) fn mutable_union_field_writes_keep_null_and_unknown_alternatives() {
    for (next, complete) in [("null", true), ("unknown", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;y:=true;unknown<A>:{{->r:{{->&y}}}};row:{{->item<A><B><null>:={{->r:&x}}}};row.item={next};copy:row.item;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn mutable_composed_union_fields_keep_snapshot_independence() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;z:=false;source:{->item<A><B>:={->r:&x}};row:{->a:7;->source};old:row;source.item={->r:&y};row.item={->r:&z};copy:row.item;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let out = checker.locals.len() - 1;
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "z")])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(out));
    checker.mark_derived(id(&checker, "z"));
    assert!(checker.derived_local(out));
    assert!(!checker.derived_local(id(&checker, "old")));
}

#[test]
pub(crate) fn mutable_union_fields_preserve_loan_checks_and_union_interior_write_gates() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;row:{->item<A><B>:={->r:&x}};old:row;row.item={->r:&y};x=true;copy:old.item;|copy<A>|out:copy.r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
    let source = "x:=false;y:=true;left:{->r:=&x};<A>:left<>;<B>:<{n<int32>}>;row:{->item<A><B>:=left};|row.item<A>|row.item.r=&y";
    let errors = crate::compile(source).unwrap_err();
    assert_eq!(errors[0].code, "B001");
    assert!(
        errors[0]
            .message
            .contains("mutable field access requires concrete record storage")
    );
}

#[test]
pub(crate) fn mutable_field_marks_control_queries_without_tainting_prior_copies() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;seed:false;x:=false;y:=seed;p:@\"proof\";row:{->item<A><B>:={->r:&x}};old:row;row.item={->r:&y};copy:old.item;current:row.item;|copy<A>|first:p.can_copy<uint32>();|current<A>|second:p.can_copy<uint32>()";
    let checker = crate::check::dependencies::slots::check(source);
    assert_eq!(checker.queries.len(), 2);
    assert!(!checker.queries[0].control);
    assert!(checker.queries[1].control);
}
