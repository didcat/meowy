use super::{super::tests::checked, *};

#[test]
pub(crate) fn coercion_effects_keep_projected_never_without_operation_or_result() {
    let source = "d:@\"debug\";f<boolean>:(r<{-><never>;tag<boolean>}>){->r};x<boolean>:d.panic(\"stop\");later<int32><null>:7";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    let stops: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.kind == CoercionKind::Stopped)
        .map(|(&id, op)| (id, op.owner, op.primary))
        .collect();
    assert_eq!(stops.len(), 2);
    for (id, owner, primary) in stops {
        assert_eq!(reports.effects.contains_key(&id), primary);
        if primary {
            let (_, Effect::Coercion(op)) = &reports.effects[&id] else {
                panic!()
            };
            assert!(op.projected && !op.operation && !op.result);
            assert!(!reports.index.operations.contains_key(&id));
        }
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .coercion_effect_stage(&reports, owner, port, Span::default())
                    .is_err()
            );
        }
    }
    let later: Vec<_> = checker
        .coercions
        .iter()
        .filter(|(_, op)| op.owner == 0 && op.kind == CoercionKind::Convert)
        .collect();
    assert_eq!(later.len(), 1);
    assert!(!reports.effects.contains_key(later[0].0));
}

#[test]
pub(crate) fn coercion_effects_keep_required_and_shared_reference_paths_separate() {
    let (checker, reports) = checked("<T>:{n<int32>:1+2;-><int32[n]>}", false);
    assert!(checker.coercions.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Coercion(_)))
    );
    let source = "n:=7;p:&!n;q<&int32>:p;copy:*q";
    crate::compile(source).unwrap();
    let (mut checker, reports) = checked(source, false);
    assert_eq!(checker.reborrow_ops.len(), 1);
    let id = *checker.reborrow_ops.first_key_value().unwrap().0;
    assert!(!checker.coercions.contains_key(&id));
    assert!(matches!(reports.effects[&id].1, Effect::Reborrow(_)));
    assert!(
        checker
            .coercion_effect_stage(&reports, 0, Port::Operation(id), Span::default())
            .unwrap()
            .is_none()
    );
    let (checker, _) = checked("n:7;p:&n;q<&int32>:p", false);
    assert!(checker.coercions.is_empty() && checker.reborrow_ops.is_empty());
}

#[test]
pub(crate) fn coercion_effects_preserve_type_ascription_and_borrow_errors() {
    for (source, code) in [
        ("x<boolean>:1+2", "E207"),
        ("r:{->7;->tag:true};x<string>:r", "E207"),
        ("f:(v<int32><null>){x:v~<int32>}", "E208"),
        ("n:=7;p:&!n;q<&int32>:p;n=8;copy:*q", "E302"),
        ("d:@\"debug\";d.panic(\"stop\");x<boolean>:1", "E207"),
    ] {
        assert_eq!(
            crate::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}
