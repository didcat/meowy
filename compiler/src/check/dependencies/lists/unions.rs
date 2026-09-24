use super::{tests::check, *};

#[test]
pub(crate) fn union_lists_keep_source_slots_across_candidate_selection() {
    for source in [
        "n<uint8>:2;xs<uint8[3]><uint16[3]>:[1,n,3]",
        "f<uint8>:(){->2};xs<uint8[3]><uint16[3]>:[1,f(),3]",
        "xs<uint8[1]><uint16[1]>:[300]",
    ] {
        crate::compile(source).unwrap();
        let checker = check(source);
        let (key, sequence) = checker
            .sequences
            .iter()
            .find(|(key, _)| matches!(key, SequenceSource::Expr(_)))
            .unwrap();
        let SequenceSource::Expr(id) = *key else {
            panic!()
        };
        assert!(sequence.items.iter().all(Option::is_some));
        for pair in sequence.items.windows(2) {
            let (left, right) = (pair[0].unwrap(), pair[1].unwrap());
            assert!(checker.points[left].span.start < checker.points[right].span.start);
            assert!(sequence.edges.contains(&Edge::new(
                Port::Normal(left),
                Port::Entry(right),
                Route::Next
            )));
        }
        assert!(checker.endpoints[key].contains(&Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            Route::Next
        )));
    }
}

#[test]
pub(crate) fn custom_effect_elements_remain_explicit_sequence_gaps_without_replayed_effects() {
    let source = "d:@\"debug\";xs<uint8[3]><int32[3]>:[1,{x:300;d.print(1);->x},2]";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (key, sequence) = checker
        .sequences
        .iter()
        .find(|(key, seq)| matches!(key, SequenceSource::Expr(_)) && seq.items.len() == 3)
        .unwrap();
    assert!(sequence.items[0].is_some());
    assert!(sequence.items[1].is_none());
    assert!(sequence.items[2].is_some());
    assert!(sequence.edges.is_empty());
    let SequenceSource::Expr(id) = *key else {
        panic!()
    };
    assert!(!checker.endpoints[key].contains(&Edge::new(
        Port::Entry(id),
        Port::Operation(id),
        Route::Next
    )));
    assert_eq!(
        checker
            .operations
            .values()
            .filter(|op| op.kind == super::super::OperationKind::Bind)
            .count(),
        2
    );
    let source = "d:@\"debug\";xs<uint8[1]><int32[1]>:[{x:300;d.print(1);->x}]";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (key, sequence) = checker
        .sequences
        .iter()
        .find(|(key, _)| matches!(key, SequenceSource::Expr(_)))
        .unwrap();
    assert_eq!(sequence.items, [None]);
    let SequenceSource::Expr(id) = *key else {
        panic!()
    };
    assert_eq!(
        checker.endpoints[key],
        [Edge::new(
            Port::Operation(id),
            Port::Normal(id),
            Route::Next
        )]
    );
}

#[test]
pub(crate) fn union_list_failures_keep_capacity_ambiguity_and_effect_boundaries() {
    for (source, code) in [
        ("xs<uint8[1]><uint16[1]>:[1,2]", "E103"),
        ("xs<uint8[1]><uint16[1]>:[1]", "E207"),
        ("xs<uint8[2]><string[1]>:[256,1]", "E216"),
        (
            "d:@\"debug\";xs<uint8[2]><uint16[2]>:[{d.print(1);->1},{n<uint8>:2;->n}]",
            "B001",
        ),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
