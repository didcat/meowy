use super::*;
use crate::check::dependencies::edges::forward::results::tests::checked;

#[test]
pub(crate) fn direct_graph_borrows_qualified_sources_and_preserves_original_reports() {
    let (mut checker, reports) =
        checked("r:{->n:1};s:{->r.n};t:{->(r.n)};u:{->r};f<null>:(){r:{->n:2};s:{->r.n}}");
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let (view, parts) = checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert!(std::ptr::eq(view.graph.results, &reports.results));
    assert!(std::ptr::eq(view.graph.inputs, &reports.candidate_inputs));
    assert!(std::ptr::eq(view.sources, &reports.direct_sources));
    assert!(parts < MAX_EDGES);
    assert_eq!(
        view.sources
            .values()
            .filter_map(|(owner, direct)| direct.source.map(|_| *owner))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1])
    );
    assert!(
        view.sources
            .values()
            .any(|(_, direct)| direct.source.is_none())
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn direct_graph_rejects_missing_extra_changed_and_unqualified_descriptors() {
    for fault in 0..12 {
        let (mut checker, mut reports) = checked("r:{->n:1};a:{->r.n};b:{->r.n};c:{->r}");
        let (&key, &(owner, direct)) = reports
            .direct_sources
            .iter()
            .rev()
            .find(|(_, (_, direct))| direct.source.is_some())
            .unwrap();
        let source = direct.source.unwrap();
        match fault {
            0 => {
                reports.direct_sources.remove(&key);
            }
            1 => {
                reports
                    .direct_sources
                    .insert((key.0, key.1, usize::MAX), (owner, direct));
            }
            2 => reports.direct_sources.get_mut(&key).unwrap().0 += 1,
            3 => reports.direct_sources.get_mut(&key).unwrap().1.point = source.field,
            4 => reports.direct_sources.get_mut(&key).unwrap().1.source = None,
            5 => {
                reports
                    .direct_sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .source
                    .as_mut()
                    .unwrap()
                    .field = direct.point
            }
            6 => {
                reports
                    .direct_sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .source
                    .as_mut()
                    .unwrap()
                    .slot
                    .index = 0
            }
            7 => reports.candidate_inputs.get_mut(&key).unwrap().1.point = usize::MAX,
            8 => {
                reports.field_results.remove(&Port::Normal(source.field));
            }
            9 => {
                let (_, Effect::Narrowing(op)) = reports.effects.get_mut(&direct.point).unwrap()
                else {
                    panic!()
                };
                op.control = !op.control;
            }
            10 => {
                let key = *reports
                    .candidate_inputs
                    .iter()
                    .find(|(_, (_, input))| input.projection != Projection::Value)
                    .unwrap()
                    .0;
                reports.direct_sources.insert(key, (owner, direct));
            }
            11 => {
                let (_, value) = reports
                    .direct_sources
                    .values_mut()
                    .find(|(_, value)| value.source.is_none())
                    .unwrap();
                value.source = Some(source);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        let result = checker.direct_graph(&reports, Span::default(), MAX_EDGES);
        assert!(result.is_err(), "fault {fault}");
        assert!(
            result.err().unwrap().message.contains("identity"),
            "fault {fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}

#[test]
pub(crate) fn direct_graph_charges_shared_qualification_caches_and_exact_work() {
    let (mut checker, reports) = checked("r:{->n:1};a:{->r.n};b:{->r.n};c:{->r}");
    let work = checker.flow.work;
    let (_, left) = checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    let work = checker.flow.work - work;
    let parts = MAX_EDGES - left;
    let (_, expected) = checker
        .direct_sources_limited(&reports, Span::default(), MAX_EDGES, MAX_EDGES)
        .unwrap();
    assert_eq!(left, expected);
    assert_eq!(
        checker
            .direct_graph(&reports, Span::default(), parts)
            .unwrap()
            .1,
        0
    );
    assert!(
        checker
            .direct_graph(&reports, Span::default(), parts - 1)
            .err()
            .unwrap()
            .message
            .contains("budget")
    );
    let before = format!("{reports:?}");
    for short in [0, 1] {
        checker.flow.work = crate::flow::MAX_PROOF_WORK - work + short;
        checker.flow.full = false;
        let result = checker.direct_graph(&reports, Span::default(), parts);
        assert_eq!(result.is_ok(), short == 0);
        assert_eq!(format!("{reports:?}"), before);
    }
}
