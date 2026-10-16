use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn argument_slot_forests_preserve_caller_roots_and_shared_candidate_sources() {
    let (mut checker, reports) = checked(
        "f:(a<int32>,b<int32>,c<int32>,d<int32>){};g:(){r:{->n:1};s:{->n:r.n};copy:s.n;f(copy,copy,({->s.n}),2.{->$})};r:{->n:3};s:{->n:r.n};copy:s.n;f(copy,copy,({->s.n}),4.{->$})",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (sources, parts) = checker
        .argument_source_view(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let roots = sources
        .iter()
        .map(|(&point, &(owner, direct))| {
            assert_eq!(direct.point, point);
            assert_eq!(reports.arguments[&point].owner, owner);
            let slot = match (direct.source, direct.block, direct.dispatch) {
                (Some(source), None, None) => source.slot,
                (None, Some(source), None) => source.slot,
                (None, None, Some(source)) => source.slot,
                _ => panic!(),
            };
            Root { owner, slot }
        })
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 8);
    assert_eq!(
        roots
            .iter()
            .map(|root| root.owner)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    let (_, parts) = checker
        .direct_graph(&reports, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .candidate_graph_limited(&reports, Span::default(), parts)
        .unwrap();
    let (walk, _) = graph
        .walk_sources(
            roots.iter().copied(),
            Some(&reports.direct_sources),
            &mut checker.flow,
            Span::default(),
            parts,
        )
        .unwrap();
    assert_eq!(
        walk.visits
            .iter()
            .filter_map(|visit| match visit {
                Visit::Root(root) => Some(*root),
                _ => None,
            })
            .collect::<Vec<_>>(),
        roots
    );
    assert!(
        walk.visits
            .iter()
            .any(|visit| matches!(visit, Visit::Shared(_)))
    );
    assert!(
        walk.visits
            .iter()
            .any(|visit| matches!(visit, Visit::Field(_, _, _)))
    );
    for visit in &walk.visits {
        match visit {
            Visit::Root(root) | Visit::Slot(root) | Visit::Shared(root) => {
                assert_eq!(reports.results[&root.slot.block].0, root.owner)
            }
            Visit::Field(key, input, source) => {
                assert_eq!(reports.candidate_inputs[key].1, *input);
                assert_eq!(reports.direct_sources[key].1.source, Some(*source));
            }
            _ => {}
        }
    }
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_slot_forests_keep_unknown_histories_and_stopped_call_prefixes() {
    let (mut checker, reports) =
        checked("d:@\"debug\";f:(a<int32>,b<int32>,c<int32>){};f({->n:=1}.n,2,d.panic(\"stop\"))");
    let call = checker.invocations.first_key_value().unwrap().1.clone();
    assert!(!reports.effects.contains_key(&call.point));
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (sources, parts) = checker
        .argument_source_view(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(sources.len(), 2);
    let (owner, direct) = sources[&call.args[0]];
    let unknown = sources[&call.args[1]].1;
    assert_eq!(
        (unknown.source, unknown.block, unknown.dispatch),
        (None, None, None)
    );
    assert!(!sources.contains_key(&call.args[2]));
    let root = Root {
        owner,
        slot: direct.source.unwrap().slot,
    };
    let (_, parts) = checker
        .direct_graph(&reports, Span::default(), parts)
        .unwrap();
    let (graph, parts) = checker
        .candidate_graph_limited(&reports, Span::default(), parts)
        .unwrap();
    let (walk, _) = graph
        .walk_sources(
            std::iter::once(root),
            Some(&reports.direct_sources),
            &mut checker.flow,
            Span::default(),
            parts,
        )
        .unwrap();
    assert_eq!(
        walk.visits,
        [Visit::Root(root), Visit::Slot(root), Visit::Unknown(root)]
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn argument_slot_forests_require_candidate_and_direct_graph_qualification() {
    for direct in [false, true] {
        let (mut checker, mut reports) = checked("f:(a<int32>){};r:{->n:1};f(({->r.n}))");
        let point = checker.invocations.first_key_value().unwrap().1.args[0];
        let source = reports.argument_sources[&point].1.block.unwrap();
        let key = *reports
            .candidate_inputs
            .keys()
            .find(|key| key.0 == source.slot.block)
            .unwrap();
        if direct {
            reports.direct_sources.remove(&key);
        } else {
            reports.candidate_inputs.get_mut(&key).unwrap().1.point = usize::MAX;
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let (sources, _) = checker
            .argument_source_view(&reports, Span::default(), MAX_EDGES)
            .unwrap();
        assert_eq!(sources[&point].1.block, Some(source));
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
                .message
                .contains("identity")
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
