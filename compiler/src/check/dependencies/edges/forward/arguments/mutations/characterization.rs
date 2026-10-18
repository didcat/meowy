use super::*;
use crate::borrow_value::Source;
use crate::check::dependencies::{carriers::id, tests::statements};
use crate::flow::TRUE;
use crate::hir::Stmt;

#[test]
pub(crate) fn call_fact_owners_keep_distinct_field_sources_under_one_storage_root() {
    let source = "f:(a<&!int32>,b<&!int32>){};r:{->a:=1;->b:=2};f(r.&!a,r.&!b)";
    let (mut checker, program, reports, facts) = tests::context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let call = calls.first_key_value().unwrap().1;
    let first = call.inputs[0].1.origins[0];
    let second = call.inputs[1].1.origins[0];
    assert_ne!(first.0.source, second.0.source);
    let root = checker.place_borrows[&first.1].storage;
    assert_eq!(checker.place_borrows[&second.1].storage, root);
    for (origin, point) in [first, second] {
        assert!(
            matches!(&origin.source, Source::Local { id, fields } if *id == root && fields.len() == 1)
        );
        assert_eq!(
            checker.origin_id(checker.place_borrows[&point].place.root),
            root
        );
    }
    checker.mark_derived(root);
    assert!(checker.derived_storage(root));
}

#[test]
pub(crate) fn call_fact_owners_use_canonical_alias_families_without_clearing_marks() {
    let source = "outside:=false;sink:(p<&!boolean>){};f:(flag<boolean>)'out{|flag|{'out->row:{->n:=false};sink(row.&!n)};|!flag|{'out->row:{->n:=true};sink(row.&!n)}}";
    let (mut checker, program, reports, facts) = tests::context(source);
    let (calls, _) = checker
        .call_mutation_targets(&program, &reports, &facts, Span::default(), MAX_EDGES)
        .unwrap();
    let Stmt::Bind { id: outside, .. } = &program.body.stmts[0] else {
        panic!()
    };
    let views = calls
        .values()
        .map(|call| {
            let (origin, point) = call.inputs[0].1.origins[0];
            let Source::Slot { root, view, .. } = origin.source else {
                panic!()
            };
            assert_eq!(root, checker.place_borrows[&point].storage);
            (view, root)
        })
        .collect::<Vec<_>>();
    assert_eq!(views.len(), 2);
    assert_ne!(views[0].0, views[1].0);
    assert_eq!(views[0].1, views[1].1);
    for &(view, root) in &views {
        assert_eq!(checker.origin_id(view), root);
        checker.bools.insert((view, vec!["n".into()]), TRUE);
    }
    checker.bools.insert((*outside, vec![]), TRUE);
    checker.mark_derived(views[0].0);
    let marks = checker.derived.clone();
    checker.forget(views[1].0);
    for (view, _) in views {
        assert!(!checker.bools.contains_key(&(view, vec!["n".into()])));
        assert!(checker.derived_storage(view));
    }
    assert_eq!(checker.bools.get(&(*outside, vec![])), Some(&TRUE));
    assert_eq!(checker.derived, marks);
}

#[test]
pub(crate) fn exclusive_calls_forget_visible_mutable_refinements_but_keep_dependency_marks() {
    for exclusive in [false, true] {
        let mode = if exclusive { "&!" } else { "&" };
        let prefix = format!("f:(p<{mode}boolean>){{}};a:=false;b:=false;copy:false");
        crate::compile(&format!("{prefix};f({mode}a)")).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &prefix);
        let a = id(&checker, "a");
        let b = id(&checker, "b");
        let copy = id(&checker, "copy");
        for local in [a, b, copy] {
            checker.bools.insert((local, vec![]), TRUE);
        }
        checker.mark_derived(a);
        let marks = checker.derived.clone();
        statements(&mut checker, &format!("f({mode}a)"));
        for local in [a, b] {
            assert_eq!(checker.bools.contains_key(&(local, vec![])), !exclusive);
        }
        assert!(checker.bools.contains_key(&(copy, vec![])));
        assert_eq!(checker.derived, marks);
    }
}
