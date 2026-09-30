use super::{super::tests::checked, *};

#[test]
pub(crate) fn typed_effects_exclude_stopped_operands_and_later_typed_operations() {
    for tail in ["stop()<int32>", "stop()~<int32>"] {
        let source =
            format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};v:{tail};later:7<int32>");
        crate::compile(&source).unwrap();
        let (mut checker, reports) = checked(&source, false);
        assert_eq!(checker.typed_ops.len(), 2);
        for id in checker.typed_ops.keys() {
            assert!(!reports.effects.contains_key(id));
        }
        let (&id, op) = checker.typed_ops.iter().find(|(_, op)| !op.normal).unwrap();
        let owner = op.owner;
        let call = checker.invocations.values().next().unwrap();
        assert_eq!(call.point, op.input);
        assert!(!call.may_return);
        for port in [Port::Operation(id), Port::Normal(id)] {
            assert!(
                checker
                    .typed_effect_stage(&reports, owner, port, Span::default())
                    .is_err()
            );
        }
    }
}

#[test]
pub(crate) fn typed_effects_keep_computed_targets_required_reads_and_queries_separate() {
    let source = "n:2;v:[1,2]~<({-><int32[n]>})>";
    crate::compile(source).unwrap();
    let (checker, reports) = checked(source, false);
    assert_eq!(checker.typed_ops.len(), 1);
    let (&id, op) = checker.typed_ops.first_key_value().unwrap();
    assert!(matches!(reports.effects[&id].1, Effect::Typed(_)));
    let reads: Vec<_> = checker.body_inputs.values().flatten().collect();
    assert!(!reads.is_empty());
    for read in reads {
        assert_ne!(read.point, op.input);
        assert!(!reports.effects.contains_key(&read.point));
        assert!(
            !op.edges
                .iter()
                .any(|edge| edge.to == Port::Entry(read.point))
        );
    }
    let (checker, reports) = checked("<T>:{n:2;-><int32[n]>};<U>:7<>", false);
    assert!(checker.typed_ops.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Typed(_)))
    );
    let source = "p:@\"proof\";q:p.can_copy<uint8>();<F>:q.always<>;<T>:7<>";
    assert_eq!(crate::compile(source).unwrap_err()[0].code, "B001");
    let (checker, reports) = checked(source, false);
    assert!(checker.typed_ops.is_empty());
    assert!(
        !reports
            .effects
            .values()
            .any(|(_, effect)| matches!(effect, Effect::Typed(_)))
    );
}

#[test]
pub(crate) fn typed_effects_preserve_operand_target_and_ascription_error_precedence() {
    for (tail, code) in [
        ("missing~<Missing>", "E201"),
        ("7~<Missing>", "E202"),
        ("(1/0)~<Missing>", "E107"),
        ("d.panic(\"stop\")<Missing>", "E202"),
        ("d.panic(\"stop\")~<int32[1/0]>", "E107"),
        ("7~<boolean>", "E208"),
        ("v<int32><null>:=1;|v<int32>|{v=null;x:v~<int32>}", "E208"),
        ("n:=7;p:&n;n=8;x:(*p)~<int32>", "E302"),
    ] {
        let source = format!("d:@\"debug\";{tail}");
        assert_eq!(crate::compile(&source).unwrap_err()[0].code, code, "{tail}");
    }
}
