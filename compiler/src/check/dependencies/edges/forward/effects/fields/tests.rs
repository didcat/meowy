use super::{super::tests::checked, *};

#[test]
pub(crate) fn field_effects_preserve_indices_and_owned_shared_explicit_loads() {
    for (source, index, load, explicit) in [
        ("r:{->z:1;->a:2};x:r.z", 1, false, false),
        ("r:{->z:1;->a:2};p:&r;x:p.z", 1, true, false),
        ("r:{->z:1};p:&r;x:(*p).z", 0, false, true),
        ("n:1;r:{->z:&n};x:r.z", 0, false, false),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        assert!(checker.locals.is_empty());
        let (&id, field) = checker.fields.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                0,
                Effect::Field {
                    input: field.input,
                    index,
                    load,
                    normal: true,
                    control: false,
                }
            )
        );
        assert_eq!(
            reports
                .effects
                .values()
                .any(|(_, effect)| matches!(effect, Effect::Deref { .. })),
            explicit
        );
    }
}

#[test]
pub(crate) fn field_effects_preserve_call_roots_nested_fields_narrowing_and_owners() {
    let source = "f<{z<int32>}>:(){->{->z:3}};x:f().z;r:{->i:{->z:4}};y:r.i.z;g<null>:(r<{n<int32><null>}>){|r.n<int32>|{v<int32>:r.n}}";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.fields.len(), 5);
    for (&id, field) in &checker.fields {
        assert_eq!(reports.effects[&id].0, field.owner);
        assert!(
            matches!(reports.effects[&id].1, Effect::Field { input, normal: true, .. } if input == field.input)
        );
        assert!(checker.narrowings.values().any(|op| op.input == id));
    }
    let call = checker.invocations.values().next().unwrap();
    assert!(
        checker
            .fields
            .values()
            .any(|field| field.input == call.point)
    );
    assert!(matches!(
        reports.effects[&call.point].1,
        Effect::Call {
            may_return: true,
            ..
        }
    ));
    assert!(
        reports
            .effects
            .values()
            .any(|(owner, effect)| *owner != 0 && matches!(effect, Effect::Field { .. }))
    );
    let (_, reports) = checked("flag:false;r:{->n:1};|flag|v:r.n", true);
    assert!(
        reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Field { control: true, .. }))
    );
}

#[test]
pub(crate) fn field_effects_preserve_never_stopped_required_and_static_boundaries() {
    for (source, load) in [
        ("f<never>:(r<{n<never>}>){->r.n}", false),
        ("f<never>:(r<&{n<never>}>){->r.n}", true),
    ] {
        crate::compile(source).unwrap();
        let (checker, reports) = checked(source, false);
        let (&id, field) = checker.fields.first_key_value().unwrap();
        assert_eq!(
            reports.effects[&id],
            (
                field.owner,
                Effect::Field {
                    input: field.input,
                    index: 0,
                    load,
                    normal: false,
                    control: false,
                }
            )
        );
        assert!(
            !reports.entries[&field.owner]
                .1
                .ports
                .contains(&Port::Normal(id))
        );
    }
    let source = "r:{->n:1};stop<never>:(){'loop{'loop.restart()}};stop();x:r.n";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.fields.len(), 1);
    assert!(
        !reports
            .effects
            .contains_key(checker.fields.first_key_value().unwrap().0)
    );
    let source = "p:@\"proof\";v:p.revision;r:{->n:3};<T>:{-><uint8[r.n]>};xs<T>:[1]";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert!(checker.fields.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Field { .. }))
    );
}

#[test]
pub(crate) fn field_effects_reject_out_of_range_indices_without_changing_reports() {
    let (mut checker, reports) = checked("r:{->n:1};x:r.n", false);
    let id = *checker.fields.first_key_value().unwrap().0;
    let before = reports.effects.clone();
    let field = checker.fields.get_mut(&id).unwrap();
    field.index = field.count;
    let error = checker
        .operation_effects(&reports, Span::default())
        .unwrap_err();
    assert_eq!(error.code, "B001");
    assert!(error.message.contains("field-effect identity"));
    assert_eq!(reports.effects, before);
}
