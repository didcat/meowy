use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_union_fields_merge_nested_candidates_and_unknowns() {
    for (field, owner, complete) in [
        ("&link", "&pack", true),
        ("{->&link}", "&pack", false),
        ("&link", "{->&pack}", false),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{view<&R>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;<N>:<{{view<& &U>}}>;<W>:<{{inner<N>}}> ;f<&R>:(p<&W>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};wide<U>:{{->view:&two}};link:&wide;pack<W>:{{->inner:{{->view:{field}}}}};view:f({owner},&one);out:view.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn borrowed_union_fields_keep_nullable_owners_and_null_contents() {
    for init in ["null", "{->view:&empty}", "{->view:&wide}"] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{view<&R>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B><null>;<W>:<{{view<&U>}}>;<M>:<W><null>;f<&R>:(p<&M>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};wide<U>:{{->view:&two}};empty<U>:null;pack<M>:{init};view:f(&pack,&one);out:view.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert!(origins.complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if init == "{->view:&wide}" {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn borrowed_union_fields_retain_union_and_carrier_returns() {
    for (result, expr, arg, read) in [
        ("&U", "&two", "&one", "*view"),
        ("& &U", "&b", "&a", "**view"),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<C>:<{{view<{result}>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;<W>:<{{view<& &V>}}> ;f<{result}>:(p<& &W>,q<{result}>){{->q}};g<{result}>:(p<{result}>){{->p}};x:=false;y:=true;one<U>:{{->r:&x}};two<U>:{{->r:&y}};a:&one;b:&two;wide<V>:{{->view:{expr}}};link:&wide;pack<W>:{{->view:&link}};outer:&pack;view:g(f(&outer,{arg}));copy:{read};|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
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
pub(crate) fn borrowed_union_fields_keep_budgets_and_no_replay() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{view<&U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};pack<W>:{->view:&wide};view:f(&pack,&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker
            .reference_cell_at(value, crate::check::dependencies::calls::MAX_DEPTH)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn borrowed_union_fields_preserve_owner_lifetimes_and_unknown_contents() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{view<&U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row}";
    let source = format!(
        "{prefix};pack<W>:={{->view:&wide}};view:f(&pack,&row);pack={{->view:&wide}};out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};view:f(&({{->view:&wide}}),&row);out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
    let source = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<A>:<{view<&N>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{view<&U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;row<R>:{->r:&x};inner<N>:{->view:&row};wide<U>:{->view:&inner};pack<W>:{->view:&wide};view:f(&pack,&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(checker.reference_cells[&id(&checker, "view")].complete);
}
