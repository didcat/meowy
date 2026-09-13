use super::file_modules::case;

#[test]
pub(crate) fn boolean_primaries_keep_values_aliases_projections_and_reexports() {
    for value in [false, true] {
        for fields in ["", ";->label:\"ready\""] {
            let data = format!("private:{value};->{{->private;unused:7}}{fields}");
            case(
                "s:@\"./data.mwy\";alias:s;copy:!!alias;m:@\"./outer.mwy\";n:{flag:m&&true;ok:flag==copy&&m.enabled==s;|ok|->4;|!ok|->2};<T>:{-><int32[n]>};v<T>:[7];f<int32>:(){<U>:{count:n;-><int32[count]>};u<U>:[9];->u[1]};d:@\"debug\";d.print(v[1]);d.print(n);d.print(!m);d.print(f())",
                &[
                    ("data.mwy", &data),
                    ("facade.mwy", "m:@\"./data.mwy\";->m;->enabled<boolean>:m"),
                    ("outer.mwy", "m:@\"./facade.mwy\";->m"),
                ],
            ).runs(format!("7\n4\n{}\n9\n", !value).as_bytes());
            case(
                "m:@\"./data.mwy\";n:{flag:!m;|flag|->2;|!flag|->4};<T>:{-><int32[n]>};d:@\"debug\";d.print(n)",
                &[("data.mwy", &data)],
            ).runs(format!("{}\n", if value { 4 } else { 2 }).as_bytes());
        }
    }
    case(
        "m:@\"./data.mwy\";alias:m;<Row>:{->alias<>};row<Row>:{->false;->label:\"copy\"};n:{m:@\"./data.mwy\";flag:m&&true;|flag|->4;|!flag|->2};<T>:{-><int32[n]>};d:@\"debug\";d.print(alias.label);d.print(row.label);d.print(n)",
        &[("data.mwy", "->true;->label:\"ready\"")],
    ).runs(b"ready\ncopy\n4\n");
}

#[test]
pub(crate) fn boolean_primaries_reject_unproved_initializers_through_facades() {
    for data in [
        "d:@\"debug\";->{->true;d.print(\"must not run\")}",
        "value:=true;->value",
        "|true|->true",
        "get<boolean>:(){->true};->get()",
    ] {
        for facade in ["m:@\"./data.mwy\";->m", "m:@\"./data.mwy\";copy:!m;->!copy"] {
            let case = case(
                "m:@\"./facade.mwy\";n:{flag:!m;->4};<T>:{v:n;-><int32>}",
                &[("data.mwy", data), ("facade.mwy", facade)],
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
pub(crate) fn boolean_primaries_keep_kind_scope_and_record_boundaries() {
    for (body, data, code) in [
        ("copy<boolean>:m", "->true", "B001"),
        (
            "n:{flag:m==m;->4};<T>:{v:n;-><int32>}",
            "->true;->label:1",
            "E211",
        ),
        ("<T>:{flag<boolean>:=m;-><int32>}", "->true", "B001"),
        ("<T>:{flag:m<true;-><int32>}", "->true", "B001"),
        ("<T>:{-><int32[m]>}", "->true", "B001"),
        ("f<boolean>:(){->!m}", "->true", "B001"),
        ("copy:!!m;f<boolean>:(){->copy}", "->true", "B001"),
        (
            "f<int32>:(){n:{flag:!m;->4};<T>:{v:n;-><int32>};->1}",
            "->true",
            "B001",
        ),
        (
            "n:{copy:{->m};->4};<T>:{v:n;-><int32>}",
            "->true;->enabled:true",
            "E211",
        ),
        (
            "n:{flag:m.private;->4};<T>:{v:n;-><int32>}",
            "private:true;->private",
            "E201",
        ),
        (
            "row:{->true;->n:4};n:{flag:!row;->4};<T>:{v:n;-><int32>}",
            "->true",
            "E211",
        ),
    ] {
        let source = format!("m:@\"./data.mwy\";{body}");
        let output = case(&source, &[("data.mwy", data)]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
}

#[test]
pub(crate) fn boolean_primaries_charge_forwarding_work_per_read_and_reset_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    for fields in ["", ";->label:\"ready\""] {
        let data = format!("->{{->true;v0:1;{tail}}}{fields}");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
            ("outer.mwy", "m:@\"./facade.mwy\";->m"),
        ];
        for value in ["!m", "m==true", "!m==!m", "copy", "false&&m"] {
            let root = format!("m:@\"./outer.mwy\";copy:!!m;n:{{unused:{value};->4}};");
            let separate = (0..20)
                .map(|id| format!("<T{id}>:{{v:n;-><int32[v]>}};"))
                .collect::<String>();
            let repeated = (0..20).map(|id| format!("v{id}:n;")).collect::<String>();
            for profile in ["debug", "release"] {
                let output = case(&format!("{root}{separate}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let output = case(&format!("{root}<T>:{{{repeated}-><int32>}}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
                let error = String::from_utf8_lossy(&output.stderr);
                if value == "false&&m" {
                    assert!(output.status.success(), "{error}");
                } else {
                    assert_eq!(output.status.code(), Some(1));
                    assert!(error.contains("\"code\":\"B001\""), "{error}");
                    assert!(error.contains("computed type bootstrap budget"), "{error}");
                }
            }
        }
    }
}

#[test]
pub(crate) fn boolean_primaries_keep_independent_exports_and_diamond_startup() {
    let case = case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";n:{flag:a&&!b;|flag|->4;|!flag|->2};<T>:{-><int32[n]>};v<T>:[7];d:@\"debug\";d.print(\"entry\");d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->true;->label:{d.print(\"label\");->\"ready\"};d.print(\"tail\")",
            ),
            ("a.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(\"a\");->m"),
            (
                "b.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"b\");->!m",
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
    case.runs(b"data\nlabel\ntail\na\nb\nentry\n7\n");
}

#[test]
pub(crate) fn boolean_primaries_report_original_dependency_errors_through_reexports() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"must not run\");row:{->width<uint8>:255};->row.width+1==0;->label:\"ready\"";
    for value in ["!m", "m.flag"] {
        let source = format!("m:@\"./outer.mwy\";n:{{unused:{value};->4}};<T>:{{v:n;-><int32>}}");
        let case = case(
            &source,
            &[
                ("data.mwy", data),
                ("facade.mwy", "m:@\"./data.mwy\";->m;->flag:!!m"),
                ("outer.mwy", "m:@\"./facade.mwy\";->m"),
            ],
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
                    error.contains(&format!("\"start\":{}", data.find("row.width+1").unwrap())),
                    "{error}"
                );
            }
        }
    }
}

#[test]
pub(crate) fn boolean_primaries_preserve_runtime_initialization_failure() {
    let case = case(
        "m:@\"./data.mwy\";n:{unused:!m;->4};<T>:{v:n;-><int32>};d:@\"debug\";d.print(\"entry must not run\")",
        &[(
            "data.mwy",
            "d:@\"debug\";stop<boolean>:(){->true};d.print(\"init\");|stop()|d.panic(\"stop\");->true",
        )],
    );
    for profile in ["debug", "release"] {
        let output = case.command("check", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"init\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}
