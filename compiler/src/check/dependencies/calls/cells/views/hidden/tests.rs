use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use std::collections::BTreeSet;

#[test]
pub(crate) fn hidden_union_returns_use_variant_keys_for_different_layouts() {
    for (init, names) in [
        ("{->a:&one}", vec!["one"]),
        ("{->a:false;->b:&two}", vec!["two"]),
        ("null", vec![]),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{a<&R>}}>;<B>:<{{a<boolean>;b<&R>}}>;<U>:<A><B><null>;<W>:<{{hidden<U>}}> ;f<&R>:(p<&W>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};pack<W>:{{->hidden:{init}}};view:f(&pack,&one);copy:*view;out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert!(cells.complete, "{init}");
        let mut places = BTreeSet::from([(id(&checker, "one"), vec![])]);
        places.extend(names.iter().map(|name| (id(&checker, name), vec![])));
        assert_eq!(cells.places, places);
        assert!(checker.pointees[&id(&checker, "out")].complete);
        let roots = cells
            .places
            .iter()
            .map(|(root, _)| {
                if *root == id(&checker, "one") {
                    id(&checker, "x")
                } else {
                    id(&checker, "y")
                }
            })
            .collect();
        assert_eq!(checker.pointees[&id(&checker, "out")].roots, roots);
    }
}

#[test]
pub(crate) fn hidden_union_returns_merge_owned_projections_and_unknown_leaves() {
    for (arg, complete) in [("&other", true), ("{->&other}", false)] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<C>:<{{view<&U>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;<R>:<{{item<U>;hidden<V>}}> ;f<&U>:(p<&R>){{copy:p.hidden;|copy<C>|->copy.view;|copy<D>|->p.&item}};x:=false;y:=true;wide<U>:{{->r:&x}};other<U>:{{->r:&y}};pack<R>:{{->item:wide;->hidden:{{->view:{arg}}}}};view:f(&pack);copy:*view;|copy<A>|out:copy.r"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let cells = &checker.reference_cells[&id(&checker, "view")];
        assert_eq!(cells.complete, complete);
        let mut places = BTreeSet::from([(id(&checker, "pack"), vec![1])]);
        if complete {
            places.insert((id(&checker, "other"), vec![]));
        }
        assert_eq!(cells.places, places);
        let out = checker.locals.len() - 1;
        assert_eq!(checker.pointees[&out].complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if complete {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(checker.pointees[&out].roots, roots);
        checker.mark_derived(id(&checker, "x"));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn hidden_union_returns_expand_deeper_candidates_and_nested_calls() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{view<& &R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{hidden<U>}>;f<&R>:(p<&W>,q<&R>){->q};g<&R>:(p<&R>){->p};x:=false;y:=true;one<R>:{->r:&x};two<R>:{->r:&y};link:&two;pack<W>:{->hidden:{->view:&link}};view:g(f(&pack,&one));out:view.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    assert!(checker.reference_cells[&id(&checker, "view")].complete);
    assert_eq!(
        checker.reference_cells[&id(&checker, "view")].places,
        BTreeSet::from([(id(&checker, "one"), vec![]), (id(&checker, "two"), vec![])])
    );
    let crate::hir::Stmt::Bind { value, .. } = &stmts[stmts.len() - 2] else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_cell(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
}

#[test]
pub(crate) fn hidden_union_returns_resolve_borrowed_record_contents() {
    let source = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<A>:<{view<&N>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{hidden<U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;one<R>:{->r:&x};nested<N>:{->view:&one};pack<W>:{->hidden:{->view:&nested}};view:f(&pack,&one)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    assert!(checker.reference_cells[&id(&checker, "view")].complete);
}

#[test]
pub(crate) fn hidden_union_candidate_discovery_bounds_types_and_owned_targets() {
    let source = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{hidden<U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;one<R>:{->r:&x};pack<W>:{->hidden:{->view:&one}};view:f(&pack,&one)";
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let crate::hir::Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let crate::hir::Type::Record { fields, .. } = &checker.locals[id(&checker, "pack")] else {
        panic!()
    };
    let ty = fields[0].ty.clone();
    let paths = checker
        .hidden_union_paths(&ty, &value.ty, value)
        .unwrap()
        .unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].key.fields, vec![0]);
    assert_eq!(paths[0].key.variants.len(), 1);
    let crate::hir::Type::Union(members) = ty else {
        panic!()
    };
    let member = members.iter().find(|ty| ty.has_borrowed()).unwrap();
    let large = crate::hir::Type::Union(vec![member.clone(); 257]);
    assert_eq!(
        checker
            .hidden_union_paths(&large, &value.ty, value)
            .err()
            .unwrap()
            .code,
        "B001"
    );
    let result = crate::hir::Type::Reference(Box::new(value.ty.clone()));
    assert!(
        checker
            .hidden_union_paths(&value.ty, &result, value)
            .unwrap()
            .is_none()
    );
    let mut deep = value.ty.pointee().unwrap().clone();
    for _ in 0..34 {
        deep = crate::hir::Type::Record {
            primary: Box::new(crate::hir::Type::Null),
            fields: vec![crate::hir::Field {
                name: "item".into(),
                mutable: false,
                ty: deep,
            }],
        };
    }
    assert_eq!(
        checker
            .hidden_union_paths(&deep, &value.ty, value)
            .err()
            .unwrap()
            .code,
        "B001"
    );
}

#[test]
pub(crate) fn hidden_union_returns_keep_input_lifetimes() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<W>:<{hidden<U>}>;f<&R>:(p<&W>,q<&R>){->q};x:=false;one<R>:{->r:&x}";
    let source = format!(
        "{prefix};pack<W>:={{->hidden:{{->view:&one}}}};view:f(&pack,&one);pack={{->hidden:{{->view:&one}}}};out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!(
        "{prefix};pack<W>:{{->hidden:{{->view:&one}}}};view:f(&({{->hidden:pack.hidden}}),&one);out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
