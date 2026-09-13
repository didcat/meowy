use super::file_modules::case;

#[test]
pub(crate) fn required_booleans_read_fields_exports_and_scoped_imports() {
    for value in [false, true] {
        let data = format!(
            "private:{value};->private;->flag:private;->row:{{->part:{{->flag:private}}}};->width<uint8>:4;->label:\"ready\""
        );
        case(
            "m:@\"./facade.mwy\";alias:m;local:{->flag:false};<T>:{a:alias.flag;b:alias.flag;c:alias.row.part.flag;d:local.flag;copy<boolean>:(c);n<uint8>:m.width;-><(copy<>)[n]>};v<T>:[false,true];f<boolean>:(){<U>:{a:m.flag;b:m.flag;->a<>};v<U>:true;->v};g<boolean>:(){p:@\"./primary.mwy\";<U>:{flag:p.flag;copy:flag;->copy<>};v<U>:false;->v};d:@\"debug\";d.print(v[2]);d.print(f());d.print(g());d.print(alias.label)",
            &[
                ("data.mwy", &data),
                ("facade.mwy", "m:@\"./data.mwy\";->m"),
                ("primary.mwy", "m:@\"./data.mwy\";->flag:!!m"),
            ],
        ).runs(b"true\ntrue\nfalse\nready\n");
    }
}

#[test]
pub(crate) fn required_booleans_reject_unproved_field_and_primary_inputs() {
    for data in [
        "d:@\"debug\";value:{->true;d.print(9)};->value;->flag:value;->row:{->flag:value}",
        "value:=true;->value;->flag:value;->row:{->flag:value}",
        "get<boolean>:(){->true};value:get();->value;->flag:value;->row:{->flag:value}",
        "|true|->true;|true|->flag:true;|true|->row:{->flag:true}",
    ] {
        for binding in ["flag<boolean>:m", "flag:m.flag", "flag:m.row.flag"] {
            let source = format!("m:@\"./facade.mwy\";<T>:{{{binding};-><int32>}}");
            let case = case(
                &source,
                &[("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")],
            );
            for profile in ["debug", "release"] {
                let output = case.command("run", &["--profile", profile, "--json"]);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                let error = String::from_utf8_lossy(&output.stderr);
                assert!(error.contains("\"code\":\"E211\""), "{data}: {error}");
            }
        }
    }
}

#[test]
pub(crate) fn required_booleans_preserve_kind_privacy_identity_and_capture_gates() {
    for (body, code) in [
        ("<T>:{flag<int32>:m.flag;-><int32>}", "E207"),
        ("<T>:{flag<int32>:m;-><int32>}", "E207"),
        ("<T>:{flag<boolean>:m.width;-><int32>}", "E207"),
        ("<T>:{flag:m.flag;-><int32[flag]>}", "B001"),
        ("<T>:{-><int32[m.flag]>}", "B001"),
        ("<T>:{flag:m.private;-><int32>}", "E201"),
        ("<T>:{flag:m;-><int32>}", "E211"),
        ("<T>:{flag:m.flag;->flag}", "E211"),
        ("f<boolean>:(){<T>:{flag:m.flag;->flag<>};->m.flag}", "B001"),
        ("f<boolean>:(){<T>:{flag<boolean>:m;->flag<>};->!m}", "B001"),
        ("<T>:{flag:m.flag;other:!flag;-><int32>}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "private:true;->private;->flag:private;->width<uint8>:4",
            )],
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
}

#[test]
pub(crate) fn required_booleans_read_scalar_and_mixed_module_primaries() {
    for value in [false, true] {
        let data = format!("private:{value};->private;->label:\"ready\"");
        case(
            "m:@\"./facade.mwy\";alias:m;p:@\"./primary.mwy\";<T>:{a<boolean>:(alias);b:p;copy:a;->copy<>};v<T>:false;f<boolean>:(){<U>:{a<boolean>:m;->a<>};v<U>:true;->v};g<boolean>:(){m:@\"./facade.mwy\";<U>:{a<boolean>:m;->a<>};v<U>:false;->v};<Row>:{->alias<>};row<Row>:{->false;->label:\"copy\"};d:@\"debug\";d.print(v);d.print(f());d.print(g());d.print(row.label);d.print(alias.label)",
            &[
                ("data.mwy", &data),
                ("facade.mwy", "m:@\"./data.mwy\";->m"),
                ("primary.mwy", "m:@\"./data.mwy\";->!!m"),
            ],
        ).runs(b"false\ntrue\nfalse\ncopy\nready\n");
    }
}

#[test]
pub(crate) fn required_booleans_charge_sources_per_read_and_reuse_static_scratch() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data =
        format!("private:{{->true;v0:1;{tail}}};->private;->flag:private;->row:{{->flag:private}}");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m"),
    ];
    for value in ["m", "m.flag", "m.row.flag", "copy"] {
        let root = "m:@\"./facade.mwy\";copy:m.flag;";
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{flag<boolean>:{value};->flag<>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("v{id}<boolean>:{value};"))
            .collect::<String>();
        let aliases = (0..20).map(|id| format!("v{id}:flag;")).collect::<String>();
        for profile in ["debug", "release"] {
            for body in [
                separate.clone(),
                format!("<T>:{{flag<boolean>:{value};{aliases}->flag<>}}"),
            ] {
                let output = case(&format!("{root}{body}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            let output = case(&format!("{root}<T>:{{{repeated}-><int32>}}"), &files)
                .command("check", &["--profile", profile, "--json"]);
            assert_eq!(output.status.code(), Some(1));
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"B001\""), "{error}");
            assert!(error.contains("computed type bootstrap budget"), "{error}");
        }
    }
}

#[test]
pub(crate) fn required_booleans_report_retained_dependency_errors_before_execution() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"must not run\");row:{->n<uint8>:255};flag:row.n+1==0;->flag;->ready:flag;->record:{->flag:flag}";
    for value in ["m", "m.ready", "m.record.flag"] {
        let source = format!("m:@\"./facade.mwy\";<T>:{{flag<boolean>:{value};->flag<>}}");
        let case = case(
            &source,
            &[("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")],
        );
        for action in ["check", "build", "run"] {
            for profile in ["debug", "release"] {
                let output = case.command(action, &["--profile", profile, "--json"]);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                let error = String::from_utf8_lossy(&output.stderr);
                assert!(error.contains("\"code\":\"E107\""), "{error}");
                assert!(
                    error.contains(&format!(
                        "\"path\":\"{}\"",
                        case.path.join("data.mwy").display()
                    )),
                    "{error}"
                );
                assert!(
                    error.contains(&format!("\"start\":{}", data.find("row.n+1").unwrap())),
                    "{error}"
                );
            }
        }
    }
}

#[test]
pub(crate) fn required_booleans_check_silently_and_preserve_diamond_initialization() {
    let case = case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";<T>:{flag<boolean>:a;other:b.flag;->flag<>};v<T>:false;f<boolean>:(){p:@\"./data.mwy\";<U>:{flag:p.flag;->flag<>};v<U>:true;->v};d:@\"debug\";d.print(\"entry\");d.print(v);d.print(f())",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->true;->flag:false;->bad:{d.print(\"bad\");->true}",
            ),
            ("a.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(\"a\");->m"),
            (
                "b.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"b\");->flag:m.flag",
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
    case.runs(b"data\nbad\na\nb\nentry\nfalse\ntrue\n");
}
