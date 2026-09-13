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

#[test]
pub(crate) fn required_record_construction_charges_fields_and_skips_unselected_sources() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("->width:{{->4;v0:1;{tail}}}");
    let files = [("data.mwy", data.as_str())];
    for (body, heavy) in [
        ("->width:m.width", true),
        ("->width:4;unused:m.width", true),
        ("|false|->width:m.width;|true|->width:4", false),
    ] {
        let root = "m:@\"./data.mwy\";<R>:<{width<int32>}>;";
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{r<R>:{{{body}}};-><int32[r.width]>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("r{id}<R>:{{{body}}};"))
            .collect::<String>();
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
pub(crate) fn required_record_construction_keeps_copy_errors_and_runtime_startup() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"init\");raw:{->n<uint8>:255};->width:raw.n+1;->row:{->width:width}";
    let files = [("data.mwy", data)];
    let root = "m:@\"./data.mwy\";<R>:<{part<{width<uint8>}>}>;";
    for body in [
        "->part:m.row",
        "->part:{->width:m.width}",
        "->part:{->width:4};unused:m.width",
    ] {
        let case = case(&format!("{root}<T>:{{r<R>:{{{body}}};-><int32>}}"), &files);
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
        &format!(
            "{root}<T>:{{r<R>:{{|false|->part:m.row;|true|->part:{{->width:4}}}};-><int32[r.part.width]>}}"
        ),
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
pub(crate) fn required_record_construction_exports_types_and_keeps_checks_silent() {
    let case = case(
        "m:@\"./types.mwy\";v<m.Items>:[3,7];d:@\"debug\";d.print(\"entry\");d.print(v[2])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->width<uint8>:4;->enabled:true",
            ),
            (
                "types.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"types\");<R>:<{width<uint8>;enabled<boolean>}>;-><Items>:{r<R>:{->width:m.width;->enabled:m.enabled};|r.enabled|-><int32[r.width]>;|!r.enabled|-><string>}",
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
    case.runs(b"data\ntypes\nentry\n7\n");
    super::file_modules::case("m:@\"./data.mwy\";f<int32>:(){<T>:{r<{width<uint8>}>:{->width:m.width};-><int32[r.width]>};v<T>:[9];->v[1]};d:@\"debug\";d.print(f())", &[("data.mwy", "->width<uint8>:4")]).runs(b"9\n");
}

#[test]
pub(crate) fn required_composition_supports_full_records_aliases_and_selected_sources() {
    for (flag, count) in [("true", 4), ("false", 2)] {
        let source = format!(
            "a:{{->width<uint8>:4;->enabled:true}};b:{{->width<uint8>:2;->enabled:false}};<R>:<{{enabled<boolean>;width<uint8>}}>;<T>:{{flag:{flag};copy:a;r<R>:{{|flag|->copy;|!flag|->b}};n<uint8>:r.width;|r.enabled==flag|-><int32[n]>;|r.enabled!=flag|-><string>}};v<T>:[3,7];d:@\"debug\";d.print(v[2]);expected<int32[{count}]>:v"
        );
        case(&source, &[]).runs(b"7\n");
    }
    case("m:@\"./data.mwy\";<R>:<{part<{width<uint8>}>;enabled<boolean>}>;<T>:{r<R>:{->m.row};|r.enabled|-><string>;|!r.enabled|-><int32[r.part.width]>};v<T>:[9];d:@\"debug\";d.print(v[1])", &[("data.mwy", "->row:{->enabled:false;->part:{->width<uint8>:4}}")]).runs(b"9\n");
}

#[test]
pub(crate) fn required_composition_charges_sources_once_and_reuses_required_aliases() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("heavy:{{->4;v0:1;{tail}}};->part:{{->width:heavy}}");
    let files = [("data.mwy", data.as_str())];
    let root = "m:@\"./data.mwy\";<R>:<{width<int32>}>;";
    let separate = (0..20)
        .map(|id| format!("<T{id}>:{{r<R>:{{->m.part}};-><int32[r.width]>}};"))
        .collect::<String>();
    let aliases = (0..20)
        .map(|id| format!("r{id}<R>:{{->copy}};"))
        .collect::<String>();
    let repeated = (0..20)
        .map(|id| format!("r{id}<R>:{{->m.part}};"))
        .collect::<String>();
    let skipped = (0..20)
        .map(|id| format!("r{id}<R>:{{|false|->m.part;->width:4}};"))
        .collect::<String>();
    for profile in ["debug", "release"] {
        for body in [
            separate.clone(),
            format!("<T>:{{copy:m.part;{aliases}-><int32>}}"),
            format!("<T>:{{{skipped}-><int32>}}"),
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

#[test]
pub(crate) fn required_composition_keeps_ancestor_spans_and_namespace_gates() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"init\");raw:{->n<uint8>:255};private:{->width<uint8>:4};->part:{->width<uint8>:4;->bad:raw.n+1}";
    let files = [("data.mwy", data)];
    for (source, code) in [
        (
            "m:@\"./data.mwy\";<R>:<{width<uint8>}>;<T>:{r<R>:{->m.part};-><int32>}",
            "E107",
        ),
        (
            "m:@\"./data.mwy\";<R>:<{width<uint8>}>;<T>:{r<R>:{->m.private};-><int32>}",
            "E201",
        ),
        (
            "m:@\"./data.mwy\";<R>:<{width<uint8>}>;<T>:{r<R>:{->m};-><int32>}",
            "E211",
        ),
    ] {
        let case = case(source, &files);
        for action in ["check", "build", "run"] {
            for profile in ["debug", "release"] {
                let output = case.command(action, &["--profile", profile, "--json"]);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                let error = String::from_utf8_lossy(&output.stderr);
                assert!(error.contains(&format!("\"code\":\"{code}\"")), "{error}");
                if code == "E107" {
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
    }
}

#[test]
pub(crate) fn required_composition_exports_types_and_preserves_silent_startup() {
    let case = case(
        "m:@\"./types.mwy\";v<m.Items>:[3,7];d:@\"debug\";d.print(\"entry\");d.print(v[2])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->part:{->width<uint8>:4};->bad:{d.print(\"bad\");->false}",
            ),
            (
                "types.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"types\");<R>:<{width<uint8>;enabled<boolean>}>;-><Items>:{r<R>:{->m.part;->enabled:true};-><int32[r.width]>}",
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
    super::file_modules::case("source:{->width<uint8>:4};f<int32>:(){<T>:{r<{part<{width<uint8>}>}>:{->part:{->source}};-><int32[r.part.width]>};v<T>:[9];->v[1]};d:@\"debug\";d.print(f())", &[]).runs(b"9\n");
}

#[test]
pub(crate) fn inline_required_composition_executes_partial_and_nested_sources() {
    for (flag, capacity) in [("true", 4), ("false", 2)] {
        let source = format!(
            "<R>:<{{enabled<boolean>;part<{{width<uint8>}}>}}>;<T>:{{flag:{flag};r<R>:{{|flag|->({{->{{->part:{{->width:4}}}}}});|!flag|->{{->part:{{->width:2}}}};->enabled:true}};-><int32[r.part.width]>}};v<T>:[3,7];d:@\"debug\";d.print(v[2])"
        );
        case(&source, &[]).runs(b"7\n");
        let values = vec!["1"; capacity + 1].join(",");
        let output =
            case(&format!("{source};extra<T>:[{values}]"), &[]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E103\""), "{error}");
    }
}
