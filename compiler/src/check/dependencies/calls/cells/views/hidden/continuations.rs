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
            assert!(!checker.reference_cells[&id(&checker, "view")].complete);
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
    assert!(!checker.reference_cell(value).unwrap().complete);
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
