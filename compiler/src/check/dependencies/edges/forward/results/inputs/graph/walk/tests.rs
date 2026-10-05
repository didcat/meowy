use super::{super::super::super::tests::checked, *};

#[test]
pub(crate) fn candidate_walk_follows_checked_compositions_to_terminal_direct_points() {
    let (mut checker, reports) = checked("r:{->n:1};s:{->((r))};t:{->s}");
    let (&key, &(owner, _)) = reports.candidate_inputs.last_key_value().unwrap();
    let root = Root {
        owner,
        slot: Slot {
            block: key.0,
            index: key.1,
        },
    };
    let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
    let (walk, _) = graph
        .walk_roots(
            std::iter::once(root),
            &mut checker.flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(walk.visits[0], Visit::Root(root));
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Projection(_, _)))
            .count(),
        2
    );
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Value(_, _)))
            .count(),
        1
    );
    for visit in &walk.visits {
        if let Visit::Projection(key, input) | Visit::Value(key, input) = visit {
            assert_eq!(reports.candidate_inputs[key], (owner, *input));
        }
    }
}

#[test]
pub(crate) fn candidate_walk_forest_keeps_unknown_empty_unresolved_and_owner_boundaries() {
    let (mut checker, reports) = checked("r:{->n:=1};e<{}>:{};f<{n<int32>}>:(){->n:2};s:{->f()}");
    let visits = &reports.candidate_walk.visits;
    assert!(
        visits
            .iter()
            .any(|visit| matches!(visit, Visit::Unknown(_)))
    );
    assert!(visits.iter().any(|visit| matches!(visit, Visit::Empty(_))));
    assert!(
        visits
            .iter()
            .any(|visit| matches!(visit, Visit::Unresolved(_, _)))
    );
    assert!(
        visits
            .iter()
            .any(|visit| matches!(visit, Visit::Root(root) if root.owner == 1))
    );
    let before = format!("{reports:?}");
    let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
    assert_eq!(
        graph
            .forest(&mut checker.flow, Span::default(), MAX_EDGES)
            .unwrap()
            .0,
        reports.candidate_walk
    );
    assert_eq!(format!("{reports:?}"), before);
}

#[test]
pub(crate) fn candidate_walk_keeps_checked_shared_sources_and_discarded_inputs() {
    for source in [
        "flag:=false;r:{->n:1};s:{|flag|->r;|!flag|->r}",
        "d:@\"debug\";flag:=false;r:'out{|flag|{'out->n:\"gone\";d.panic(\"stop\")};->n:3}",
    ] {
        let (mut checker, reports) = checked(source);
        let (&block, (owner, result)) = reports
            .results
            .iter()
            .find(|(_, (_, result))| {
                result.slots.as_ref().is_some_and(|slots| {
                    slots.iter().any(
                        |source| matches!(source, Sources::Candidates(values) if values.len() == 2),
                    )
                })
            })
            .unwrap();
        let index = result
            .slots
            .as_ref()
            .unwrap()
            .iter()
            .rposition(|source| matches!(source, Sources::Candidates(values) if values.len() == 2))
            .unwrap();
        let root = Root {
            owner: *owner,
            slot: Slot { block, index },
        };
        let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
        let (walk, _) = graph
            .walk_roots(
                std::iter::once(root),
                &mut checker.flow,
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        let shared = source.contains("|flag|->r");
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Shared(_)))
                .count(),
            usize::from(shared)
        );
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Value(_, _)))
                .count(),
            if shared { 1 } else { 2 }
        );
        assert!(
            !walk
                .visits
                .iter()
                .any(|visit| matches!(visit, Visit::Cycle(_)))
        );
    }
}
