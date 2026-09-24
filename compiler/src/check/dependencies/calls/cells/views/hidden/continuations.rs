use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Stmt, Type};

#[test]
pub(crate) fn hidden_discovery_retains_typed_shared_record_continuations() {
    for (field, expr, layers) in [
        ("&N", "&nested", 0),
        ("& &N", "&link", 1),
        ("& & &N", "&outer", 2),
    ] {
        for (target, init) in [("N", "{->view:&row}"), ("M", "null")] {
            let field = field.replace('N', target);
            let source = format!(
                "<R>:<{{r<&boolean>}}>;<N>:<{{view<&R>}}>;<M>:<N><null>;<A>:<{{view<{field}>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){{->q}};x:=false;row<R>:{{->r:&x}};nested<{target}>:{init};link:&nested;outer:&link;wide<U>:{{->view:{expr}}};view:f(&wide,&row)"
            );
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            let stmts = statements(&mut checker, &source);
            let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
                panic!()
            };
            let ty = checker.locals[id(&checker, "wide")].clone();
            let expected =
                Type::Reference(Box::new(checker.locals[id(&checker, "nested")].clone()));
            let paths = checker
                .hidden_union_paths(&ty, &value.ty, value)
                .unwrap()
                .unwrap();
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].layers, layers);
            assert_eq!(paths[0].view, Some(&expected));
            assert_eq!(paths[0].key.fields, vec![0]);
            assert_eq!(paths[0].key.variants.len(), 1);
            assert!(checker.reference_cells[&id(&checker, "view")].complete);
        }
    }
}

#[test]
pub(crate) fn hidden_discovery_keeps_terminal_and_continuation_keys_distinct() {
    let source = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<A>:<{view<&N>}>;<B>:<{a<boolean>;view<&R>}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){->q};x:=false;row<R>:{->r:&x};nested<N>:{->view:&row};wide<U>:{->view:&nested};view:f(&wide,&row)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let ty = checker.locals[id(&checker, "wide")].clone();
    let calls = checker.calls;
    let paths = checker
        .hidden_union_paths(&ty, &value.ty, value)
        .unwrap()
        .unwrap();
    assert_eq!(checker.calls, calls);
    assert_eq!(paths.len(), 2);
    let nested = paths.iter().find(|path| path.view.is_some()).unwrap();
    let terminal = paths.iter().find(|path| path.view.is_none()).unwrap();
    assert_eq!(nested.key.fields, vec![0]);
    assert_eq!(terminal.key.fields, vec![1]);
    assert!(nested.key.variants != terminal.key.variants);
    assert!(checker.reference_cell(value).unwrap().complete);
    assert!(!checker.flow.spend(usize::MAX));
    assert_eq!(
        checker
            .hidden_union_paths(&ty, &value.ty, value)
            .err()
            .unwrap()
            .code,
        "B001"
    );
}

#[test]
pub(crate) fn hidden_continuations_do_not_cross_exclusive_edges_or_owned_targets() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;x:=false;row<R>:{->r:&x};nested<N>:{->view:&row};view:&row",
    );
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let nested = checker.locals[id(&checker, "nested")].clone();
    let ty = Type::Reference(Box::new(Type::Exclusive(Box::new(nested))));
    assert!(
        checker
            .hidden_union_paths(&ty, &value.ty, value)
            .unwrap()
            .is_none()
    );
    let result = Type::Reference(Box::new(value.ty.clone()));
    assert!(
        checker
            .hidden_union_paths(&value.ty, &result, value)
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn continuation_entry_points_share_the_cumulative_depth_limit() {
    use crate::check::dependencies::Cells;
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;x:=false;row<R>:{->r:&x};view:&row",
    );
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let ty = value.ty.pointee().unwrap();
    assert!(
        checker
            .returned_record_cells_at(Cells::default(), &value.ty, value, None, &value.ty, 31)
            .is_ok()
    );
    let error = checker
        .returned_record_cells_at(Cells::default(), &value.ty, value, None, &value.ty, 32)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    let error = checker
        .hidden_union_cells_at(&Cells::default(), &[], ty, &value.ty, value, 33)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation depth"));
    let error = checker
        .union_location_cells_at(Cells::default(), &value.ty, &value.ty, 0, value, 33)
        .err()
        .unwrap();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("continuation depth"));
}

#[test]
pub(crate) fn resolved_continuations_merge_known_and_unknown_record_contents() {
    use std::collections::BTreeSet;
    for (inner, link, complete) in [
        ("&two", "&nested", true),
        ("{->&two}", "&nested", false),
        ("&two", "{->&nested}", false),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<N>:<{{view<&R>}}>;<A>:<{{view<&N>}}>;<B>:<{{other<boolean>}}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};nested<N>:{{->view:{inner}}};wide<U>:{{->view:{link}}};view:f(&wide,&one);out:view.r"
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
pub(crate) fn resolved_continuations_keep_union_and_carrier_results() {
    use std::collections::BTreeSet;
    for (result, arg, hidden, read) in [
        ("&U", "&one", "&two", "*view"),
        ("& &U", "&a", "&b", "**view"),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<N>:<{{view<{result}>}}>;<C>:<{{nested<& &N>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;f<{result}>:(p<&V>,q<{result}>){{->q}};x:=false;y:=true;one<U>:{{->r:&x}};two<U>:{{->r:&y}};a:&one;b:&two;nested<N>:{{->view:{hidden}}};link:&nested;wide<V>:{{->nested:&link}};view:f(&wide,{arg});copy:{read};|copy<A>|out:copy.r"
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
pub(crate) fn nested_union_record_transitions_share_depth_and_work_limits() {
    use std::collections::BTreeSet;
    for count in [2, 12] {
        let mut source =
            String::from("<R>:<{r<&boolean>}>;x:=false;y:=true;one<R>:{->r:&x};two<R>:{->r:&y}");
        let mut target = String::from("R");
        let mut owner = String::from("two");
        for index in 0..count {
            source.push_str(&format!(";<N{index}>:<{{view<&{target}>}}>;<A{index}>:<{{view<&N{index}>}}>;<B{index}>:<{{other<boolean>}}>;<U{index}>:<A{index}><B{index}>;n{index}<N{index}>:{{->view:&{owner}}};u{index}<U{index}>:{{->view:&n{index}}}"));
            target = format!("U{index}");
            owner = format!("u{index}");
        }
        source.push_str(&format!(
            ";f<&R>:(p<&{target}>,q<&R>){{->q}};view:f(&{owner},&one);out:view.r"
        ));
        if count == 12 {
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "B001");
            assert!(errors[0].message.contains("depth"));
        } else {
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            let origins = &checker.pointees[&id(&checker, "out")];
            assert!(origins.complete);
            assert_eq!(
                origins.roots,
                BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
            );
        }
    }
}

#[test]
pub(crate) fn resolved_continuations_preserve_nullable_contents_and_lifetimes() {
    use std::collections::BTreeSet;
    let prefix = "<R>:<{r<&boolean>}>;<N>:<{view<&R>}>;<M>:<N><null>;<A>:<{view<&M>}>;<B>:<{other<boolean>}>;<U>:<A><B>;f<&R>:(p<&U>,q<&R>){->q};x:=false;row<R>:{->r:&x}";
    let source =
        format!("{prefix};nested<M>:null;wide<U>:{{->view:&nested}};view:f(&wide,&row);out:view.r");
    crate::compile(&source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, &source);
    let origins = &checker.pointees[&id(&checker, "out")];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
    let source = format!(
        "{prefix};nested<M>:={{->view:&row}};wide<U>:{{->view:&nested}};view:f(&wide,&row);nested=null;out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!(
        "{prefix};nested<M>:{{->view:&row}};wide<U>:{{->view:&nested}};view:f(&({{->item:wide}}.item),&row);out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
