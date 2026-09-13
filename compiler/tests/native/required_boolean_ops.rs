use super::file_modules::case;

#[test]
pub(crate) fn required_boolean_logic_reads_modules_in_functions_without_capture() {
    case(
        "m:@\"./facade.mwy\";<T>:{flag:!m||m.flag;copy:flag&&true;->copy<>};v<T>:false;f<boolean>:(){p:@\"./facade.mwy\";<U>:{flag:p&&(!p.flag||true);->flag<>};v<U>:true;->v};d:@\"debug\";d.print(v);d.print(f());d.print(m.label)",
        &[("data.mwy", "->true;->flag:false;->label:\"ready\""), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"false\ntrue\nready\n");
}

#[test]
pub(crate) fn required_boolean_logic_checks_skipped_names_types_and_purity() {
    for (body, code) in [
        ("<T>:{flag:false&&missing;->flag<>}", "E201"),
        ("<T>:{flag:true||m.private;->flag<>}", "E201"),
        ("<T>:{flag:false&&m.width;->flag<>}", "E222"),
        ("<T>:{flag:!4;->flag<>}", "E222"),
        ("<T>:{flag:true&&m.bad;->flag<>}", "E211"),
        ("<T>:{flag:false||m.bad;->flag<>}", "E211"),
        ("<T>:{flag:false&&get();->flag<>}", "B001"),
        ("f<boolean>:(){<T>:{flag:!m;->flag<>};->!m}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";get<boolean>:(){{->true}};{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "d:@\"debug\";private:true;->true;->width:4;->bad:{d.print(9);->false}",
            )],
        )
        .command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{body}");
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
    case(
        "m:@\"./data.mwy\";f<boolean>:(runtime<boolean>){<T>:{a:false&&runtime;b:true||m.bad;->a<>};v<T>:false;->v};d:@\"debug\";d.print(f(true))",
        &[("data.mwy", "d:@\"debug\";->bad:{d.print(9);->false}")],
    ).runs(b"9\nfalse\n");
}

#[test]
pub(crate) fn required_boolean_equality_preserves_module_projection_and_capture_rules() {
    case(
        "m:@\"./facade.mwy\";<T>:{same:m==true;other:false!=m;copy:!m==!m;flag:m.flag!=same;->flag<>};v<T>:false;f<boolean>:(){p:@\"./facade.mwy\";<U>:{same:p.flag==false;->same<>};v<U>:true;->v};d:@\"debug\";d.print(v);d.print(f());d.print(m.label)",
        &[("data.mwy", "->true;->flag:false;->label:\"ready\""), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"false\ntrue\nready\n");
    for (value, code) in [
        ("m==m", "B001"),
        ("true==m.width", "E222"),
        ("m.label!=m.label", "B001"),
        ("false==m.bad", "E211"),
        ("true!=m.bad", "E211"),
    ] {
        let source = format!("m:@\"./data.mwy\";<T>:{{flag:{value};->flag<>}}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "d:@\"debug\";->true;->width:4;->label:\"ready\";->bad:{d.print(9);->false}",
            )],
        )
        .command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{value}: {error}"
        );
    }
}

#[test]
pub(crate) fn required_boolean_operators_charge_evaluated_reads_and_reset_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("private:{{->true;v0:1;{tail}}};->private;->flag:private");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m"),
    ];
    for (value, heavy) in [
        ("!m", true),
        ("m.flag==m.flag", true),
        ("m.flag!=false", true),
        ("m.flag&&m.flag", true),
        ("false&&m.flag", false),
        ("true||m.flag", false),
    ] {
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{flag:{value};->flag<>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("v{id}:{value};"))
            .collect::<String>();
        for profile in ["debug", "release"] {
            let output = case(&format!("m:@\"./facade.mwy\";{separate}"), &files)
                .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = case(
                &format!("m:@\"./facade.mwy\";<T>:{{{repeated}-><int32>}}"),
                &files,
            )
            .command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            if heavy {
                assert_eq!(output.status.code(), Some(1));
                assert!(error.contains("\"code\":\"B001\""), "{error}");
                assert!(error.contains("computed type bootstrap budget"), "{error}");
            } else {
                assert!(output.status.success(), "{error}");
            }
        }
    }
}

#[test]
pub(crate) fn required_boolean_operators_keep_dependency_spans_and_skipped_failures() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"init\");row:{->a<uint8>:255;->b<uint8>:254};->a:row.a+1==0;->b:row.b+2==0";
    let files = [("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")];
    for (value, failed) in [
        ("!m.a", "row.a+1"),
        ("m.a==m.b", "row.a+1"),
        ("m.b!=m.a", "row.b+2"),
        ("false==m.a", "row.a+1"),
    ] {
        let source = format!("m:@\"./facade.mwy\";<T>:{{flag:{value};->flag<>}}");
        let case = case(&source, &files);
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
                    error.contains(&format!("\"start\":{}", data.find(failed).unwrap())),
                    "{error}"
                );
            }
        }
    }
    let case = case(
        "m:@\"./facade.mwy\";<T>:{a:false&&m.a;b:true||m.b;->a<>}",
        &files,
    );
    for profile in ["debug", "release"] {
        let output = case.command("check", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"init\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P002]"));
    }
}

#[test]
pub(crate) fn required_boolean_operators_check_silently_and_keep_initialization_order() {
    let case = case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";<T>:{flag:(a&&true)==b.flag;skipped:false&&a.bad;->flag<>};v<T>:false;d:@\"debug\";d.print(\"entry\");d.print(v)",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->true;->flag:false;->bad:{d.print(\"bad\");->false}",
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
    case.runs(b"data\nbad\na\nb\nentry\nfalse\n");
}
