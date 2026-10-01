use super::{super::tests::checked, *};

#[test]
pub(crate) fn reborrow_effects_retain_shared_exclusive_and_implicit_modes_without_loads() {
    for (source, parent_mode, mode) in [
        (
            "n:1;p:&n;q:&*p",
            ReferenceMode::Shared,
            ReferenceMode::Shared,
        ),
        (
            "n:=1;p:&!n;q:&*p",
            ReferenceMode::Exclusive,
            ReferenceMode::Shared,
        ),
        (
            "n:=1;p:&!n;q:&!*p",
            ReferenceMode::Exclusive,
            ReferenceMode::Exclusive,
        ),
        (
            "n:=1;p:&!n;q<&int32>:p",
            ReferenceMode::Exclusive,
            ReferenceMode::Shared,
        ),
        (
            "xs:[1,2];p:&xs;q:&*p",
            ReferenceMode::Shared,
            ReferenceMode::Shared,
        ),
        (
            "n:1;r:&n;p:&r;q:&*p",
            ReferenceMode::Shared,
            ReferenceMode::Shared,
        ),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert_eq!(checker.reborrow_ops.len(), 1);
        let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                op.owner,
                Effect::Reborrow(Observed {
                    parent: op.parent,
                    site: op.site.unwrap(),
                    parent_mode,
                    mode,
                    control: false,
                    acquired: true,
                    result: true,
                })
            )
        );
        assert!(checker.derefs.is_empty());
        assert!(!checker.local_reads.contains_key(&id));
    }
}

#[test]
pub(crate) fn reborrow_effects_keep_acquisition_and_result_visits_independent() {
    let (mut checker, mut reports) = checked("n:=1;p:&!n;q:&!*p", false);
    let id = *checker.reborrow_ops.first_key_value().unwrap().0;
    for port in [Port::Operation(id), Port::Normal(id)] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects_limited(&reports, Span::default(), 1, 0, 0)
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Reborrow(op)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(op.acquired, port == Port::Operation(id));
        assert_eq!(op.result, port == Port::Normal(id));
    }
}

#[test]
pub(crate) fn reborrow_effects_preserve_nested_sites_owners_control_and_conditional_calls() {
    let source = "flag:false;n:=1;p:&!n;|flag|q<&int32>:&!*p;f<&int32>:(p<&!int32>){->p}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert_eq!(checker.reborrow_ops.len(), 3);
    assert!(checker.reborrow_ops.values().any(|op| op.control));
    assert!(checker.reborrow_ops.values().any(|op| op.owner != 0));
    for (&id, op) in &checker.reborrow_ops {
        let (owner, Effect::Reborrow(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, op.owner);
        assert_eq!(observed.control, op.control);
        assert_eq!(observed.site, op.site.unwrap());
        assert!(observed.acquired && observed.result);
        if let Some(parent) = checker.reborrow_ops.get(&op.parent) {
            assert_ne!(observed.site, parent.site.unwrap());
            assert_eq!(observed.mode, ReferenceMode::Shared);
            assert_eq!(parent.mode, ReferenceMode::Exclusive);
        }
    }
    let source = "id<&!int32>:(p<&!int32>){->p};n:=1;q<&int32>:id(&!n)";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    let (&id, op) = checker.reborrow_ops.first_key_value().unwrap();
    let call = checker
        .invocations
        .values()
        .find(|call| call.point == op.parent)
        .unwrap();
    assert!(matches!(reports.effects[&id].1, Effect::Reborrow(_)));
    assert_ne!(id, call.point);
    assert_eq!(call.edges.last().unwrap().route, Route::Returned);
    assert_eq!(op.edges[1].from, Port::Normal(call.point));
}
