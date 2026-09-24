use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn direct_union_inputs_retain_variant_layouts_and_null() {
    for (init, extra) in [
        ("{->a:&two}", true),
        ("{->a:false;->b:&two}", true),
        ("null", false),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{a<&R>}}>;<B>:<{{a<boolean>;b<&R>}}>;<U>:<A><B><null>;f<&R>:(p<&U>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};wide<U>:{init};view:f(&wide,&one);out:view.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&id(&checker, "out")];
        assert!(origins.complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if extra {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(origins.roots, roots);
    }
}

#[test]
pub(crate) fn direct_union_inputs_preserve_unknown_owners_and_contents() {
    for (content, input) in [("{->&two}", "&wide"), ("&two", "{->&wide}")] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{view<&R>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};wide<U>:{{->view:{content}}};view:f({input},&one);out:view.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert!(!cells.complete);
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "one"), vec![])])
        );
        let origins = &checker.pointees[&id(&checker, "out")];
        assert!(!origins.complete);
        assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    }
}

#[test]
pub(crate) fn union_inputs_retain_deeper_and_stored_candidates() {
    for source in [
        "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<& &U>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};link:&wide;view:f(&link,&row)",
        "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{view<&U>}>;f<&R>:(p<W>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};pack<W>:{->view:&wide};view:f(pack,&row)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert!(cells.complete);
        assert_eq!(
            cells.places,
            BTreeSet::from([(id(&checker, "row"), vec![])])
        );
    }
}

#[test]
pub(crate) fn direct_union_inputs_keep_untraversed_borrowed_contents_incomplete() {
    let source = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<A>:<{view<&N>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){->q};x:=false;row<R>:{->r:&x};nested<N>:{->view:&row};wide<U>:{->view:&nested};view:f(&wide,&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(!checker.reference_cells[&id(&checker, "view")].complete);
}

#[test]
pub(crate) fn direct_union_inputs_preserve_owner_loans_and_temporary_expiry() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){->q};x:=false;row<R>:{->r:&x}";
    let source = format!(
        "{prefix};wide<U>:={{->view:&row}};view:f(&wide,&row);wide={{->view:&row}};out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source =
        format!("{prefix};wide<U>:{{->view:&row}};view:f(&({{->item:wide}}.item),&row);out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}

#[test]
pub(crate) fn deeper_union_inputs_merge_direct_candidates_and_unknown_layers() {
    for (link, complete, extra) in [("&wide", true, true), ("{->&wide}", false, false)] {
        for init in ["{->view:&two}", "null"] {
            let source = format!(
                "<R>:<{{r<&boolean>}}>;<A>:<{{view<&R>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B><null>;f<&R>:(p<& & &U>,q<&U>,r<&R>){{->r}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};wide<U>:{init};plain<U>:null;link:{link};outer:&link;view:f(&outer,&plain,&one);out:view.r"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&id(&checker, "out")];
            assert_eq!(origins.complete, complete);
            let mut roots = BTreeSet::from([id(&checker, "x")]);
            if extra && init != "null" {
                roots.insert(id(&checker, "y"));
            }
            assert_eq!(origins.roots, roots);
        }
    }
}

#[test]
pub(crate) fn deeper_union_inputs_retain_union_and_carrier_results() {
    for (result, field, expr, arg, read) in [
        ("&U", "&U", "&other", "&one", "*view"),
        ("& &U", "& &U", "&link", "&plain", "**view"),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<C>:<{{view<{field}>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;f<{result}>:(p<& &V>,q<{result}>){{->q}};g<{result}>:(p<{result}>){{->p}};x:=false;y:=true;one<U>:{{->r:&x}};other<U>:{{->r:&y}};link:&other;plain:&one;wide<V>:{{->view:{expr}}};outer:&wide;view:g(f(&outer,{arg}));copy:{read};|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
        );
    }
}

#[test]
pub(crate) fn deeper_union_queries_keep_modes_depth_and_no_replay() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<& &U>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};link:&wide;view:f(&link,&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    let union = checker.locals[id(&checker, "wide")].clone();
    let exclusive = crate::hir::Type::Reference(Box::new(crate::hir::Type::Exclusive(Box::new(
        union.clone(),
    ))));
    assert_eq!(
        checker
            .unmatched_union_layers(&exclusive, &value.ty, 1, value)
            .unwrap(),
        None
    );
    for count in [1, 2, 65] {
        let mut ty = union.clone();
        for _ in 0..count {
            ty = crate::hir::Type::Reference(Box::new(ty));
        }
        assert_eq!(
            checker
                .unmatched_union_layers(&ty, &value.ty, 1, value)
                .unwrap(),
            Some(count - 1)
        );
    }
    let mut ty = union;
    for _ in 0..66 {
        ty = crate::hir::Type::Reference(Box::new(ty));
    }
    assert_eq!(
        checker
            .unmatched_union_layers(&ty, &value.ty, 1, value)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn deeper_union_inputs_preserve_live_cell_loans_and_expiry() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<& &U>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row}";
    let source = format!("{prefix};link:=&wide;view:f(&link,&row);link=&wide;out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!("{prefix};view:f(&(&wide),&row);out:view.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
