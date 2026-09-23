use super::*;
use crate::check::dependencies::records::shapes::tests::snapshot;
use crate::check::dependencies::{carriers::id, tests::statements};
use std::collections::BTreeSet;

#[test]
pub(crate) fn projected_subrecords_preserve_seeded_shape_offsets_and_siblings() {
    let mut checker = Checker::new();
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;row:{->left<A><B>:{->r:&x};->right<A><B>:{->r:&y}}";
    let tail = "copy:row.left;|copy<A>|out:copy.r";
    crate::compile(&format!("{source};{tail}")).unwrap();
    statements(&mut checker, source);
    let row = id(&checker, "row");
    let x = id(&checker, "x");
    let y = id(&checker, "y");
    for (key, value) in &mut checker.record_shapes.get_mut(&row).unwrap().entries {
        *value = snapshot(if key.fields[0] == 0 { x } else { y });
    }
    statements(&mut checker, tail);
    let out = checker.locals.len() - 1;
    assert!(checker.pointees[&out].complete);
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
    checker.mark_derived(y);
    assert!(!checker.derived_local(out));
}

#[test]
pub(crate) fn projected_narrowed_records_retain_outer_and_inner_selections() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;<C>:<{inner<A><B>}>;<D>:<{other<boolean>}>;x:=false;wide<C><D>:{->inner<A><B>:{->r:&x}}";
    let tail = "|wide<C>|{copy:wide.inner;|copy<A>|out:copy.r}";
    crate::compile(&format!("{source};{tail}")).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let wide = id(&checker, "wide");
    let x = id(&checker, "x");
    for value in checker
        .record_shapes
        .get_mut(&wide)
        .unwrap()
        .entries
        .values_mut()
    {
        *value = snapshot(x);
    }
    statements(&mut checker, tail);
    let out = checker.locals.len() - 1;
    assert!(checker.pointees[&out].complete);
    assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
}
