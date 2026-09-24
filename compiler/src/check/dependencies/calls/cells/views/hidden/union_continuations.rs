use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::{Stmt, Type};
use std::collections::BTreeSet;

#[test]
pub(crate) fn union_continuations_preserve_distinct_layouts_null_and_unknowns() {
    for (init, link, complete, extra) in [
        ("{->a:&two}", "&inner", true, true),
        ("{->a:false;->b:&two}", "&inner", true, true),
        ("null", "&inner", true, false),
        ("{->a:unknown}", "&inner", false, false),
        ("{->a:&two}", "{->&inner}", false, false),
    ] {
        let source = format!(
            "<R>:<{{r<&boolean>}}>;<A>:<{{a<&R>}}>;<B>:<{{a<boolean>;b<&R>}}>;<U>:<A><B><null>;<C>:<{{view<&U>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;f<&R>:(p<&V>,q<&R>){{->q}};x:=false;y:=true;one<R>:{{->r:&x}};two<R>:{{->r:&y}};unknown<&R>:{{->&two}};inner<U>:{init};wide<V>:{{->view:{link}}};view:f(&wide,&one);out:view.r"
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
pub(crate) fn union_continuations_keep_deeper_layers_and_all_result_kinds() {
    for (result, arg, hidden, read) in [
        ("&U", "&one", "&two", "*view"),
        ("& &U", "&a", "&b", "**view"),
    ] {
        let source = format!(
            "<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;<U>:<A><B>;<C>:<{{view<{result}>}}>;<D>:<{{other<boolean>}}>;<V>:<C><D>;<E>:<{{view<& &V>}}>;<F>:<{{other<int32>}}>;<W>:<E><F>;f<{result}>:(p<&W>,q<{result}>){{->q}};g<{result}>:(p<{result}>){{->p}};x:=false;y:=true;one<U>:{{->r:&x}};two<U>:{{->r:&y}};a:&one;b:&two;inner<V>:{{->view:{hidden}}};link:&inner;wide<W>:{{->view:&link}};view:g(f(&wide,{arg}));copy:{read};|copy<A>|out:copy.r"
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
pub(crate) fn union_continuation_chains_keep_cumulative_depth_and_no_replay() {
    for count in [3, 12] {
        let mut source =
            String::from("<R>:<{r<&boolean>}>;x:=false;y:=true;one<R>:{->r:&x};two<R>:{->r:&y}");
        let mut target = String::from("R");
        let mut owner = String::from("two");
        for index in 0..count {
            source.push_str(&format!(";<A{index}>:<{{view<&{target}>}}>;<B{index}>:<{{other<boolean>}}>;<U{index}>:<A{index}><B{index}>;u{index}<U{index}>:{{->view:&{owner}}}"));
            target = format!("U{index}");
            owner = format!("u{index}");
        }
        source.push_str(&format!(
            ";f<&R>:(p<&{target}>,q<&R>){{->q}};view:f(&{owner},&one)"
        ));
        if count == 12 {
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "B001");
            assert!(errors[0].message.contains("depth"));
        } else {
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            let stmts = statements(&mut checker, &source);
            let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
                panic!()
            };
            let calls = checker.calls;
            let cells = checker.reference_cell(value).unwrap();
            assert!(cells.complete);
            assert_eq!(
                cells.places,
                BTreeSet::from([(id(&checker, "one"), vec![]), (id(&checker, "two"), vec![])])
            );
            assert_eq!(checker.calls, calls);
            assert!(!checker.flow.spend(usize::MAX));
            assert_eq!(checker.reference_cell(value).err().unwrap().code, "B001");
        }
    }
}

#[test]
pub(crate) fn union_continuations_preserve_unsupported_members_and_exclusive_edges() {
    let mut checker = Checker::new();
    let stmts = statements(
        &mut checker,
        "<R>:<{r<&boolean>}>;x:=false;row<R>:{->r:&x};view:&row",
    );
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let record = checker.locals[id(&checker, "row")].clone();
    let mixed = Type::Reference(Box::new(Type::Union(vec![record.clone(), Type::Bool])));
    assert!(
        checker
            .hidden_union_paths(&mixed, &value.ty, value)
            .unwrap()
            .is_none()
    );
    let exclusive = Type::Reference(Box::new(Type::Exclusive(Box::new(record))));
    assert!(
        checker
            .hidden_union_paths(&exclusive, &value.ty, value)
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn union_continuations_preserve_owner_loans_and_temporary_expiry() {
    let prefix = "<R>:<{r<&boolean>}>;<A>:<{view<&R>}>;<B>:<{other<boolean>}>;<U>:<A><B>;<C>:<{view<&U>}>;<D>:<{other<int32>}>;<V>:<C><D>;f<&R>:(p<&V>,q<&R>){->q};x:=false;row<R>:{->r:&x}";
    let source = format!(
        "{prefix};inner<U>:={{->view:&row}};wide<V>:{{->view:&inner}};view:f(&wide,&row);inner={{->view:&row}};out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!(
        "{prefix};inner<U>:{{->view:&row}};wide<V>:{{->view:&inner}};view:f(&({{->item:wide}}.item),&row);out:view.r"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
