use super::{super::tests::checked, *};
use std::collections::BTreeSet;

#[test]
pub(crate) fn scalar_block_sources_preserve_terminal_consumers_through_wrappers_and_owners() {
    let (mut checker, reports) =
        checked("n:{->1};copy<int32>:((n~<int32>));f<int32>:(){n:{->2};->{->((n))}}");
    let inputs: Vec<_> = checker
        .operations
        .values()
        .filter_map(|op| op.input.map(|input| (op.owner, input)))
        .chain(
            reports
                .candidate_inputs
                .values()
                .map(|(owner, input)| (*owner, input.point)),
        )
        .collect();
    let before = format!("{reports:?}{:?}", checker.edge_counts());
    let mut owners = BTreeSet::new();
    let mut wrapped = 0;
    for (owner, input) in inputs {
        let Some(source) = checker
            .scalar_block_source(&reports, input, owner, Span::default())
            .unwrap()
        else {
            continue;
        };
        assert_eq!(source.slot.index, 0);
        assert_eq!(
            reports.consumers[&source.consumer],
            (owner, source.slot.block)
        );
        assert_eq!(
            checker.bodies[&source.slot.block].parent,
            Some(source.consumer)
        );
        assert!(matches!(
            checker.bodies[&source.slot.block].completion.result,
            Shape::Scalar(_)
        ));
        assert!(reports.blocks[&source.slot.block].1.result);
        assert_eq!(
            reports.results[&source.slot.block].1.consumer,
            Some(source.consumer)
        );
        wrapped += usize::from(input != source.consumer);
        owners.insert(owner);
    }
    assert_eq!(owners, BTreeSet::from([0, 1]));
    assert!(wrapped >= 2);
    assert_eq!(format!("{reports:?}{:?}", checker.edge_counts()), before);
}

#[test]
pub(crate) fn scalar_block_sources_cover_scalar_kinds_and_keep_record_projection_separate() {
    let (mut checker, reports) =
        checked("a:{};b:{->null};c:{->true};d:{->1};e:{->1.5};f:{->\"x\"};r:{->2;->n:3}");
    let mut count = 0;
    for (&input, &(owner, block)) in &reports.consumers {
        let source = checker
            .scalar_block_source(&reports, input, owner, Span::default())
            .unwrap();
        if matches!(checker.bodies[&block].completion.result, Shape::Scalar(_)) {
            assert_eq!(
                source,
                Some(Source {
                    consumer: input,
                    slot: Slot { block, index: 0 }
                })
            );
            count += 1;
        } else {
            assert_eq!(source, None);
            assert_eq!(
                checker
                    .primary_slot(&reports, input, owner, Span::default())
                    .unwrap(),
                Some(Slot { block, index: 0 })
            );
        }
    }
    assert_eq!(count, 6);
}

#[test]
pub(crate) fn scalar_block_sources_keep_unobserved_stopped_and_nonscalar_inputs_opaque() {
    for source in [
        "x:{->n:1}",
        "flag:=false;x:{|flag|->1}",
        "x:{->[1,2]}",
        "n:1;x:{->&n}",
        "d:@\"debug\";x:{d.panic(\"stop\")}",
        "f<int32>:(){->1};x:f()",
        "r:{->n:1};x:r.n",
        "n:{->1};m:=n;x:m",
    ] {
        let (mut checker, reports) = checked(source);
        let input = checker
            .operations
            .values()
            .rev()
            .find_map(|op| op.input)
            .unwrap();
        assert_eq!(
            checker
                .scalar_block_source(&reports, input, 0, Span::default())
                .unwrap(),
            None,
            "{source}"
        );
    }
    let (mut checker, mut reports) = checked("n:{->1};copy:((n))");
    let input = checker
        .operations
        .values()
        .rev()
        .find_map(|op| op.input)
        .unwrap();
    reports.consumers.clear();
    assert_eq!(
        checker
            .scalar_block_source(&reports, input, 0, Span::default())
            .unwrap(),
        None
    );
}
