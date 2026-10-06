use super::super::graph::Visit;
use super::{super::super::tests::checked, *};

pub(super) const SOURCE: &str = "r:3.{->$;->a:9;->b:true};copy:{->((r))}";

#[test]
pub(crate) fn dispatch_candidate_sources_preserve_composed_slots_across_both_forests() {
    let source =
        "f<{n<int32>}>:(){r:3.{->n:$};->r};r:4.{->$;->a:8};copy:{->r};deep:{->copy};x:{->deep.a}";
    let (mut checker, reports) = checked(source);
    let mut origins = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let mut count = 0;
    for (&key, &(owner, input)) in &reports.candidate_inputs {
        let Some(slot) = input.source else { continue };
        let origin = &reports.results[&slot.block].1;
        origins.insert(origin.dispatch.is_some());
        owners.insert(owner);
        assert_eq!(checker.bodies[&slot.block].owner, owner);
        assert_ne!(slot.block, key.0);
        assert_eq!(
            reports.slot_uses[&Port::Emission(input.candidate.emission)],
            (owner, slot)
        );
        assert_eq!(
            slot.index,
            match input.projection {
                Projection::Primary => 0,
                Projection::Field(index) => index + 1,
                Projection::Value => panic!(),
            }
        );
        for walk in [&reports.candidate_walk, &reports.expanded_walk] {
            assert!(walk.visits.contains(&Visit::Projection(key, input)));
            assert!(!walk.visits.contains(&Visit::Unresolved(key, input)));
        }
        count += 1;
    }
    assert_eq!(count, 6);
    assert_eq!(origins, BTreeSet::from([false, true]));
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert!(
        reports
            .expanded_walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Field(..)))
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    assert_eq!(
        checker
            .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
            .unwrap()
            .0,
        reports.candidate_inputs
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

#[test]
pub(crate) fn dispatch_candidate_sources_keep_missing_links_unresolved_without_selecting_values() {
    let (mut checker, mut reports) = checked(SOURCE);
    let (&key, &(_, input)) = reports
        .candidate_inputs
        .iter()
        .find(|(_, (_, input))| input.source.is_some())
        .unwrap();
    let port = Port::Emission(input.candidate.emission);
    reports.slot_uses.remove(&port);
    let before = format!("{reports:?}");
    let (inputs, _) = checker
        .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(inputs[&key].1.source, None);
    assert_eq!(
        inputs
            .values()
            .filter(|(_, input)| input.source.is_some())
            .count(),
        2
    );
    assert!(checker.candidate_graph(&reports, Span::default()).is_err());
    assert_eq!(format!("{reports:?}"), before);
    reports.candidate_inputs = inputs;
    let unresolved = Visit::Unresolved(key, reports.candidate_inputs[&key].1);
    for walk in [
        checker
            .candidate_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        checker
            .direct_walk_report(&reports, Span::default())
            .unwrap()
            .0,
    ] {
        assert!(walk.visits.contains(&unresolved));
    }
    assert!(!reports.slot_uses.contains_key(&port));
}

#[test]
pub(crate) fn dispatch_candidate_sources_preserve_empty_multiple_and_unknown_histories() {
    for (kind, source) in [
        (0, "r<{}>:3.{};copy:{->r}"),
        (1, "flag:=false;r:flag.{|$|->n:1;|!$|->n:2};copy:{->r}"),
        (
            2,
            "d:@\"debug\";flag:=false;r:0.{->'out{|flag|{'out->{->n:=1};d.panic(\"stop\")};->n:2}};copy:{->r}",
        ),
    ] {
        let (mut checker, reports) = checked(source);
        let slots: Vec<_> = reports
            .candidate_inputs
            .values()
            .filter_map(|(_, input)| input.source)
            .filter(|slot| reports.results[&slot.block].1.dispatch.is_some())
            .collect();
        let sources: Vec<_> = slots
            .iter()
            .map(|slot| &reports.results[&slot.block].1.slots.as_ref().unwrap()[slot.index])
            .collect();
        assert!(!sources.is_empty(), "{source}");
        match kind {
            0 => {
                assert!(sources.iter().all(
                    |source| matches!(source, Sources::Candidates(values) if values.is_empty())
                ))
            }
            1 => {
                assert!(sources.iter().any(
                    |source| matches!(source, Sources::Candidates(values) if values.len() == 2)
                ))
            }
            2 => {
                let (&key, &(_, input)) = reports
                    .candidate_inputs
                    .iter()
                    .find(|(_, (_, input))| {
                        input.source.is_some_and(|source| {
                            matches!(
                                reports.results[&source.block].1.slots.as_ref().unwrap()
                                    [source.index],
                                Sources::Unknown
                            )
                        })
                    })
                    .unwrap();
                let slot = input.source.unwrap();
                let middle = Slot {
                    block: key.0,
                    index: key.1,
                };
                let (&via_key, &(_, via)) = reports
                    .candidate_inputs
                    .iter()
                    .find(|(key, (_, input))| {
                        slots
                            .iter()
                            .any(|slot| (slot.block, slot.index) == (key.0, key.1))
                            && input.source == Some(middle)
                    })
                    .unwrap();
                for walk in [&reports.candidate_walk, &reports.expanded_walk] {
                    assert!(walk.visits.contains(&Visit::Projection(via_key, via)));
                    assert!(walk.visits.contains(&Visit::Projection(key, input)));
                    assert!(
                        walk.visits.iter().any(
                            |visit| matches!(visit, Visit::Unknown(root) if root.slot == slot)
                        )
                    );
                }
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert_eq!(
            checker
                .candidate_inputs_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
                .unwrap()
                .0,
            reports.candidate_inputs
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
