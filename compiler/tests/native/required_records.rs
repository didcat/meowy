use super::file_modules::case;

#[test]
pub(crate) fn required_records_support_aliases_fields_and_scoped_function_reads() {
    case(
        "m:@\"./facade.mwy\";<Items>:{record:m.row;part:record.part;alias:part;flag:record.enabled;count<uint8>:{copy:alias;->copy.width};|flag|-><int32[count]>;|!flag|-><string>};v<Items>:[3,7];f<int32>:(){p:@\"./facade.mwy\";<T>:{r:p.row;-><int32[r.part.width]>};v<T>:[9];->v[1]};d:@\"debug\";d.print(v[2]);d.print(f())",
        &[("data.mwy", "private<uint8>:4;->row:{->enabled:true;->part:{->width:private}}"), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"7\n9\n");
}

#[test]
pub(crate) fn required_records_keep_privacy_capture_and_namespace_gates() {
    for (body, code) in [
        ("<T>:{copy:m;-><int32>}", "E211"),
        ("<T>:{copy:m.private;-><int32>}", "E201"),
        (
            "<T>:{copy:m.row;count:copy.enabled;-><int32[count]>}",
            "B001",
        ),
        (
            "f<int32>:(){<T>:{copy:m.row;-><int32[copy.width]>};->m.row.width}",
            "B001",
        ),
        (
            "f<int32>:(row<{width<uint8>}>){<T>:{copy:row;-><int32>};->1}",
            "E211",
        ),
    ] {
        let output = case(
            &format!("m:@\"./data.mwy\";{body}"),
            &[(
                "data.mwy",
                "private:{->width:4};->row:{->enabled:true;->width<uint8>:4}",
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
pub(crate) fn required_records_charge_source_materialization_once_per_binding() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data =
        format!("heavy:{{->4;v0:1;{tail}}};->row:{{->part:{{->width<uint8>:4}};->unused:heavy}}");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m"),
    ];
    for value in ["m.row", "m.row.part"] {
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{r:{value};-><int32>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("r{id}:{value};"))
            .collect::<String>();
        let aliases = (0..20).map(|id| format!("r{id}:copy;")).collect::<String>();
        for profile in ["debug", "release"] {
            for body in [
                separate.clone(),
                format!("<T>:{{copy:{value};{aliases}-><int32>}}"),
            ] {
                let output = case(&format!("m:@\"./facade.mwy\";{body}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            let output = case(
                &format!("m:@\"./facade.mwy\";<T>:{{{repeated}-><int32>}}"),
                &files,
            )
            .command("check", &["--profile", profile, "--json"]);
            assert_eq!(output.status.code(), Some(1));
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"B001\""), "{error}");
            assert!(error.contains("computed type bootstrap budget"), "{error}");
        }
    }
}

#[test]
pub(crate) fn required_records_keep_ancestor_error_spans_and_runtime_failures() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"init\");raw:{->n<uint8>:255};->row:{->part:{->width:4};->bad:raw.n+1}";
    let files = [("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")];
    for value in ["m.row", "m.row.part"] {
        let source = format!("m:@\"./facade.mwy\";<T>:{{r:{value};-><int32>}}");
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
                    error.contains(&format!("\"start\":{}", data.find("raw.n+1").unwrap())),
                    "{error}"
                );
            }
        }
    }
    let case = case(
        "m:@\"./facade.mwy\";<T>:{|false|copy:m.row.part;-><int32>}",
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
pub(crate) fn required_records_preserve_annotations_and_silent_module_startup() {
    let case = case(
        "m:@\"./types.mwy\";v<m.Items>:[3,7];d:@\"debug\";d.print(\"entry\");d.print(v[2])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->row:{->enabled:true;->part:{->width<uint8>:4}};->bad:{d.print(\"bad\");->false}",
            ),
            (
                "types.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"types\");-><Items>:{r<(m.row<>)>:m.row;part:r.part;|r.enabled|-><int32[part.width]>;|!r.enabled|-><string>}",
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
    case.runs(b"data\nbad\ntypes\nentry\n7\n");
}

#[test]
pub(crate) fn required_record_construction_supports_nested_fields_copies_and_matchers() {
    case(
        "source:{->width<uint8>:4};<R>:<{enabled<boolean>;part<{width<uint8>}>}>;<Items>:{r<R>:{->part:source;|part.width>=4|->enabled:true;|part.width<4|->enabled:false};copy:r.part;-><int32[copy.width]>};v<Items>:[3,7];d:@\"debug\";d.print(v[2])",
        &[],
    ).runs(b"7\n");
    case(
        "<R>:<{enabled<boolean>;part<{width<uint8>}>}>;<T>:{r<R>:{->part:{->width:{->4}};->enabled:false};copy:r;|copy.enabled|-><string>;|!copy.enabled|-><int32[copy.part.width]>};v<T>:[9];d:@\"debug\";d.print(v[1])",
        &[],
    ).runs(b"9\n");
}
