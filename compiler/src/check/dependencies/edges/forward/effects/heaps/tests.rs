use super::{super::tests::checked, *};

#[test]
pub(crate) fn heap_effects_preserve_alias_roots_before_conversion_and_call_consumers() {
    let source = "m:@\"memory\";alias:m;a<m.Allocator><null>:alias.heap;b:m.heap;c:b;f<m.Allocator>:(p<m.Allocator>){->p};result:f(((m.heap)));{m:{->heap:7};x:m.heap}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.heap_leaves.len(), 3);
    for (&id, leaf) in &checker.heap_leaves {
        assert_eq!(
            reports.effects[&id],
            (
                leaf.owner,
                Effect::Heap(Observed {
                    ty: FoundationType::Allocator,
                    control: false,
                    operation: true,
                    result: true,
                })
            )
        );
    }
    let first = *checker.heap_leaves.first_key_value().unwrap().0;
    assert!(checker.coercions.values().any(|op| op.input == first));
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Call { .. }))
    );
    assert!(!checker.local_reads.is_empty());
    for id in checker.local_reads.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Read { .. }));
    }
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Scalar(_)))
    );
}

#[test]
pub(crate) fn heap_effects_keep_function_owners_control_and_temporary_consumers() {
    let source =
        "m:@\"memory\";flag:false;|flag|a:m.heap;f<m.Allocator>:(){->m.heap};copy:*(&{->m.heap})";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.heap_leaves.len(), 3);
    assert!(checker.heap_leaves.values().any(|leaf| leaf.control));
    assert!(checker.heap_leaves.values().any(|leaf| leaf.owner != 0));
    assert_eq!(checker.temporary_borrows.len(), 1);
    for (&id, leaf) in &checker.heap_leaves {
        let (owner, Effect::Heap(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, leaf.owner);
        assert_eq!(observed.control, leaf.control);
        assert_eq!(observed.ty, FoundationType::Allocator);
        assert!(observed.operation && observed.result);
    }
}

#[test]
pub(crate) fn heap_effects_keep_handle_and_result_observations_independent() {
    let (mut checker, mut reports) = checked("m:@\"memory\";m.heap", false);
    let id = *checker.heap_leaves.first_key_value().unwrap().0;
    for (port, operation, result) in [
        (Port::Operation(id), true, false),
        (Port::Normal(id), false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Heap(leaf)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(leaf.operation, operation);
        assert_eq!(leaf.result, result);
    }
    assert!(
        checker
            .heap_effect_stage(&reports, 0, Port::Entry(id), Span::default())
            .unwrap()
            .is_none()
    );
    let (mut checker, reports) = checked("7", false);
    let scalar = *checker.scalar_leaves.first_key_value().unwrap().0;
    assert!(
        checker
            .heap_effect_stage(&reports, 0, Port::Operation(scalar), Span::default())
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn heap_effects_exclude_required_hints_and_stopped_successors() {
    let (checker, reports) = checked("m:@\"memory\";<T>:m.heap<>", false);
    assert!(checker.heap_leaves.is_empty());
    let mut checker = Checker::new();
    checker
        .stmt(&crate::parser::parse("m:@\"memory\"").unwrap().stmts[0])
        .unwrap();
    let crate::ast::StmtKind::Expr(expr) =
        crate::parser::parse("m.heap").unwrap().stmts.remove(0).kind
    else {
        panic!()
    };
    assert_eq!(
        checker.hint(&expr),
        Some(crate::hir::Type::Foundation(FoundationType::Allocator))
    );
    checker.required = true;
    checker.expr_point(&expr, None).unwrap();
    assert!(checker.heap_leaves.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Heap(_)))
    );
    let source =
        "m:@\"memory\";before:m.heap;stop<never>:(){'loop{'loop.restart()}};stop();later:m.heap";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.heap_leaves.len(), 2);
    let seen: Vec<_> = checker
        .heap_leaves
        .keys()
        .map(|id| reports.effects.contains_key(id))
        .collect();
    assert_eq!(seen, [true, false]);
}
