use super::{super::tests::checked, *};

#[test]
pub(crate) fn scalar_effects_preserve_contextual_widths_and_signed_literal_roots() {
    for signed in [false, true] {
        for bits in [8, 16, 32, 64] {
            let name = if signed { "int" } else { "uint" };
            let value = if signed { "-1" } else { "1" };
            let source = format!("x<{name}{bits}><null>:{value}");
            crate::compile(&source).unwrap();
            let (checker, reports) = checked(&source, false);
            assert_eq!(checker.scalar_leaves.len(), 1);
            assert!(checker.unaries.is_empty());
            let (&id, _) = checker.scalar_leaves.first_key_value().unwrap();
            assert_eq!(
                reports.effects[&id],
                (
                    0,
                    Effect::Scalar(Observed {
                        ty: ScalarKind::Int { bits, signed },
                        control: false,
                        operation: true,
                        result: true,
                    })
                )
            );
            assert!(checker.coercions.values().any(|op| op.input == id));
        }
    }
}

#[test]
pub(crate) fn scalar_effects_follow_constants_groups_control_and_function_owners() {
    let source = "flag:false;|flag|x:7;f<float32>:(){->1.25};c:@\"core\";alias:c;t:alias.true;n:((c.null));p:@\"proof\";r:p.revision;value<uint32>:r;text:\"é🙂\";f64:1.25;true:7;shadow:true";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, true);
    assert!(checker.scalar_leaves.values().any(|leaf| leaf.control));
    assert!(checker.scalar_leaves.values().any(|leaf| leaf.owner != 0));
    let mut kinds = Vec::new();
    for (&id, leaf) in &checker.scalar_leaves {
        let (owner, Effect::Scalar(observed)) = &reports.effects[&id] else {
            panic!()
        };
        assert_eq!(*owner, leaf.owner);
        assert_eq!(observed.ty, leaf.kind);
        assert_eq!(observed.control, leaf.control);
        assert!(observed.operation && observed.result);
        kinds.push(observed.ty);
    }
    for ty in [
        ScalarKind::Bool,
        ScalarKind::Null,
        ScalarKind::String,
        ScalarKind::Float { bits: 32 },
        ScalarKind::Float { bits: 64 },
        ScalarKind::Int {
            bits: 32,
            signed: false,
        },
    ] {
        assert!(kinds.contains(&ty), "missing {ty:?} in {kinds:?}");
    }
    for id in checker.local_reads.keys() {
        assert!(matches!(reports.effects[id].1, Effect::Read { .. }));
    }
    assert_eq!(kinds.len(), 9);
}

#[test]
pub(crate) fn scalar_effects_keep_construction_and_result_observations_independent() {
    let (mut checker, mut reports) = checked("7", false);
    let id = *checker.scalar_leaves.first_key_value().unwrap().0;
    for (port, operation, result) in [
        (Port::Operation(id), true, false),
        (Port::Normal(id), false, true),
    ] {
        reports.entries.get_mut(&0).unwrap().1.ports = vec![port, port];
        let effects = checker
            .operation_effects(&reports, Span::default())
            .unwrap();
        assert_eq!(effects.len(), 1);
        let (_, Effect::Scalar(leaf)) = &effects[&id] else {
            panic!()
        };
        assert_eq!(leaf.operation, operation);
        assert_eq!(leaf.result, result);
    }
    assert!(
        checker
            .scalar_effect_stage(&reports, 0, Port::Entry(id), Span::default())
            .unwrap()
            .is_none()
    );
    let (mut checker, reports) = checked("xs:[1];xs.size()", false);
    let method = *checker.methods.first_key_value().unwrap().0;
    assert!(
        checker
            .scalar_effect_stage(&reports, 0, Port::Operation(method), Span::default())
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn scalar_effects_exclude_required_leaves_format_text_and_stopped_successors() {
    let (checker, reports) = checked("<T>:{n:2;-><int32[n]>}", false);
    assert!(checker.scalar_leaves.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Scalar(_)))
    );
    let (checker, reports) = checked(
        "d:@\"debug\";d.print(\"before {7} after\");stop<never>:(){'loop{'loop.restart()}};stop();later:9",
        false,
    );
    assert_eq!(checker.scalar_leaves.len(), 2);
    let mut seen = Vec::new();
    for id in checker.scalar_leaves.keys() {
        seen.push(reports.effects.contains_key(id));
    }
    assert_eq!(seen, [true, false]);
}
