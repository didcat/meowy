use super::*;
use crate::check::dependencies::{carriers::id, tests::statements};

#[test]
pub(crate) fn composed_union_snapshots_remap_field_names_and_nested_offsets() {
    for tail in [
        "source:{->z<A><B>:{->r:&x}};row:{->a:7;->source};copy:row.z;|copy<A>|out:copy.r",
        "source:{->z:{->inner<A><B>:{->r:&x}}};row:{->a:7;->source};copy:row.z.inner;|copy<A>|out:copy.r",
        "source:{->z<A><B>:{->r:&x}};row:{->a:7;->source};again:{->row};copy:again.z;|copy<A>|out:copy.r",
    ] {
        let source = format!("<A>:<{{r<&boolean>}}>;<B>:<{{r<&int32>}}>;x:=false;{tail}");
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let out = checker.locals.len() - 1;
        let x = id(&checker, "x");
        assert!(checker.pointees[&out].complete, "{tail}");
        assert_eq!(checker.pointees[&out].roots, BTreeSet::from([x]));
        checker.mark_derived(x);
        assert!(checker.derived_local(id(&checker, "row")));
        assert!(checker.derived_local(out));
    }
}

#[test]
pub(crate) fn composed_union_carriers_and_nullable_sources_keep_locations() {
    for (init, empty) in [("{->r:&a}", false), ("null", true)] {
        let source = format!(
            "<A>:<{{r<& &boolean>}}>;<B>:<{{r<& &int32>}}>;x:=false;a:&x;source:{{->z<A><B><null>:{init}}};row:{{->before:false;->source}};copy:row.z;|copy<A>|out:*(copy.r)"
        );
        crate::compile(&source).unwrap();
        let mut checker = Checker::new();
        statements(&mut checker, &source);
        let origins = &checker.pointees[&(checker.locals.len() - 1)];
        assert!(origins.complete);
        assert_eq!(
            origins.roots,
            if empty {
                BTreeSet::new()
            } else {
                BTreeSet::from([id(&checker, "x")])
            }
        );
    }
}

#[test]
pub(crate) fn mixed_composition_alternatives_preserve_unknown_inputs() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;c:=false;unknown<A>:{->r:{->&y}};other:{->z<A><B>:unknown};row:{|c|->z<A><B>:{->r:&x};|!c|->other};copy:row.z;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let origins = &checker.pointees[&(checker.locals.len() - 1)];
    assert!(!origins.complete);
    assert_eq!(origins.roots, BTreeSet::from([id(&checker, "x")]));
}

#[test]
pub(crate) fn composition_keeps_prior_union_snapshots_and_independent_fields() {
    let source = "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;y:=true;origin<A>:={->r:&x};source:{->left<A><B>:origin;->right<A><B>:{->r:&y}};row:{->a:7;->source};origin={->r:&y};copy:row.left;|copy<A>|out:copy.r";
    crate::compile(source).unwrap();
    let mut checker = Checker::new();
    statements(&mut checker, source);
    let out = checker.locals.len() - 1;
    assert_eq!(
        checker.pointees[&out].roots,
        BTreeSet::from([id(&checker, "x")])
    );
    checker.mark_derived(id(&checker, "y"));
    assert!(!checker.derived_local(id(&checker, "copy")));
    assert!(!checker.derived_local(out));
    checker.mark_derived(id(&checker, "x"));
    assert!(checker.derived_local(out));
}

#[test]
pub(crate) fn composition_snapshots_bound_candidates_without_replaying_calls() {
    let source =
        "<A>:<{r<&boolean>}>;<B>:<{r<&int32>}>;x:=false;source:{->z<A><B>:{->r:&x}};row:{->source}";
    let mut checker = Checker::new();
    let stmts = statements(&mut checker, source);
    let Stmt::Bind { value, .. } = stmts.last().unwrap() else {
        panic!()
    };
    let crate::hir::ExprKind::Block(block) = &value.kind else {
        panic!()
    };
    let row = id(&checker, "row");
    let key = checker.record_shapes[&row]
        .entries
        .keys()
        .next()
        .unwrap()
        .clone();
    let temp = checker.record_compositions[&block.id][0];
    let calls = checker.calls;
    checker
        .record_compositions
        .insert(block.id, vec![temp; MAX_FIELDS]);
    assert!(
        checker
            .record_shape_source(value, &key)
            .unwrap()
            .origins
            .complete
    );
    checker
        .record_compositions
        .insert(block.id, vec![temp; MAX_FIELDS + 1]);
    let error = checker.record_shape_source(value, &key).err().unwrap();
    assert_eq!(error.code, "B001");
    assert_eq!(error.span, value.span);
    assert!(error.message.contains("composition budget"));
    assert_eq!(checker.calls, calls);
    assert!(
        checker.record_shapes[&row]
            .snapshots(&[])
            .any(|value| !value.origins.roots.is_empty())
    );
    let source = format!("{source};copy:row.z;x=true;|copy<A>|out:copy.r");
    assert_eq!(crate::compile(&source).unwrap_err()[0].code, "E302");
}

#[test]
pub(crate) fn composed_snapshot_merges_bound_both_origins_and_carrier_locations() {
    let limit = crate::check::dependencies::references::MAX_ROOTS;
    let span = crate::ast::Span::new(1, 2);
    for carriers in [false, true] {
        let mut flow = crate::flow::Flow::new();
        let mut merged = Snapshot::empty();
        let mut next = Snapshot::empty();
        if carriers {
            next.cells.places = (0..limit).map(|id| (id, vec![])).collect();
        } else {
            next.origins.roots = (0..limit).collect();
        }
        merged.merge(next.clone(), &mut flow, span).unwrap();
        merged.merge(next, &mut flow, span).unwrap();
        let mut extra = Snapshot::empty();
        if carriers {
            extra.cells.places.insert((limit, vec![]));
        } else {
            extra.origins.roots.insert(limit);
        }
        let error = merged.merge(extra, &mut flow, span).unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("capacity"));
    }
}
