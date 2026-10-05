use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn field_walk_retains_composition_and_original_direct_identity() {
    let (mut checker, reports) = checked("r:{->n:1};s:{->n:r.n};t:{->s}");
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (&key, &(owner, _)) = reports.candidate_inputs.last_key_value().unwrap();
    let root = Root {
        owner,
        slot: Slot {
            block: key.0,
            index: key.1,
        },
    };
    let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
    let before = format!("{reports:?}");
    let (walk, _) = graph
        .walk_sources(
            std::iter::once(root),
            Some(&reports.direct_sources),
            &mut checker.flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Projection(_, _)))
            .count(),
        1
    );
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Field(_, _, _)))
            .count(),
        1
    );
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Value(_, _)))
            .count(),
        1
    );
    for visit in &walk.visits {
        if let Visit::Field(key, input, source) = visit {
            assert_eq!(reports.candidate_inputs[key], (owner, *input));
            assert_eq!(input.projection, Projection::Value);
            assert_eq!(input.source, None);
            assert_eq!(reports.direct_sources[key].1.source, Some(*source));
            assert_ne!(input.point, source.field);
            assert!(
                reports
                    .candidate_walk
                    .visits
                    .contains(&Visit::Value(*key, *input))
            );
        }
    }
    assert_eq!(format!("{reports:?}"), before);
}

#[test]
pub(crate) fn field_walk_keeps_unknown_empty_multiple_and_discarded_histories() {
    for (source, count) in [
        ("s:{->{->n:=1}.n}", None),
        ("r<{n<null>}>:{};s:{->r.n}", Some(0)),
        (
            "flag:=false;r:{|flag|->n:1;|!flag|->n:2};s:{->r.n}",
            Some(2),
        ),
        (
            "d:@\"debug\";flag:=false;r:'out{|flag|{'out->n:\"gone\";d.panic(\"stop\")};->n:3};s:{->r.n}",
            Some(2),
        ),
    ] {
        let (mut checker, reports) = checked(source);
        checker
            .direct_graph(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        let (&key, &(owner, direct)) = reports
            .direct_sources
            .iter()
            .find(|(_, (_, direct))| direct.source.is_some())
            .unwrap();
        let root = Root {
            owner,
            slot: Slot {
                block: key.0,
                index: key.1,
            },
        };
        let target = Root {
            owner,
            slot: direct.source.unwrap().slot,
        };
        let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
        let (walk, _) = graph
            .walk_sources(
                std::iter::once(root),
                Some(&reports.direct_sources),
                &mut checker.flow,
                Span::default(),
                MAX_EDGES,
            )
            .unwrap();
        assert!(walk.visits.contains(&Visit::Slot(target)), "{source}");
        assert_eq!(
            walk.visits
                .iter()
                .filter(|visit| matches!(visit, Visit::Value(_, _)))
                .count(),
            count.unwrap_or(0),
            "{source}"
        );
        if count == Some(0) {
            assert!(walk.visits.contains(&Visit::Empty(target)));
        } else if count.is_none() {
            assert!(walk.visits.contains(&Visit::Unknown(target)));
        }
    }
}

#[test]
pub(crate) fn field_walk_marks_repeated_field_sources_as_shared() {
    let (mut checker, reports) = checked("flag:=false;r:{->n:1};s:{|flag|->r.n;|!flag|->r.n}");
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let (&key, &(owner, direct)) = reports
        .direct_sources
        .iter()
        .find(|(_, (_, direct))| direct.source.is_some())
        .unwrap();
    let root = Root {
        owner,
        slot: Slot {
            block: key.0,
            index: key.1,
        },
    };
    let (graph, _) = checker.candidate_graph(&reports, Span::default()).unwrap();
    let (walk, _) = graph
        .walk_sources(
            std::iter::once(root),
            Some(&reports.direct_sources),
            &mut checker.flow,
            Span::default(),
            MAX_EDGES,
        )
        .unwrap();
    assert_eq!(
        walk.visits
            .iter()
            .filter(|visit| matches!(visit, Visit::Field(_, _, _)))
            .count(),
        2
    );
    assert!(walk.visits.contains(&Visit::Shared(Root {
        owner,
        slot: direct.source.unwrap().slot
    })));
    assert!(
        !walk
            .visits
            .iter()
            .any(|visit| matches!(visit, Visit::Cycle(_)))
    );
}
