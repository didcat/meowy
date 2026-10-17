use super::*;
use crate::borrow_value::{Projection, Source, Step};
use crate::check::dependencies::edges::forward::entries::tests::checked;
use crate::hir::{Expr, ExprKind, LocalId, Program, Stmt};

pub(super) fn context(source: &str) -> (Checker, Program, Reports, crate::borrow::Facts) {
    crate::compile(source).unwrap();
    let (mut checker, program) = checked(source);
    checker.proofs.conditions = checker.guards.clone();
    let facts = crate::borrow::check(&program, &mut checker.flow, &checker.proofs).unwrap();
    let reports = checker.entry_reports(&program, Span::default()).unwrap();
    (checker, program, reports, facts)
}

pub(super) fn bindings(program: &Program) -> Vec<(LocalId, &Expr)> {
    program
        .body
        .stmts
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Bind { id, value } => Some((*id, value)),
            _ => None,
        })
        .collect()
}

#[test]
pub(crate) fn call_mutation_inputs_are_not_recoverable_from_void_results_or_unknown_forests() {
    for (ty, arg) in [
        ("int32", "1"),
        ("&int32", "&n"),
        ("&!int32", "&!n"),
        ("{p<&int32>}", "{->p:&n}"),
    ] {
        let source = format!("f:(unused<{ty}>){{}};n:=1;f({arg})");
        let (mut checker, _, mut reports, facts) = context(&source);
        let call = checker.invocations.first_key_value().unwrap().1.clone();
        assert!(facts.calls[&call.site].origins.is_empty());
        assert!(facts.calls[&call.site].bounds.is_empty());
        let direct = &reports.argument_sources[&call.args[0]].1;
        assert_eq!(direct.point, call.args[0]);
        assert!(direct.source.is_none());
        assert!(direct.block.is_none() && direct.dispatch.is_none());
        reports.effects.remove(&call.point);
        let (forest, _) = checker
            .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap();
        let (_, mut inputs) = checker
            .call_argument_forest(&forest, call.site, Span::default())
            .unwrap();
        let (param, point, row, marker) = inputs.next().unwrap();
        assert_eq!(
            param,
            reports.signatures.functions[&call.function].params[0]
        );
        let origins = &facts.locals[&param].origins;
        assert_eq!(origins.is_empty(), ty == "int32");
        assert!(
            origins
                .iter()
                .all(|origin| { matches!(origin.source, Source::Input { id, .. } if id == param) })
        );
        assert_eq!(point, call.args[0]);
        assert!(row.unwrap().entry && row.unwrap().normal && marker.is_some());
        assert!(inputs.next().is_none());
    }
}

#[test]
pub(crate) fn call_mutation_inputs_need_projection_paths_beyond_dependency_root_sets() {
    let source = "f:(a<&int32>,b<&int32>){};r:{->a:1;->b:2};p:r.&a;q:r.&b;f(p,q)";
    let (mut checker, program, _, facts) = context(source);
    let values = bindings(&program);
    let root = values[0].0;
    for (index, (id, value)) in values[1..].iter().enumerate() {
        let roots = checker.reference_origins(value).unwrap();
        assert!(roots.complete);
        assert_eq!(roots.roots, std::collections::BTreeSet::from([root]));
        let origins = &facts.locals[id].origins;
        assert_eq!(origins.len(), 1);
        assert_eq!(
            origins[0].source,
            Source::Local {
                id: root,
                fields: vec![Projection::Field(index)]
            }
        );
    }
}

#[test]
pub(crate) fn call_mutation_inputs_distinguish_reference_cells_and_aggregate_components() {
    let source = "f:(p<& &int32>,r<{a<&int32>;b<&int32>}>){};x:1;y:2;p:&x;cell:&p;r:{->a:&x;->b:&y};f(cell,r)";
    let (mut checker, program, _, facts) = context(source);
    let values = bindings(&program);
    let [x, y, p, cell, record] = values.as_slice() else {
        panic!()
    };
    let origins = &facts.locals[&cell.0].origins;
    assert_eq!(origins.len(), 2);
    for (component, id) in [(vec![], p.0), (vec![Step::Deref], x.0)] {
        assert!(origins.iter().any(|origin| {
            origin.component == component && origin.source == Source::Local { id, fields: vec![] }
        }));
    }
    assert_eq!(
        facts.locals[&p.0].origins[0].source,
        Source::Local {
            id: x.0,
            fields: Vec::new()
        }
    );
    let origins = &facts.locals[&record.0].origins;
    assert_eq!(origins.len(), 2);
    for (index, id) in [x.0, y.0].into_iter().enumerate() {
        assert!(origins.iter().any(|origin| {
            origin.component == [Step::Slot(index + 1)]
                && origin.source == Source::Local { id, fields: vec![] }
        }));
    }
    let roots = checker.reference_origins(record.1).unwrap();
    assert!(!roots.complete && roots.roots.is_empty());
}

#[test]
pub(crate) fn call_mutation_inputs_require_snapshots_before_later_argument_writes() {
    let source = "f<&int32>:(p<&int32>,unused<int32>){->p};a:1;b:2;r:=&a;out:f(r,{r=&b;->0})";
    let (mut checker, program, _, facts) = context(source);
    let values = bindings(&program);
    let [a, b, _, output] = values.as_slice() else {
        panic!()
    };
    let ExprKind::Call { site, args, .. } = &output.1.kind else {
        panic!()
    };
    let origins = &facts.calls[site].origins;
    assert_eq!(origins.len(), 1);
    assert_eq!(
        origins[0].source,
        Source::Local {
            id: a.0,
            fields: Vec::new()
        }
    );
    let roots = checker.reference_origins(&args[0]).unwrap();
    assert!(roots.roots.contains(&b.0));
    assert_ne!(roots.roots, std::collections::BTreeSet::from([a.0]));
}

#[test]
pub(crate) fn call_mutation_inputs_keep_stopped_reference_prefixes_separate_from_calls() {
    let source =
        "d:@\"debug\";f:(a<&int32>,b<int32>,c<&!int32>){};x:1;y:=2;f(&x,d.panic(\"stop\"),&!y)";
    let (mut checker, _, reports, facts) = context(source);
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    assert_eq!(facts.calls[&call.site].present, crate::flow::FALSE);
    assert!(!reports.effects.contains_key(&call.point));
    let (forest, _) = checker
        .argument_forest_view(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    let (_, inputs) = checker
        .call_argument_forest(&forest, call.site, Span::default())
        .unwrap();
    for (position, (_, _, row, marker)) in inputs.enumerate() {
        assert_eq!(row.is_some_and(|row| row.entry), position < 2);
        assert_eq!(row.is_some_and(|row| row.normal), position == 0);
        assert_eq!(marker.is_some(), position == 0);
    }
}
