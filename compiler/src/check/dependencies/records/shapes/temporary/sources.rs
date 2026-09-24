use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn temporary_union_copies_and_reborrows_retain_external_origins() {
    for copy in [
        "*(&({->item:wide}.item))",
        "*(&*(&({->item:wide}.item)))",
        "*(&(*(&({->item:wide}.item))))",
        "*(&({->outer:{->item:wide}}.outer.item))",
        "(&{->item:wide}).item",
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;wide<A><B>:{{->r:&x}};copy:{copy};|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let x = id(&checker, "x");
        let out = checker.locals.len() - 1;
        assert!(checker.pointees[&out].complete, "{copy}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]), "{copy}");
        assert!(!checker.proofs.temporaries.is_empty());
        assert!(!checker.proofs.temporaries.contains_key(&x));
        checker.mark_derived(x);
        assert!(checker.derived_local(id(&checker, "copy")));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn temporary_union_carriers_distinguish_null_and_unknown_contents() {
    for (init, complete, empty) in [
        ("{->r:&a}", true, false),
        ("null", true, true),
        ("unknown", false, true),
    ] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{other<boolean>}}>;x:=false;a:&x;unknown<A>:{{->r:{{->&a}}}};wide<A><B><null>:{init};copy:*(&({{->item:wide}}.item));|copy<A>|out:*(copy.r)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert_eq!(origins.complete, complete);
        assert_eq!(
            origins.roots,
            if empty {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(&checker, "x")])
            }
        );
    }
}

#[test]
pub(crate) fn temporary_union_copies_keep_snapshots_across_source_replacement() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;wide<A><B>:={->r:&x};copy:*(&({->item:wide}.item));wide={->r:&y};|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let out = checker.locals.len() - 1;
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x")])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(id(&checker, "copy")));
    assert!(!checker.derived_local(out));
}

#[test]
pub(crate) fn temporary_union_tracking_keeps_expiry_separate_from_named_views() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}";
    for tail in [
        "view:&({->item:wide}.item);copy:*view",
        "view:&({->item:wide}.item);copy:*(&*view)",
    ] {
        let errors = crate::compile(&format!("{prefix};{tail}")).unwrap_err();
        assert_eq!(errors[0].code, "E303");
    }
    let source = format!("{prefix};copy:*(&wide);|copy<A>|out:copy.r");
    crate::compile(&source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, &source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}
