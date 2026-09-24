use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn owned_union_inputs_keep_layouts_null_and_unknown_origins() {
    for (init, complete, extra) in [
        ("{->a:&y}", true, true),
        ("{->a:false;->b:&y}", true, true),
        ("null", true, false),
        ("{->a:unknown}", false, false),
    ] {
        let source = format!(
            "<A>:<{{a<&boolean>}}>;<B>:<{{a<boolean>;b<&boolean>}}>;<U>:<A><B><null>;f<&boolean>:(p<U>,q<&boolean>){{->q}};x:=false;y:=true;unknown<&boolean>:{{->&y}};wide<U>:{init};copy:wide;out:f(copy,&x)"
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
pub(crate) fn owned_union_inputs_keep_nested_fields_and_inline_call_wrappers() {
    for call in ["f(wide,&x)", "f(make(&y),&x)", "h({->inner:make(&y)},&x)"] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<W>:<{{inner<U>}}> ;make<U>:(p<&boolean>){{->{{->r:p}}}};f<&boolean>:(p<U>,q<&boolean>){{->q}};h<&boolean>:(p<W>,q<&boolean>){{->q}};x:=false;y:=true;wide<U>:{{->r:&y}};out:{call}"
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
pub(crate) fn owned_union_inputs_preserve_public_contracts_across_union_results() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;make<U>:(p<&boolean>){->{->r:p}};copy<U>:(p<U>){->p};x:=false;row:copy(copy(make(&x)));|row<A>|out:row.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}

#[test]
pub(crate) fn owned_union_input_queries_keep_depth_work_and_no_replay() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;make<U>:(p<&boolean>){->{->r:p}};f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;out:f(make(&x),&x)";
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
    let crate::hir::ExprKind::Call { args, .. } = &value.kind else {
        panic!()
    };
    let crate::hir::Type::Union(members) = &args[0].ty else {
        panic!()
    };
    let large = crate::hir::Type::Union(vec![members[0].clone(); 257]);
    assert_eq!(
        checker
            .call_union_input_origins(&args[0], &large, &[], &value.ty, 0)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_origins(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn owned_union_inputs_preserve_carrier_boundaries_and_lifetimes() {
    let source = "<A>:<{r<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;a:&x;wide<U>:{->r:&a};out:f(wide,&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.pointees[&id(&checker, "out")].complete);
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<U>:<A><B>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true";
    let source = format!("{prefix};wide<U>:{{->r:&y}};out:f(wide,&x);y=false;copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};out:{{z:=false;wide<U>:{{->r:&z}};->f(wide,&x)}};copy:*out");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
