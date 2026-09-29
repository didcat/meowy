use super::{super::tests::checked, *};

#[test]
pub(crate) fn index_effects_preserve_pre_position_snapshots_control_and_owners() {
    let source = "flag:false;xs<int32[3]>:=[1,2];p:&xs;|flag|p[{xs=[];->1}];f<int32>:(ys<int32[1]>){->ys[1]}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.indices.len(), 2);
    for (&id, op) in &checker.indices {
        let (owner, Effect::Index(index)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(index.receiver, op.receiver);
        assert_eq!(index.access, op.access.unwrap());
        assert_eq!(index.load, op.load);
        assert_eq!(index.loaded, op.load);
        assert_eq!(index.control, *owner == 0);
        assert!(index.snapshot && index.read && index.access.normal);
        assert_eq!(index.access.length, None);
    }
}

#[test]
pub(crate) fn index_effects_keep_nested_calls_explicit_loads_and_element_borrows_separate() {
    let source = "get<&int32[2]>:(p<&int32[2]>){->p};xs<int32[2]>:[7];get(&xs)[1];nested:[[1]];nested[1][1];p:&xs;(*p)[1];borrow:&(xs[1])";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.indices.len(), 4);
    assert_eq!(checker.elements.len(), 1);
    for id in checker.indices.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Index(_)));
    }
    for id in checker.elements.keys() {
        assert!(!matches!(reports.effects[id].1, Effect::Index(_)));
    }
    assert!(reports.effects.values().any(|(_, effect)| matches!(
        effect,
        Effect::Call {
            may_return: true,
            ..
        }
    )));
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Deref { .. }))
    );
}

#[test]
pub(crate) fn index_effects_keep_partial_loads_and_snapshots_without_phantom_reads() {
    for (source, load) in [
        ("xs:[1];'out{xs[{'out.leave()}]}", false),
        ("xs:[1];p:&xs;'out{p[{'out.leave()}]}", true),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let id = *checker.indices.first_key_value().unwrap().0;
        let (_, Effect::Index(index)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(index.loaded, load);
        assert!(index.snapshot);
        assert!(!index.access.may_return);
        assert!(!index.read && !index.access.normal);
        assert!(!reports.index.operations.contains_key(&id));
    }
    for source in [
        "stop<never>:(){'loop{'loop.restart()}};stop()[missing]",
        "xs:[1];stop<never>:(){'loop{'loop.restart()}};stop();xs[1]",
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.indices.len(), 1);
        assert!(
            !reports
                .effects
                .contains_key(checker.indices.first_key_value().unwrap().0)
        );
    }
}

#[test]
pub(crate) fn index_effects_report_only_observed_stages() {
    let (mut checker, mut reports) = checked("xs:[1];p:&xs;p[1]", false);
    let id = *checker.indices.first_key_value().unwrap().0;
    for (port, loaded, snapshot, read) in [
        (Port::Projection { point: id, step: 0 }, true, false, false),
        (Port::Snapshot(id), false, true, false),
        (Port::Operation(id), false, false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Index(index)) = &effects[&id] else {
            panic!()
        };
        assert!(index.load);
        assert_eq!(index.loaded, loaded);
        assert_eq!(index.snapshot, snapshot);
        assert_eq!(index.read, read);
    }
}
