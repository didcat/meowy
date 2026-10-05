use super::*;
use crate::check::dependencies::{
    TypedKind,
    edges::forward::{consumers::tests::checked, results::inputs::graph::Visit},
};

#[test]
pub(crate) fn ascribed_field_sources_retain_candidates_owners_and_mixed_routes() {
    for source in [
        "r:{->n:1};s:{->r.n~<int32>};f<null>:(){r:{->n:2};s:{->n:r.n~<int32>}}",
        "r:{->n:1};s:{->((r.n~<int32>))};f<null>:(){r:{->n:2};s:{->n:((r.n~<int32>))}}",
        "r:{->n:1};s<int32>:{->((r.n~<int32>)~<int32>)};f<null>:(){r:{->n:2};s:{->n<int32>:((r.n~<int32>)~<int32>)}}",
    ] {
        let (mut checker, reports) = checked(source);
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let mut owners = BTreeSet::new();
        let mut ctx = Lookup::new(&reports, MAX_EDGES);
        for (&key, &(owner, direct)) in &reports.direct_sources {
            let input = reports.candidate_inputs[&key].1;
            let mut current = input.point;
            let mut hops = 0;
            let mut typed = 0;
            let mut control = false;
            loop {
                current = if let Some(op) = checker.typed_ops.get(&current) {
                    typed += 1;
                    assert_eq!(op.kind, TypedKind::Ascription);
                    assert!(!op.changed);
                    assert!(op.normal);
                    control |= op.control;
                    op.input
                } else if let Some(op) = checker.coercions.get(&current) {
                    op.input
                } else if let Some(group) = checker.group_inputs.get(&current) {
                    group.input
                } else if let Some(op) = checker.narrowings.get(&current) {
                    op.input
                } else {
                    break;
                };
                hops += 1;
            }
            if typed == 0 {
                continue;
            }
            let field = direct.source.expect("unchanged ascription field source");
            assert_eq!(current, field.field);
            assert_ne!(direct.point, field.field);
            assert_eq!(direct.point, input.point);
            assert_eq!(input.source, None);
            assert!(!control);
            assert_eq!(
                checker
                    .field_narrowing_source(&mut ctx, input.point, owner, Span::default(), hops)
                    .unwrap(),
                Some(field)
            );
            assert_eq!(
                reports.field_results[&Port::Normal(field.field)],
                (owner, field.slot)
            );
            assert!(
                reports
                    .candidate_walk
                    .visits
                    .contains(&Visit::Value(key, input))
            );
            assert!(
                reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Field(key, input, field))
            );
            assert!(
                !reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Value(key, input))
            );
            assert_eq!(
                checker
                    .grouped_consumer(&reports, input.point, owner, Span::default())
                    .unwrap(),
                None
            );
            owners.insert(owner);
        }
        assert_eq!(owners, BTreeSet::from([0, 1]), "{source}");
        assert_eq!(
            checker.direct_sources(&reports, Span::default()).unwrap().0,
            reports.direct_sources
        );
        assert_eq!(
            checker
                .candidate_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.candidate_walk
        );
        assert_eq!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap()
                .0,
            reports.expanded_walk
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn ascribed_field_sources_preserve_seeded_control_and_separate_histories() {
    for marked in [false, true] {
        let (mut checker, mut reports) = checked(
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{|flag|->((r.n~<int32>));|!flag|->((r.n~<int32>))}",
        );
        for (&id, op) in &mut checker.typed_ops {
            op.control = marked;
            let (_, Effect::Typed(observed)) = reports.effects.get_mut(&id).unwrap() else {
                panic!()
            };
            observed.control = marked;
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let sources: Vec<_> = reports
            .direct_sources
            .iter()
            .filter(|(_, (_, direct))| direct.source.is_some())
            .collect();
        assert_eq!(sources.len(), 2);
        let mut slots = BTreeSet::new();
        let mut points = BTreeSet::new();
        for (&key, &(owner, direct)) in sources {
            let field = direct.source.unwrap();
            let input = reports.candidate_inputs[&key].1;
            let group = checker.group_inputs[&direct.point].input;
            let typed = checker.group_inputs[&group].input;
            assert_eq!(checker.typed_ops[&typed].control, marked);
            assert_eq!(
                checker
                    .field_narrowing_source(
                        &mut Lookup::new(&reports, MAX_EDGES),
                        direct.point,
                        owner,
                        Span::default(),
                        MAX_GROUPS
                    )
                    .unwrap(),
                Some(field)
            );
            assert!(
                reports
                    .expanded_walk
                    .visits
                    .contains(&Visit::Field(key, input, field))
            );
            slots.insert(field.slot);
            points.insert(field.field);
        }
        assert_eq!(slots.len(), 1);
        assert_eq!(points.len(), 2);
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
