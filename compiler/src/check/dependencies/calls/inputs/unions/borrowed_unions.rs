use crate::check::{
    Checker,
    dependencies::{carriers::id, tests::statements},
};
use crate::hir::Stmt;
use std::collections::BTreeSet;

#[test]
pub(crate) fn borrowed_union_input_contents_keep_layouts_null_and_unknowns() {
    for (init, link, complete, extra) in [
        ("{->a:&y}", "&inner", true, true),
        ("{->a:false;->b:&y}", "&inner", true, true),
        ("null", "&inner", true, false),
        ("{->a:unknown}", "&inner", false, false),
        ("{->a:&y}", "{->&inner}", false, false),
    ] {
        let source = format!(
            "<A>:<{{a<&boolean>}}>;<B>:<{{a<boolean>;b<&boolean>}}>;<V>:<A><B><null>;<C>:<{{view<& &V>}}>;<D>:<{{other<boolean>}}>;<U>:<C><D>;f<&boolean>:(p<U>,q<&boolean>){{->q}};x:=false;y:=true;unknown<&boolean>:{{->&y}};inner<V>:{init};link:{link};wide<U>:{{->view:&link}};out:f(wide,&x)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = id(&checker, "out");
        assert_eq!(checker.pointees[&out].complete, complete);
        let mut roots = BTreeSet::from([id(&checker, "x")]);
        if extra {
            roots.insert(id(&checker, "y"));
        }
        assert_eq!(checker.pointees[&out].roots, roots);
    }
}

#[test]
pub(crate) fn borrowed_union_input_contents_keep_record_transitions_and_inline_calls() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{other<boolean>}>;<V>:<A><B>;<R>:<{inner<V>}>;<C>:<{view<&R>}>;<D>:<{other<int32>}>;<U>:<C><D>;make<U>:(p<&R>){->{->view:p}};f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true;inner<V>:{->r:&y};row<R>:{->inner:inner};out:f(make(&row),&x)";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let out = id(&checker, "out");
    assert!(checker.pointees[&out].complete);
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
    );
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let calls = checker.calls;
    assert!(checker.reference_origins(value).unwrap().complete);
    assert_eq!(checker.calls, calls);
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.reference_origins(value).is_err());
}

#[test]
pub(crate) fn borrowed_union_input_contents_keep_cumulative_depth() {
    for count in [3, 12] {
        let mut source = String::from(
            "x:=false;y:=true;<A0>:<{r<&boolean>}>;<B0>:<{other<boolean>}>;<U0>:<A0><B0>;u0<U0>:{->r:&y}",
        );
        for index in 1..count {
            source.push_str(&format!(";<A{index}>:<{{view<&U{}>}}>;<B{index}>:<{{other<boolean>}}>;<U{index}>:<A{index}><B{index}>;u{index}<U{index}>:{{->view:&u{}}}",index-1,index-1));
        }
        source.push_str(&format!(
            ";f<&boolean>:(p<U{}>,q<&boolean>){{->q}};out:f(u{},&x)",
            count - 1,
            count - 1
        ));
        if count == 12 {
            let errors = crate::compile(&source).unwrap_err();
            assert_eq!(errors[0].code, "B001");
            assert!(errors[0].message.contains("depth"));
        } else {
            crate::compile(&source).unwrap();
            let mut checker = Checker::new();
            statements(&mut checker, &source);
            assert!(checker.pointees[&id(&checker, "out")].complete);
            assert_eq!(
                checker.pointees[&id(&checker, "out")].roots,
                BTreeSet::from([id(&checker, "x"), id(&checker, "y")])
            );
        }
    }
}

#[test]
pub(crate) fn borrowed_union_input_contents_preserve_lifetimes() {
    let prefix = "<A>:<{r<&boolean>}>;<B>:<{other<boolean>}>;<V>:<A><B>;<C>:<{view<&V>}>;<D>:<{other<int32>}>;<U>:<C><D>;f<&boolean>:(p<U>,q<&boolean>){->q};x:=false;y:=true";
    let source = format!(
        "{prefix};inner<V>:={{->r:&y}};wide<U>:{{->view:&inner}};out:f(wide,&x);inner={{->r:&x}};copy:*out"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
    let source = format!(
        "{prefix};out:{{inner<V>:{{->r:&y}};wide<U>:{{->view:&inner}};->f(wide,&x)}};copy:*out"
    );
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E303");
}
