use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked, results::inputs::graph::Visit,
};

#[test]
pub(crate) fn grouped_field_sources_share_exact_hops_cache_payload_and_work() {
    let depth = 32;
    let source = format!(
        "r:{{->n:1}};s:{{->{}r.n{}}}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let (mut checker, reports) = checked(&source);
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.source.is_some())
        .unwrap()
        .1;
    let expected = direct.source;
    let roots = reports
        .candidate_walk
        .visits
        .iter()
        .filter(|visit| matches!(visit, Visit::Root(_)))
        .count();
    let parts = roots + 1;
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    for (input, hops, parts, accepted) in [
        (direct.point, depth + 1, parts, true),
        (direct.point, depth, parts, false),
        (direct.point, 0, parts, false),
        (expected.unwrap().field, 0, parts, true),
        (direct.point, depth + 1, parts - 1, false),
    ] {
        let mut ctx = Lookup::new(&reports, parts);
        let result = checker.field_narrowing_source(&mut ctx, input, 0, Span::default(), hops);
        if accepted {
            assert_eq!(result.unwrap(), expected);
            assert_eq!(ctx.parts, 0);
        } else {
            assert!(result.unwrap_err().message.contains("budget"));
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
    let mut ctx = Lookup::new(&reports, parts);
    let work = checker.flow.work;
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), depth + 1)
            .unwrap(),
        expected
    );
    let work = checker.flow.work - work;
    assert_eq!(ctx.parts, 0);
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), depth + 1)
            .unwrap(),
        expected
    );
    assert_eq!(ctx.parts, 0);
    assert!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), depth)
            .unwrap_err()
            .message
            .contains("budget")
    );
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.field_narrowing_source(
            &mut Lookup::new(&reports, parts),
            direct.point,
            0,
            Span::default(),
            depth + 1,
        );
        assert_eq!(result.is_ok(), short == 0);
        if let Ok(value) = result {
            assert_eq!(value, expected);
        }
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn grouped_field_sources_bound_registry_even_for_cached_terminal_fields() {
    let (mut checker, reports) = checked("r:{->n:1};s:{->(r.n)}");
    let direct = reports
        .direct_sources
        .values()
        .find(|(_, direct)| direct.source.is_some())
        .unwrap()
        .1;
    let expected = direct.source;
    let group = checker.group_inputs[&direct.point];
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), 2)
            .unwrap(),
        expected
    );
    let parts = ctx.parts;
    for key in 0..MAX_GROUPS - checker.group_inputs.len() {
        checker.group_inputs.insert(usize::MAX - key, group);
    }
    assert_eq!(checker.group_inputs.len(), MAX_GROUPS);
    assert_eq!(
        checker
            .field_narrowing_source(&mut ctx, direct.point, 0, Span::default(), 2)
            .unwrap(),
        expected
    );
    assert_eq!(ctx.parts, parts);
    checker.group_inputs.insert(usize::MAX - MAX_GROUPS, group);
    assert!(
        checker
            .field_narrowing_source(&mut ctx, expected.unwrap().field, 0, Span::default(), 0)
            .unwrap_err()
            .message
            .contains("budget")
    );
    assert_eq!(ctx.parts, parts);
}
