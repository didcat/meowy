use super::{super::tests::checked, *};

#[test]
pub(crate) fn method_effects_preserve_kinds_roots_snapshots_control_and_owners() {
    let source = "flag:false;xs<int32[3]>:=[1];p:&xs;|flag|p.add({xs=[2,3];->4});f<uint64>:(ys<&int32[3]>){->ys.size()};\"é\".size()";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.methods.len(), 3);
    for (&id, op) in &checker.methods {
        let (owner, Effect::Method(method)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(method.receiver, op.receiver);
        assert_eq!(method.kind, op.kind);
        assert_eq!(method.load, op.load);
        assert_eq!(method.loaded, op.load);
        assert_eq!(method.control, matches!(op.kind, MethodKind::Add { .. }));
        assert_eq!(method.snapshot, matches!(op.kind, MethodKind::Add { .. }));
        assert!(method.terminal);
    }
}

#[test]
pub(crate) fn method_effects_keep_calls_explicit_loads_indices_and_borrows_separate() {
    let source = "get<&int32[3]>:(p<&int32[3]>){->p};xs<int32[3]>:[1];get(&xs).add(2).size();p:&xs;(*p).size();xs[1];borrow:&(xs[1])";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.methods.len(), 3);
    for id in checker.methods.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Method(_)));
    }
    for id in checker.elements.keys() {
        assert!(!matches!(reports.effects[id].1, Effect::Method(_)));
    }
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Call { .. }))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Deref { .. }))
    );
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Index(_)))
    );
    assert!(
        !checker
            .operations
            .values()
            .any(|op| op.kind == OperationKind::Write)
    );
}

#[test]
pub(crate) fn method_effects_preserve_partial_adds_without_fabricated_terminals() {
    for (source, load) in [
        ("xs:[1];'out{xs.add({'out.leave()})}", false),
        ("xs:[1];p:&xs;'out{p.add({'out.leave()})}", true),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let id = *checker.methods.first_key_value().unwrap().0;
        let (_, Effect::Method(method)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(method.loaded, load);
        assert!(method.snapshot && !method.terminal);
        assert!(matches!(
            method.kind,
            MethodKind::Add {
                may_return: false,
                ..
            }
        ));
        assert!(!reports.index.operations.contains_key(&id));
    }
    for source in [
        "stop<never>:(){'loop{'loop.restart()}};stop().add(missing)",
        "xs:[1];stop<never>:(){'loop{'loop.restart()}};stop();xs.size()",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.methods.len(), 1);
        assert!(
            !reports
                .effects
                .contains_key(checker.methods.first_key_value().unwrap().0)
        );
    }
}

#[test]
pub(crate) fn method_effects_retain_only_observed_stages() {
    let (mut checker, mut reports) = checked("xs<int32[2]>:[1];p:&xs;p.add(2)", false);
    let id = *checker.methods.first_key_value().unwrap().0;
    for (port, loaded, snapshot, terminal) in [
        (Port::Projection { point: id, step: 0 }, true, false, false),
        (Port::Snapshot(id), false, true, false),
        (Port::Operation(id), false, false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Method(method)) = &effects[&id] else {
            panic!()
        };
        assert!(method.load);
        assert_eq!(method.loaded, loaded);
        assert_eq!(method.snapshot, snapshot);
        assert_eq!(method.terminal, terminal);
    }
}
