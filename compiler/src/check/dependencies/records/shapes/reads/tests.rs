use super::*;
use crate::check::dependencies::records::shapes::tests::snapshot;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn narrowed_shape_reads_select_origins_without_reusing_other_variants() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=7;wide<A><B>:{->r:&x}";
    crate::compile(&format!("{source};|wide<A>|out:wide.r")).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let wide = id(&checker, "wide");
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    let shapes = checker.record_shapes.get_mut(&wide).unwrap();
    for (key, value) in &mut shapes.entries {
        let Type::Record { fields, .. } = &key.variants[0].1 else {
            panic!()
        };
        *value = snapshot(if fields[0].ty == Type::Reference(Box::new(Type::Bool)) {
            x
        } else {
            y
        });
    }
    statements(&mut checker, "|wide<A>|out:wide.r");
    let out = checker.locals.len() - 1;
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
    assert!(checker.pointees[&out].complete);
    statements(&mut checker, "|wide<B>|out:wide.r");
    assert_eq!(
        checker.pointees[&(checker.locals.len() - 1)].roots,
        BTreeSet::from([y])
    );
    checker.mark_derived(y);
    assert!(!checker.derived_local(out));
    checker.mark_derived(x);
    assert!(checker.derived_local(out));
}

#[test]
pub(crate) fn narrowed_shape_reads_keep_carriers_and_nested_field_offsets() {
    let source =
        "<A>:<{c<& &boolean>}>;<B>:<{c<& &int32>}>;x:=false;a:&x;row:{->inner<A><B>:{->c:&a}}";
    crate::compile(&format!("{source};|row.inner<A>|out:*(row.inner.c)")).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let row = id(&checker, "row");
    let a = id(&checker, "a");
    let x = id(&checker, "x");
    for (key, value) in &mut checker.record_shapes.get_mut(&row).unwrap().entries {
        assert_eq!(key.fields, [0, 0]);
        assert_eq!(key.variants[0].0, 1);
        *value = snapshot(a);
    }
    statements(&mut checker, "|row.inner<A>|out:*(row.inner.c)");
    let out = checker.locals.len() - 1;
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
    assert!(checker.pointees[&out].complete);
}

#[test]
pub(crate) fn missing_shape_reads_do_not_fall_back_to_positional_metadata() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}",
    );
    let wide = id(&checker, "wide");
    checker.record_shapes.remove(&wide);
    checker
        .record_pointees
        .insert(wide, [(vec![0], snapshot(999).origins)].into());
    statements(&mut checker, "|wide<A>|out:wide.r");
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(!origins.complete && origins.roots.is_empty());
}

#[test]
pub(crate) fn nested_shape_reads_retain_multiple_selections() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<C>:<{i<A><B>}>;<D>:<{other<&boolean>}>;x:=false;wide<C><D>:{->i<A><B>:{->r:&x}}";
    let tail = "|wide<C>|{|wide.i<A>|out:wide.i.r}";
    crate::compile(&format!("{source};{tail}")).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let wide = id(&checker, "wide");
    let x = id(&checker, "x");
    let shapes = checker.record_shapes.get_mut(&wide).unwrap();
    assert!(shapes.entries.keys().any(|key| key.variants.len() == 2));
    for value in shapes.entries.values_mut() {
        *value = snapshot(x);
    }
    statements(&mut checker, tail);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([x]));
}

#[test]
pub(crate) fn shape_reads_bound_traversal_and_keep_ordinary_loan_errors() {
    let mut checker = Checker::new();
    statements(
        &mut checker,
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x}",
    );
    let wide = id(&checker, "wide");
    let ty = checker.record_shapes[&wide]
        .entries
        .keys()
        .next()
        .unwrap()
        .variants[0]
        .1
        .clone();
    let span = crate::ast::Span::new(7, 8);
    let local = Expr {
        kind: ExprKind::Local(wide),
        ty: checker.locals[wide].clone(),
        span,
    };
    let mut value = Expr {
        kind: ExprKind::Coerce {
            value: Box::new(local),
        },
        ty: ty.clone(),
        span,
    };
    assert!(
        checker
            .record_shape_snapshot(&value, &[0])
            .unwrap()
            .is_some()
    );
    for _ in 0..MAX_DEPTH {
        value = Expr {
            kind: ExprKind::Coerce {
                value: Box::new(value),
            },
            ty: ty.clone(),
            span,
        };
    }
    let error = checker.record_shape_snapshot(&value, &[0]).err().unwrap();
    assert_eq!(error.code, "B001");
    assert_eq!(error.span, span);
    assert!(!checker.flow.spend(usize::MAX));
    assert!(checker.record_shape_snapshot(&value, &[]).is_err());
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;wide<A><B>:{->r:&x};x=true;|wide<A>|out:wide.r";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "E302");
}
