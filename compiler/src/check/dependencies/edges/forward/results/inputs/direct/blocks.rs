use super::*;
use crate::check::dependencies::edges::forward::results::{inputs::graph::Visit, tests::checked};

#[test]
pub(crate) fn direct_block_sources_preserve_value_points_fields_and_independent_owners() {
    let (mut checker, reports) = checked(
        "n:{->1};copy<int32>:((n~<int32>));s:{->copy};r:{->n:2};t:{->r.n};f<int32>:(){n:{->3};->{->((n))}}",
    );
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    let mut wrapped = 0;
    let mut fields = 0;
    for (&key, &(owner, direct)) in &reports.direct_sources {
        let input = reports.candidate_inputs[&key].1;
        assert_eq!(direct.point, input.point);
        assert_eq!(input.projection, Projection::Value);
        assert_eq!(input.source, None);
        assert!(direct.source.is_none() || direct.block.is_none());
        fields += usize::from(direct.source.is_some());
        let Some(source) = direct.block else {
            continue;
        };
        assert_eq!(source.slot.index, 0);
        assert_eq!(
            reports.consumers[&source.consumer],
            (owner, source.slot.block)
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
                .contains(&Visit::Value(key, input))
        );
        wrapped += usize::from(direct.point != source.consumer);
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert!(wrapped >= 2);
    assert_eq!(fields, 1);
    assert_eq!(
        checker.direct_sources(&reports, Span::default()).unwrap().0,
        reports.direct_sources
    );
    checker
        .direct_graph(&reports, Span::default(), MAX_EDGES)
        .unwrap();
    assert_eq!(
        checker
            .candidate_walk_report(&reports, Span::default())
            .unwrap()
            .0,
        reports.candidate_walk
    );
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn direct_block_sources_reject_corrupt_stored_descriptors_atomically() {
    for fault in 0..10 {
        let (mut checker, mut reports) = checked("n:{->1};s:{->((n))};r:{->n:2};t:{->r.n}");
        let (&key, &(owner, direct)) = reports
            .direct_sources
            .iter()
            .find(|(_, (_, direct))| direct.block.is_some())
            .unwrap();
        let field = reports
            .direct_sources
            .values()
            .find_map(|(_, direct)| direct.source)
            .unwrap();
        let source = direct.block.unwrap();
        match fault {
            0 => {
                reports
                    .direct_sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .consumer = usize::MAX
            }
            1 => {
                reports
                    .direct_sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .slot
                    .index = 1
            }
            2 => {
                reports
                    .direct_sources
                    .get_mut(&key)
                    .unwrap()
                    .1
                    .block
                    .as_mut()
                    .unwrap()
                    .slot
                    .block = field.slot.block
            }
            3 => reports.direct_sources.get_mut(&key).unwrap().1.block = None,
            4 => reports.direct_sources.get_mut(&key).unwrap().1.point = usize::MAX,
            5 => reports.direct_sources.get_mut(&key).unwrap().1.source = Some(field),
            6 => reports.direct_sources.get_mut(&key).unwrap().0 += 1,
            7 => {
                reports
                    .direct_sources
                    .insert((key.0, key.1, usize::MAX), (owner, direct));
            }
            8 => {
                reports.direct_sources.remove(&key);
            }
            9 => {
                let (_, direct) = reports
                    .direct_sources
                    .values_mut()
                    .find(|(_, direct)| direct.source.is_none() && direct.block.is_none())
                    .unwrap();
                direct.block = Some(source);
            }
            _ => unreachable!(),
        }
        let before = format!("{reports:?}{:?}", checker.edge_counts());
        assert!(
            checker
                .direct_graph(&reports, Span::default(), MAX_EDGES)
                .err()
                .unwrap()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert!(
            checker
                .direct_walk_report(&reports, Span::default())
                .unwrap_err()
                .message
                .contains("identity"),
            "{fault}"
        );
        assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
    }
}
