use super::{super::tests::checked, *};

#[test]
pub(crate) fn deref_effects_preserve_pointer_roots_modes_and_value_kinds() {
    for (source, mode) in [
        ("n:1;p:&n;x:*p", crate::hir::ReferenceMode::Shared),
        ("n:=1;p:&!n;x:*p", crate::hir::ReferenceMode::Exclusive),
        ("row:{->n:1};p:&row;x:*p", crate::hir::ReferenceMode::Shared),
        ("xs:[1];p:&xs;x:*p", crate::hir::ReferenceMode::Shared),
        ("n:1;p:&n;q:&p;x:*q", crate::hir::ReferenceMode::Shared),
        ("x:*(&(1+2))", crate::hir::ReferenceMode::Shared),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        let (&id, deref) = checker.derefs.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                0,
                Effect::Deref {
                    input: deref.input,
                    mode,
                    normal: true,
                    control: false,
                }
            )
        );
    }
}

#[test]
pub(crate) fn deref_effects_preserve_nested_pointer_calls_owners_and_control() {
    let source =
        "id<&int32>:(p<&int32>){->p};n:=1;x:*(({n=2;->id(&n)}));f<int32>:(p<&int32>){->*p}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.derefs.len(), 2);
    for (&id, deref) in &checker.derefs {
        assert_eq!(reports.effects[&id].0, deref.owner);
        assert!(matches!(
            reports.effects[&id].1,
            Effect::Deref { input, normal: true, .. } if input == deref.input
        ));
    }
    assert!(reports.effects.values().any(|(owner, effect)| {
        *owner == 0
            && matches!(
                effect,
                Effect::Call {
                    may_return: true,
                    ..
                }
            )
    }));
    let (_, reports) = checked("flag:false;n:1;p:&n;|flag|x:*p", true);
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Deref { control: true, .. }))
    );
}

#[test]
pub(crate) fn deref_effects_keep_stopped_pointers_and_never_referents_distinct() {
    let source = "n:1;stop<never>:(){'loop{'loop.restart()}};x:*(stop());copy:*(&n)";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.derefs.len(), 2);
    for id in checker.derefs.keys() {
        assert!(!reports.effects.contains_key(id));
    }
    let (checker, reports) = checked("f<never>:(p<&never>){->*p}", false);
    let (&id, deref) = checker.derefs.first_key_value().unwrap();
    assert_eq!(
        reports.effects[&id],
        (
            deref.owner,
            Effect::Deref {
                input: deref.input,
                mode: crate::hir::ReferenceMode::Shared,
                normal: false,
                control: false,
            }
        )
    );
    assert!(
        !reports.entries[&deref.owner]
            .1
            .ports
            .contains(&Port::Normal(id))
    );
}

#[test]
pub(crate) fn deref_effects_reject_loads_without_a_reference_mode() {
    let (mut checker, reports) = checked("n:1;x:*(&n)", false);
    let id = *checker.derefs.first_key_value().unwrap().0;
    let before = reports.effects.clone();
    checker.derefs.get_mut(&id).unwrap().mode = None;
    let error = checker
        .operation_effects(&reports, Span::default())
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("dereference-effect identity"));
    assert_eq!(reports.effects, before);
}
