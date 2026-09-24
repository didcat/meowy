use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Stmt, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn direct_union_origin_arguments_preserve_layouts_null_and_unknowns() {
    for (init, arg, complete, extra) in [
        ("{->a:&y}", "&wide", true, true),
        ("{->a:false;->b:&y}", "&wide", true, true),
        ("null", "&wide", true, false),
        ("{->a:unknown}", "&wide", false, false),
        ("{->a:&y}", "{->&wide}", false, false),
    ] {
        let source = format!(
            "<A>:<{{a<&boolean>}}>;<B>:<{{a<boolean>;b<&boolean>}}>;<U>:<A><B><null>;f<&boolean>:(p<&U>,q<&boolean>){{->q}};x:=false;y:=true;unknown<&boolean>:{{->&y}};wide<U>:{init};out:f({arg},&x)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if extra {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn direct_union_origins_keep_deeper_stored_and_nested_views() {
    for call in ["f(&link,&x)", "h(pack,&x)", "k(g(g(&wide)),&x)"] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<W>:<{{view<& &U>}}> ;f<&boolean>:(p<& &U>,q<&boolean>){{->q}};h<&boolean>:(p<W>,q<&boolean>){{->q}};k<&boolean>:(p<&U>,q<&boolean>){{->q}};g<&U>:(p<&U>){{->p}};x:=false;y:=true;wide<U>:{{->r:&y}};link:&wide;pack<W>:{{->view:&link}};out:{call}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = id(&checker, "out");
        assert!(checker.pointees[&out].complete);
        assert_eq!(
            checker.pointees[&out].roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
        );
        checker.mark_derived(id(&checker, "y"));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn direct_union_origins_keep_unknown_intermediate_cells() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<& &U>,q<&boolean>){->q};x:=false;y:=true;wide<U>:{->r:&y};link:{->&wide};out:f(&link,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "out")];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}

#[test]
pub(crate) fn direct_union_origin_queries_keep_call_depth_work_and_modes() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{a<boolean>;r<&boolean>}>;<U>:<A><B>;f<&boolean>:(p<&U>){copy:*p;|copy<A>|->copy.r;|copy<B>|->copy.r};x:=false;wide<U>:{->r:&x};out:f(&wide)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_origins(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker
            .reference_origins_at(value, crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    let ty = Type::Reference(Box::new(Type::Exclusive(Box::new(
        checker.locals[id(&checker, "wide")].clone(),
    ))));
    assert!(checker.call_origin_view(&ty, value).unwrap().is_none());
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.reference_origins(value).is_err());
}

#[test]
pub(crate) fn direct_union_origin_arguments_preserve_owner_loans_and_expiry() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<&U>,q<&boolean>){->q};x:=false;y:=true";
    let source = format!("{prefix};wide<U>:={{->r:&y}};out:f(&wide,&x);wide={{->r:&x}};copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};wide<U>:{{->r:&y}};out:f(&({{->item:wide}}.item),&x);copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
