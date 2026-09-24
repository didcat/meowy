use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn union_carrier_results_keep_all_candidates_unknowns_and_copies() {
    for (arg, complete) in [("&b", true), ("{->&b}", false)] {
        let source = format!(
            "<A>:<{{c<& &boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;f<U>:(p<& &boolean>,q<& &boolean>){{->{{->c:p}}}};x:=false;y:=true;a:&x;b:&y;row:f(&a,{arg});copy:row;|copy<A>|{{cell:copy.c;out:*cell}}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
        let origins = &checker.pointees[&out];
        assert_eq!(origins.complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn union_carrier_results_keep_stored_deeper_inputs_and_null_contracts() {
    for body in ["{->c:p.cell}", "null"] {
        let source = format!(
            "<A>:<{{c<& & &boolean>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B><null>;<W>:<{{cell<& & &boolean>}}> ;f<U>:(p<W>){{->{body}}};g<& & &boolean>:(p<& & &boolean>){{->p}};x:=false;a:&x;b:&a;pack<W>:{{->cell:g(g(&b))}};row:f(pack);|row<A>|out:**(row.c)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn union_carrier_results_keep_record_and_union_view_contents() {
    for (target, tail) in [("R", "out:copy.r"), ("V", "|copy<R>|out:copy.r")] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<S>:<{{other<boolean>}}>;<V>:<R><S>;<A>:<{{view<&{target}>}}>;<B>:<{{other<int32>}}>;<U>:<A><B>;f<U>:(p<&{target}>){{->{{->view:p}}}};x:=false;data<{target}>:{{->r:&x}};row:f(&data);|row<A>|{{view:row.view;copy:*view;{tail}}}"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn union_carrier_result_queries_keep_independent_completeness_and_budgets() {
    let source = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<U>:(p<& &boolean>){->{->c:p}};x:=false;a:&x;row:f(&a)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let paths = checker.record_origin_paths(value, true).unwrap();
    let path = paths.iter().find(|path| path.supported).unwrap();
    let key =
        super::ShapeKey::new(&path.fields, &path.variants, &mut checker.flow, value.span).unwrap();
    let calls = checker.calls;
    let snapshot = checker.record_shape_source(value, &key).unwrap();
    assert!(snapshot.cells.complete);
    assert!(!snapshot.origins.complete);
    assert_eq!(checker.calls, calls);
    assert_eq!(
        checker
            .record_shape_source_at(value, &key, 32)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.record_shape_source(value, &key).is_err());
}

#[test]
pub(crate) fn union_carrier_results_preserve_live_cell_loans_and_temporary_expiry() {
    let prefix = "<A>:<{c<& &boolean>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<U>:(p<& &boolean>){->{->c:p}};x:=false;y:=true";
    let source = format!("{prefix};a:=&x;row:f(&a);a=&y;|row<A>|out:*(row.c)");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};row:f(&(&x));|row<A>|out:*(row.c)");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
