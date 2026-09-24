use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn union_carrier_inputs_preserve_layouts_null_and_unknown_layers() {
    for (init, link, complete, extra) in [
        ("{->r:&y}", "&b", true, true),
        ("{->a:false;->c:&link}", "&b", true, true),
        ("null", "&b", true, false),
        ("{->a:false;->c:&link}", "{->&b}", false, false),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{a<boolean>;c<& & &boolean>}}>;<U>:<A><B><null>;f<&boolean>:(p<U>,q<&boolean>){{->q}};x:=false;y:=true;b:&y;link:{link};wide<U>:{init};copy:wide;out:f(copy,&x)"
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
pub(crate) fn union_carrier_inputs_keep_nested_fields_and_inline_results() {
    for arg in ["pack", "{->inner:make(&b)}"] {
        let source = format!(
            "<A>:<{{c<& &boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;<W>:<{{inner<U>}}> ;make<U>:(p<& &boolean>){{->{{->c:p}}}};f<&boolean>:(p<W>,q<&boolean>){{->q}};x:=false;y:=true;b:&y;wide<U>:{{->c:&b}};pack<W>:{{->inner:wide}};out:f({arg},&x)"
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
pub(crate) fn union_carrier_inputs_keep_reference_free_record_projections() {
    let source = "<R>:<{flag<boolean>}>;<A>:<{c<& &R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;row<R>:{->flag:true};link:&row;wide<U>:{->c:&link};out:f(wide,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&id(&checker, "out")];
    assert!(origins.complete);
    assert_eq!(
        origins.roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "row")])
    );
}

#[test]
pub(crate) fn union_carrier_input_queries_keep_budgets_and_no_replay() {
    let source = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;make<U>:(p<& &boolean>){->{->c:p}};f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;a:&x;out:f(make(&a),&x)";
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
        checker.reference_origins_at(value, 32).err().unwrap().code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.reference_origins(value).is_err());
}

#[test]
pub(crate) fn union_carrier_inputs_keep_borrowed_terminal_and_lifetime_boundaries() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{c<& &R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;row<R>:{->r:&x};link:&row;wide<U>:{->c:&link};out:f(wide,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "out")].complete);
    let prefix = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true";
    let source =
        format!("{prefix};link:=&y;wide<U>:{{->c:&link}};out:f(wide,&x);link=&x;copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};out:{{link:&y;wide<U>:{{->c:&link}};->f(wide,&x)}};copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
