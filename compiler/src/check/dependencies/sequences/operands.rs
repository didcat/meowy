use super::{tests::check, *};

#[test]
pub(crate) fn operand_sequences_order_effectful_blocks_and_retain_erased_uses() {
    let source = "p:@\"proof\";n:3;x:({q:p.can_copy<uint8>();->1})+({<T>:{-><uint8[n]>};->2})";
    let (checker, _) = check(source);
    let (root, sequence) = checker
        .sequences
        .iter()
        .find_map(|(source, sequence)| {
            if let Source::Expr(root) = source {
                Some((*root, sequence))
            } else {
                None
            }
        })
        .unwrap();
    let left = sequence.items[0].unwrap();
    let right = sequence.items[1].unwrap();
    assert_eq!(
        sequence.edges,
        [Edge::new(
            Port::Normal(left),
            Port::Entry(right),
            Route::Next
        )]
    );
    assert_eq!(checker.points[left].parent, Some(root));
    assert_eq!(checker.points[right].parent, Some(root));
    assert!(source[checker.points[left].span.start..checker.points[left].span.end].contains("q:"));
    assert!(
        source[checker.points[right].span.start..checker.points[right].span.end].contains("<T>")
    );
    assert_eq!(checker.queries.len(), 1);
    assert_eq!(checker.body_inputs.values().flatten().count(), 1);
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
}

#[test]
pub(crate) fn operand_sequences_keep_nested_functions_and_short_circuit_paths_distinct() {
    let source = "f<int32>:(){->1+2};x:(1+2)*(3+4);y:true&&(1<2)";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let mut count = 0;
    for (source, sequence) in &checker.sequences {
        let Source::Expr(root) = source else { continue };
        assert_eq!(checker.points[*root].kind, PointKind::Expr);
        for child in sequence.items.iter().flatten() {
            assert_eq!(checker.points[*child].parent, Some(*root));
            assert_eq!(checker.points[*child].owner, sequence.owner);
        }
        count += 1;
    }
    assert_eq!(count, 5);
    let logical = *checker.branch_edges.keys().next().unwrap();
    assert!(!checker.sequences.contains_key(&Source::Expr(logical)));
}

#[test]
pub(crate) fn operand_sequences_keep_composed_roots_unknown() {
    let source = "a:{->1;->n:2};b:{->2;->n:3};same:a==b";
    crate::compile(source).unwrap();
    let (checker, _) = check(source);
    let sequence = checker
        .sequences
        .iter()
        .find_map(|(source, sequence)| matches!(source, Source::Expr(_)).then_some(sequence))
        .unwrap();
    assert_eq!(sequence.items, [None, None]);
    assert!(sequence.edges.is_empty());
}

#[test]
pub(crate) fn operand_sequences_preserve_failures_and_unknown_call_completion() {
    let mut checker = Checker::new();
    let block = crate::parser::parse("1+false").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E222");
    assert!(checker.sequences.is_empty());
    let mut checker = Checker::new();
    let block = crate::parser::parse("x<boolean>:1+2").unwrap();
    assert_eq!(checker.block(&block, None, None).unwrap_err().code, "E207");
    let Source::Expr(root) = checker.sequences.keys().next().unwrap() else {
        panic!()
    };
    assert!(!checker.points[*root].complete);
    assert!(checker.bodies.is_empty());
    let (checker, _) = check("f<int32>:(){->1};x:f()+2");
    let sequence = checker
        .sequences
        .iter()
        .find_map(|(source, sequence)| matches!(source, Source::Expr(_)).then_some(sequence))
        .unwrap();
    let call = sequence.items[0].unwrap();
    assert!(
        !sequence
            .edges
            .iter()
            .any(|edge| edge.from == Port::Entry(call))
    );
}
