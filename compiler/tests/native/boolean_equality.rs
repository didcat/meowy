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

#[test]
pub(crate) fn boolean_equality_charges_both_cached_operands_on_every_read() {
    let tail = (1..20)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    for value in [
        "a", "a==a", "a==alias", "false==a", "true!=a", "false&&a", "true||a",
    ] {
        let data = format!("a:{{->true;v0:1;{tail}}};alias:a;n:{{unused:{value};->4}};->width:n");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{n:m.width;-><int32[n]>}};"))
            .collect::<String>();
        for profile in ["debug", "release"] {
            let output =
                super::file_modules::case(&format!("m:@\"./facade.mwy\";{separate}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{value}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            for reads in [4, 20] {
                let repeated = (0..reads)
                    .map(|id| format!("n{id}:m.width;"))
                    .collect::<String>();
                let output = super::file_modules::case(
                    &format!("m:@\"./facade.mwy\";<T>:{{{repeated}-><int32>}}"),
                    &files,
                )
                .command("check", &["--profile", profile, "--json"]);
                let error = String::from_utf8_lossy(&output.stderr);
                let heavy = !matches!(value, "false&&a" | "true||a")
                    && (reads == 20 || matches!(value, "a==a" | "a==alias"));
                if heavy {
                    assert_eq!(output.status.code(), Some(1), "{value}, {reads}");
                    assert!(error.contains("\"code\":\"B001\""), "{error}");
                    assert!(error.contains("computed type bootstrap budget"), "{error}");
                } else {
                    assert!(output.status.success(), "{value}, {reads}: {error}");
                }
            }
        }
    }
}

#[test]
pub(crate) fn boolean_equality_preserves_runtime_order_and_module_staging() {
    for op in ["==", "!="] {
        let source = format!(
            "d:@\"debug\";left<boolean>:(){{d.print(\"left\");->false}};right<boolean>:(){{d.print(\"right\");->true}};|left(){op}right()|d.print(\"taken\");d.print(\"done\")"
        );
        Case::new(&source).runs(if op == "==" {
            b"left\nright\ndone\n"
        } else {
            b"left\nright\ntaken\ndone\n"
        });
    }
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{-><int32[m.width]>};v<T>:[7];d.print(\"entry\");d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->width<uint8>:4",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";n<uint8>:{base:m.width;flag:base==4;same:flag==true;ready:{->same!=false};|ready|->base;|!ready|->2};->width:n;d.print(\"facade\")",
            ),
        ],
    );
    for action in ["check", "build"] {
        for profile in ["debug", "release"] {
            let output = case.command(action, &["--profile", profile]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
    }
    case.runs(b"data\nfacade\nentry\n7\n");
}
