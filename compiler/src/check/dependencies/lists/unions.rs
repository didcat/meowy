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
pub(crate) fn custom_effect_elements_retain_exact_roots_without_replayed_effects() {
    let source = "d:@\"debug\";xs<uint8[3]><int32[3]>:[1,{x:300;d.print(1);->x},2]";
    crate::compile(source).unwrap();
    let checker = check(source);
    let (key, sequence) = checker
        .sequences
        .iter()
        .find(|(key, seq)| matches!(key, SequenceSource::Expr(_)) && seq.items.len() == 3)
        .unwrap();
    assert!(sequence.items[0].is_some());
    assert!(sequence.items[1].is_some());
    assert!(sequence.items[2].is_some());
    assert_eq!(sequence.edges.len(), 2);
    let element = sequence.items[1].unwrap();
    let span = checker.points[element].span;
    assert_eq!(&source[span.start..span.end], "{x:300;d.print(1);->x}");
    assert!(
        checker
            .endpoints
            .contains_key(&SequenceSource::Expr(element))
    );
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
    assert_eq!(sequence.items.len(), 1);
    let element = sequence.items[0].unwrap();
    let SequenceSource::Expr(id) = *key else {
        panic!()
    };
    assert_eq!(
        checker.endpoints[key],
        [
            Edge::new(Port::Entry(id), Port::Entry(element), Route::Next),
            Edge::new(Port::Normal(element), Port::Operation(id), Route::Next),
            Edge::new(Port::Operation(id), Port::Normal(id), Route::Next),
        ]
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

#[test]
pub(crate) fn custom_effect_body_roots_connect_checked_prefix_and_emission_statements() {
    let source = "d:@\"debug\";f:(){xs<uint8[1]><int32[1]>:[({x:300;d.print(x);->x})]}";
    crate::compile(source).unwrap();
    let checker = check(source);
    let element = checker
        .sequences
        .iter()
        .find_map(|(key, sequence)| {
            let SequenceSource::Expr(id) = *key else {
                return None;
            };
            source[checker.points[id].span.start..checker.points[id].span.end]
                .starts_with('[')
                .then(|| sequence.items[0].unwrap())
        })
        .unwrap();
    let (&block, body) = checker
        .bodies
        .iter()
        .find(|(_, body)| body.parent == Some(element))
        .unwrap();
    assert_ne!(body.owner, 0);
    assert_eq!(body.owner, checker.points[element].owner);
    let sequence = &checker.sequences[&SequenceSource::Block(block)];
    assert_eq!(sequence.items.len(), 3);
    for point in sequence.items.iter().flatten() {
        let point = &checker.points[*point];
        let site = &checker.sites[&point.site.unwrap()];
        assert!(site.complete);
        assert_eq!(point.block, Some(block));
        assert_eq!(point.parent, Some(element));
    }
    for pair in sequence.items.windows(2) {
        assert!(sequence.edges.contains(&Edge::new(
            Port::Normal(pair[0].unwrap()),
            Port::Entry(pair[1].unwrap()),
            Route::Next
        )));
    }
    assert_eq!(
        checker.endpoints[&SequenceSource::Expr(element)],
        [
            Edge::new(Port::Entry(element), Port::BlockEntry(block), Route::Next),
            Edge::new(
                Port::BlockResult(block),
                Port::Normal(element),
                Route::Result
            ),
        ]
    );
}

#[test]
pub(crate) fn custom_body_graph_budget_failures_do_not_complete_element_roots() {
    let mut checker = Checker::new();
    checker.index_edges = super::super::edges::MAX_EDGES;
    let block = crate::parser::parse("xs<uint8[1]><int32[1]>:[{x:300;->x}]").unwrap();
    let error = checker.block(&block, None, None).unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("budget"));
    assert!(
        checker
            .points
            .iter()
            .filter(|point| point.kind == super::super::PointKind::Expr && point.parent.is_some())
            .any(|point| !point.complete)
    );
    assert!(checker.point.is_none());
}
