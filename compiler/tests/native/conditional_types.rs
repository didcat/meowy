use super::file_modules::case;

#[test]
pub(crate) fn conditional_types_select_scalars_records_and_nested_lists() {
    for (condition, value, output) in [
        ("true", "7", "7\n"),
        ("false", "\"selected\"", "selected\n"),
    ] {
        case(&format!("<T>:{{flag:{condition};|flag|-><int32>;|!flag|-><string>}};v<T>:{value};d:@\"debug\";d.print(v)"), &[]).runs(output.as_bytes());
    }
    case(
        "enabled:true;<Row>:{|enabled|->{<Number>:<int32>;-><{n<Number>}>};|!enabled|-><string>};<Items>:{size<uint8>:4;|size>=4|-><Row[size]>;|size<4|-><Row[2]>};v<Items>:[{->n:3},{->n:7}];d:@\"debug\";d.print(v[2].n)",
        &[],
    ).runs(b"7\n");
}

#[test]
pub(crate) fn conditional_types_keep_function_required_reads_and_runtime_capture_gates() {
    case(
        "m:@\"./data.mwy\";f<int32>:(){<T>:{|m.width>=4|-><int32>;|m.width<4|-><string>};v<T>:7;->v};d:@\"debug\";d.print(f())",
        &[("data.mwy", "->width<uint8>:4")],
    ).runs(b"7\n");
    let output = case(
        "m:@\"./data.mwy\";f<uint8>:(){<T>:{|m.width>=4|-><int32>};->m.width}",
        &[("data.mwy", "->width<uint8>:4")],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
}

#[test]
pub(crate) fn conditional_types_charge_conditions_and_selected_bodies_per_root() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("->flag:{{->false;v0:1;{tail}}}");
    let files = [("data.mwy", data.as_str())];
    for (stmt, heavy) in [
        ("|m.flag|unused:0;", true),
        ("|true|unused:m.flag;", true),
        ("|false|unused:m.flag;", false),
        ("|false| |m.flag|unused:0;", false),
    ] {
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{{stmt}-><int32>}};"))
            .collect::<String>();
        let repeated = stmt.repeat(20);
        for profile in ["debug", "release"] {
            let output = case(&format!("m:@\"./data.mwy\";{separate}"), &files)
                .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = case(
                &format!("m:@\"./data.mwy\";<T>:{{{repeated}-><int32>}}"),
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
pub(crate) fn conditional_types_keep_condition_body_and_tail_dependency_errors() {
    let data =
        "#é🙂#\nd:@\"debug\";d.print(\"init\");row:{->n<uint8>:255};->bad:row.n+1;->flag:bad==0";
    let files = [("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")];
    for body in [
        "|m.flag|-><int32>",
        "|true|-><int32[m.bad]>",
        "-><int32>;|m.flag|unused:0",
    ] {
        let source = format!("m:@\"./facade.mwy\";<T>:{{{body}}}");
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
                    error.contains(&format!("\"start\":{}", data.find("row.n+1").unwrap())),
                    "{error}"
                );
            }
        }
    }
    let case = case(
        "m:@\"./facade.mwy\";<T>:{|false| |m.flag|-><Missing>;-><int32>}",
        &files,
    );
    for profile in ["debug", "release"] {
        let checked = case.command("check", &["--profile", profile]);
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
        assert!(checked.stdout.is_empty());
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"init\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P002]"));
    }
}

#[test]
pub(crate) fn conditional_types_export_selected_shapes_without_running_initializers() {
    let case = case(
        "m:@\"./facade.mwy\";v<m.Items>:[{->n:3},{->n:7}];d:@\"debug\";d.print(\"entry\");d.print(v[2].n)",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->enabled:true;->width<uint8>:4;->bad:{d.print(\"bad\");->false}",
            ),
            (
                "types.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"types\");-><Row>:{|m.enabled|->{<Number>:<int32>;-><{n<Number>}>};|!m.enabled|-><string>};-><Items>:{|m.width>=4|-><Row[m.width]>;|m.width<4|-><Row[2]>}",
            ),
            (
                "facade.mwy",
                "m:@\"./types.mwy\";d:@\"debug\";d.print(\"facade\");-><Items>:<m.Items>",
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
    case.runs(b"data\nbad\ntypes\nfacade\nentry\n7\n");
}
