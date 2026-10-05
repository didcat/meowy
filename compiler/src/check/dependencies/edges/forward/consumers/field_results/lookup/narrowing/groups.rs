use super::*;
use crate::check::dependencies::edges::forward::{
    consumers::tests::checked, results::inputs::graph::Visit,
};

#[test]
pub(crate) fn grouped_field_sources_retain_original_candidates_and_terminal_fields_across_owners() {
    let (mut checker, reports) =
        checked("r:{->n:1};s:{->((r.n))};f<null>:(){r:{->n:2};s:{->((r.n))}}");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    let mut ctx = Lookup::new(&reports, MAX_EDGES);
    for (&key, &(owner, direct)) in &reports.direct_sources {
        let Some(source) = direct.source else {
            continue;
        };
        let input = reports.candidate_inputs[&key].1;
        let mut current = input.point;
        let mut groups = 0;
        let mut narrowings = 0;
        while current != source.field {
            if let Some(group) = checker.group_inputs.get(&current) {
                groups += 1;
                current = group.input;
            } else {
                narrowings += 1;
                current = checker.narrowings[&current].input;
            }
        }
        assert_eq!((groups, narrowings), (2, 1));
        assert_eq!(
            checker
                .field_narrowing_source(&mut ctx, input.point, owner, Span::default(), 3)
                .unwrap(),
            Some(source)
        );
        assert_eq!(direct.point, input.point);
        assert_ne!(input.point, source.field);
        assert_eq!(input.source, None);
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
                .contains(&Visit::Field(key, input, source))
        );
        assert_eq!(
            reports.field_results[&Port::Normal(source.field)],
            (owner, source.slot)
        );
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn grouped_field_sources_require_explicit_groups_and_observed_terminal_routes() {
    for missing in 0..4 {
        let (mut checker, mut reports) = checked("r:{->n:1};s:{->((r.n))}");
        let direct = reports
            .direct_sources
            .values()
            .find(|(_, direct)| direct.source.is_some())
            .unwrap()
            .1;
        let field = direct.source.unwrap().field;
        let narrow = *checker
            .narrowings
            .iter()
            .find(|(_, op)| op.input == field)
            .unwrap()
            .0;
        match missing {
            0 => {
                checker.group_inputs.remove(&direct.point);
            }
            1 => {
                reports.field_results.remove(&Port::Normal(field));
            }
            2 => {
                reports.effects.remove(&narrow);
            }
            3 => {
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&narrow).unwrap() else {
                    panic!()
                };
                op.result = false;
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(checker.region_edges.contains_key(&direct.point));
        assert_eq!(
            checker
                .field_narrowing_source(
                    &mut Lookup::new(&reports, MAX_EDGES),
                    direct.point,
                    0,
                    Span::default(),
                    MAX_GROUPS
                )
                .unwrap(),
            None
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn grouped_field_sources_preserve_other_wrapper_and_logical_boundaries() {
    for source in [
        "r:{->n:1};s:{->((r.n~<int32>))}",
        "r:{->n:1};p:&r;s:{->((p.n))}",
        "f<int32>:(){->1};s:{->((f()))}",
        "r:{->n:1};n:r.n;s:{->((n))}",
        "r:{->n:1};s<int32>:{->((r.n))}",
        "r<{n<int32><null>}>:{->n:1};|r.n<int32>|s:{->((r.n))}",
        "s:{->((false&&true))}",
        "s:{->((true||false))}",
    ] {
        let (_, reports) = checked(source);
        assert!(
            reports
                .direct_sources
                .values()
                .all(|(_, direct)| direct.source.is_none()),
            "{source}"
        );
        assert!(
            !reports
                .expanded_walk
                .visits
                .iter()
                .any(|visit| matches!(visit, Visit::Field(_, _, _))),
            "{source}"
        );
    }
}
