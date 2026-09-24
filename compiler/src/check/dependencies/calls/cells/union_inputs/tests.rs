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
pub(crate) fn direct_union_inputs_keep_deeper_and_stored_input_boundaries() {
    for source in [
        "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<& &U>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};link:&wide;view:f(&link,&row)",
        "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{view<&U>}>;f<&R>:(p<W>,q<&R>){->q};x:=false;row<R>:{->r:&x};wide<U>:{->view:&row};pack<W>:{->view:&wide};view:f(pack,&row)",
    ] {
        crate::compile(source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, source);
        assert!(!checker.reference_cells[&id(&checker, "view")].complete);
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
