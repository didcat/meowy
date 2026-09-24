use super::*;

pub(crate) fn check(source: &str) -> (Checker, hir::Block) {
    let mut checker = Checker::new();
    let block = crate::parser::parse(source).unwrap();
    let body = checker.block(&block, None, None).unwrap();
    (checker, body)
}

#[test]
pub(crate) fn block_sequences_retain_erased_statements_and_exact_source_order() {
    let source = "p:@\"proof\";n:3;<T>:{-><uint8[n]>};q:p.can_copy<uint8>();after:1";
    let (checker, body) = check(source);
    let sequence = &checker.sequences[&Source::Block(body.id)];
    assert_eq!(sequence.items.len(), 5);
    assert_eq!(body.stmts.len(), 2);
    assert_eq!(sequence.edges.len(), 4);
    for (pair, edge) in sequence.items.windows(2).zip(&sequence.edges) {
        assert_eq!(
            *edge,
            Edge::new(
                Port::Normal(pair[0].unwrap()),
                Port::Entry(pair[1].unwrap()),
                Route::Next
            )
        );
    }
    assert_eq!(
        checker.points[checker.queries[0].point].parent,
        sequence.items[3]
    );
    let read = &checker.body_inputs[&body.id][0];
    assert_eq!(checker.points[read.point].parent, sequence.items[2]);
    assert_eq!(checker.sequence_edges, 4);
}

#[test]
pub(crate) fn block_sequences_keep_forward_barriers_nested_targets_and_functions_explicit() {
    let source = "a:1;f<()->int32>;f<int32>:(){->1};b:2;c:3";
    crate::compile(source).unwrap();
    let (checker, body) = check(source);
    let sequence = &checker.sequences[&Source::Block(body.id)];
    assert_eq!(sequence.items.len(), 4);
    assert!(sequence.items[1].is_none());
    assert_eq!(
        sequence.edges,
        [Edge::new(
            Port::Normal(sequence.items[2].unwrap()),
            Port::Entry(sequence.items[3].unwrap()),
            Route::Next
        )]
    );
    let (checker, _) =
        check("flag:=false;'outer{before:1;|flag|'outer.restart();after:2};f:(){a:1;b:2}");
    assert_eq!(checker.sequences.len(), 3);
    for (source, sequence) in &checker.sequences {
        let Source::Block(block) = source else {
            continue;
        };
        for id in sequence.items.iter().flatten() {
            assert_eq!(checker.points[*id].owner, sequence.owner);
            assert_eq!(checker.points[*id].block, Some(*block));
            assert_eq!(
                checker.sites[&checker.points[*id].site.unwrap()].point,
                Some(*id)
            );
        }
    }
}

#[test]
pub(crate) fn block_sequences_reject_duplicate_invalid_and_changed_identities_atomically() {
    for case in 0..4 {
        let (mut checker, body) = check("a:1;b:2");
        let source = Source::Block(body.id);
        let prior = checker.sequences[&source].items.clone();
        let count = checker.sequence_edges;
        checker
            .sequence(source, prior.clone(), Span::default())
            .unwrap();
        assert_eq!(checker.sequence_edges, count);
        let mut items = prior.clone();
        match case {
            0 => items[1] = items[0],
            1 => items[0] = Some(usize::MAX),
            2 => items.swap(0, 1),
            3 => checker.points[items[0].unwrap()].site = None,
            _ => unreachable!(),
        }
        let error = checker
            .sequence(source, items, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(checker.sequences[&source].items, prior);
        assert_eq!(checker.sequence_edges, count);
    }
}

#[test]
pub(crate) fn block_sequences_obey_work_item_and_shared_edge_limits() {
    for case in 0..3 {
        let (mut checker, body) = check("a:1;b:2");
        let source = Source::Block(body.id);
        let sequence = checker.sequences.remove(&source).unwrap();
        checker.sequence_edges -= sequence.edges.len();
        let mut items = sequence.items;
        match case {
            0 => {
                assert!(!checker.flow.spend(usize::MAX));
            }
            1 => checker.sequence_edges = super::super::edges::MAX_EDGES,
            2 => items = vec![None; MAX_ITEMS + 1],
            _ => unreachable!(),
        }
        let count = checker.sequence_edges;
        let error = checker
            .sequence(source, items, Span::default())
            .unwrap_err();
        assert_eq!(error.code, "B001");
        assert!(error.message.contains("sequence budget"));
        assert!(!checker.sequences.contains_key(&source));
        assert_eq!(checker.sequence_edges, count);
    }
}
