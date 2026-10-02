use super::super::tests::{accepts, rejects};

#[test]
pub(crate) fn stopped_binary_contexts_allow_independent_operand_construction() {
    for right in ["{->[1]}", "{->n:1}", "{->true}", "{->{->[1]}}"] {
        accepts(&format!("'out{{x:({{'out.leave()}})=={right}}}"));
        accepts(&format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};x:stop()!={right}"
        ));
        accepts(&format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};x:{right}==stop()"
        ));
    }
    for op in ["+", "-", "*", "/", "%", "<", "<=", ">", ">="] {
        accepts(&format!("'out{{x:({{'out.leave()}}){op}{{->7}}}}"));
        accepts(&format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};x:{{->7}}{op}stop()"
        ));
    }
    accepts("f<never>:(r<{-><never>;tag<boolean>}>){->r+{->7}}");
    accepts("d:@\"debug\";stop<never>:(){d.panic(\"stop\")};n<uint64>:1;x:(stop()+4294967296)+n");
    accepts("f<never>:(r<{-><never>;tag<boolean>}>){n<uint64>:1;->(r+4294967296)+n}");
}

#[test]
pub(crate) fn stopped_binary_contexts_preserve_expected_widths_and_boolean_domains() {
    for expr in ["stop()+{->4294967296}", "{->4294967296}+stop()"] {
        accepts(&format!(
            "d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};x<uint64>:{expr}"
        ));
    }
    for expr in ["stop()+{->256}", "{->256}+stop()"] {
        rejects(
            &format!("d:@\"debug\";stop<never>:(){{d.panic(\"stop\")}};x<uint8>:{expr}"),
            "E216",
        );
    }
    for op in ["&&", "||"] {
        accepts(&format!("'out{{x:({{'out.leave()}}){op}{{->true}}}}"));
        rejects(&format!("'out{{x:({{'out.leave()}}){op}{{->1}}}}"), "E207");
    }
    accepts("n<uint8>:7;x:{->7}==n;y:n=={->7};r:{->n;->tag:true};z:r=={->7}");
    rejects("n<uint8>:7;x:n=={->256}", "E216");
    accepts("<T>:{n:1+2;-><uint8[n]>};v<T>:[7];x:false&&{->1/0==0}");
}

#[test]
pub(crate) fn stopped_binary_contexts_keep_unreachable_body_errors_and_precedence() {
    for (right, code) in [
        ("{->missing}", "E201"),
        ("{n<uint8>:256;->[1]}", "E216"),
        ("{->n:1;->n:2}", "E203"),
        ("{x:true+1;->[1]}", "E222"),
    ] {
        rejects(&format!("'out{{x:({{'out.leave()}})=={right}}}"), code);
    }
    rejects("'out{x:({n<uint8>:256;'out.leave()})=={->missing}}", "E216");
}

#[test]
pub(crate) fn stopped_binary_contexts_keep_never_calls_out_of_record_coercion() {
    use super::super::dependencies::{BinaryClass, SequenceSource};
    use super::Checker;

    let source = "d:@\"debug\";stop<never>:(){d.panic(\"stop\")};r:{->7;->n:1};x:stop()==r";
    accepts(source);
    let mut checker = Checker::new();
    checker
        .block(&crate::parser::parse(source).unwrap(), None, None)
        .unwrap();
    let (&id, op) = checker.binaries.first_key_value().unwrap();
    assert_eq!(op.types.inputs[0], BinaryClass::Never);
    assert_eq!(op.types.result, BinaryClass::Never);
    assert_eq!(op.plan.normal, [false, true]);
    assert!(!op.plan.equality && !op.plan.checked);
    assert!(
        checker.sequences[&SequenceSource::Expr(id)]
            .edges
            .is_empty()
    );
    assert_eq!(op.edges.len(), 1);
}
