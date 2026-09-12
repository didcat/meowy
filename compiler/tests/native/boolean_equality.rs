use super::Case;

#[test]
pub(crate) fn boolean_equality_preserves_all_values_and_nested_evidence() {
    for a in [false, true] {
        for b in [false, true] {
            for op in ["==", "!="] {
                let same = if op == "==" { a == b } else { a != b };
                let source = format!(
                    "d:@\"debug\";n<uint8>:{{a:{a};b:{b};same:a{op}b;|same|->4;|!same|->2}};<T>:{{-><int32[n]>}};v<T>:[7];d.print(v[1]);d.print(n)"
                );
                Case::new(&source).runs(format!("7\n{}\n", if same { 4 } else { 2 }).as_bytes());
            }
        }
    }
    Case::new("d:@\"debug\";a:true;b:!a;same:(a!=b)==true;ready:{row:{->n<uint8>:4};->same==(row.n==4)};n:{|ready|->4;|!ready|->2};<T>:{-><int32[n]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn boolean_equality_retains_first_evaluated_operand_errors() {
    for (value, failed, code) in [
        ("left==right", "255+1", "E107"),
        ("right!=left", "254+2", "E107"),
        ("true==right", "254+2", "E107"),
        ("false!=left", "255+1", "E107"),
        ("left==get()", "255+1", "E107"),
        ("get()==left", "", "E211"),
    ] {
        let source = format!(
            "get<boolean>:(){{->true}};|false|{{left<boolean>:{{bad<uint8>:255+1;->true}};right<boolean>:{{bad<uint8>:254+2;->false}};n<uint8>:{{same:{value};|same|->4;|!same|->2}};<T>:{{v:n;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, code, "{source}: {error:?}");
        if !failed.is_empty() {
            assert_eq!(error.span.start, source.find(failed).unwrap());
        }
    }
}

#[test]
pub(crate) fn boolean_equality_requires_both_operands_to_be_eligible() {
    for value in [
        "false==get()",
        "true==get()",
        "false!=get()",
        "true!=get()",
        "get()==false",
        "mutable==false",
        "effect==effect",
        "1.0==1.0",
        "\"x\"==\"x\"",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{d.print(9);->false}};mutable:=false;effect:{{d.print(8);->true}};n:{{unused:{value};->4}};<T>:{{v:n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}
