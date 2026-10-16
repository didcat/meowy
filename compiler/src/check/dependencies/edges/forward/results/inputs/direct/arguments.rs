use super::*;
use crate::check::dependencies::edges::forward::entries::tests::checked;

#[test]
pub(crate) fn argument_direct_sources_preserve_existing_field_block_and_dispatch_rules() {
    for (source, expected) in [
        ("f:(a<int32>){};r:{->n:1};f(r.n)", "field"),
        ("f:(a<int32>){};r:{->n:1};f(((r.n~<int32>)))", "field"),
        ("f:(a<int32>){};r:{->n:1};copy:r.n;f(copy)", "field"),
        ("f:(a<int32>){};f(({->1}))", "block"),
        ("f:(a<int32>){};f(1.{->$})", "dispatch"),
        ("f:(a<int32>){};r:{->1;->tag:true};f(((r)))", "unknown"),
        ("f:(a<int32><null>){};r:{->n:1};f(r.n)", "unknown"),
        ("f:(a<int32>){};n:1;p:&n;f(*p)", "unknown"),
        ("f:(a<int32>){};g<int32>:(){->1};f(g())", "unknown"),
        ("f:(a<int32>){};r:{->n:1};copy:=r.n;f(copy)", "unknown"),
        ("f:(a<int32>){};g:(p<int32>){f(p)}", "unknown"),
    ] {
        crate::compile(source).unwrap();
        let (mut checker, program) = checked(source);
        let reports = checker.entry_reports(&program, Span::default()).unwrap();
        let call = checker
            .invocations
            .values()
            .find(|call| call.args.len() == 1)
            .unwrap()
            .clone();
        let point = call.args[0];
        assert!(reports.arguments[&point].normal);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut ctx = Lookup::new(&reports, MAX_EDGES);
        let direct = checker
            .direct_source(&mut ctx, point, call.owner, Span::default())
            .unwrap();
        assert_eq!(direct.point, point);
        let actual = if direct.source.is_some() {
            "field"
        } else if direct.block.is_some() {
            "block"
        } else if direct.dispatch.is_some() {
            "dispatch"
        } else {
            "unknown"
        };
        assert_eq!(actual, expected, "{source}");
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn argument_direct_sources_keep_independent_owners_and_missing_field_evidence() {
    let source = "f:(a<int32>){};g:(){r:{->n:1};f(r.n)};r:{->n:2};f(r.n)";
    let (mut checker, program) = checked(source);
    let mut reports = checker.entry_reports(&program, Span::default()).unwrap();
    let calls = checker.invocations.values().cloned().collect::<Vec<_>>();
    let mut owners = BTreeSet::new();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    for call in &calls {
        let direct = checker
            .direct_source(&mut ctx, call.args[0], call.owner, Span::default())
            .unwrap();
        let source = direct.source.unwrap();
        assert_eq!(
            reports.field_results[&Port::Normal(source.field)].0,
            call.owner
        );
        owners.insert(call.owner);
    }
    assert_eq!(owners.len(), 2);
    reports.field_results.clear();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    for call in calls {
        let direct = checker
            .direct_source(&mut ctx, call.args[0], call.owner, Span::default())
            .unwrap();
        assert!(direct.source.is_none() && direct.block.is_none() && direct.dispatch.is_none());
    }
}
