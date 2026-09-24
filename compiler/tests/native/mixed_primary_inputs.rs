use super::file_modules::case;

#[test]
pub(crate) fn mixed_primary_copies_keep_widths_aliases_and_named_exports() {
    case(
        "m:@\"./data.mwy\";alias:m;copy:alias+0;f:@\"./facade.mwy\";d:@\"debug\";<T>:{n:copy+f.width;-><int32[n]>};v<T>:[3,7];d.print(v[2]);d.print(alias.label);d.print(copy)",
        &[
            ("data.mwy", "base<uint8>:2;->base*2;->label:\"ready\";->row:{->n:9}"),
            ("facade.mwy", "m:@\"./data.mwy\";copy:m+0;->width:copy"),
        ],
    ).runs(b"7\nready\n4\n");
}

#[test]
pub(crate) fn mixed_primary_copies_preserve_checked_integer_failures() {
    for value in ["copy+1", "export.width+1"] {
        let source = format!(
            "m:@\"./data.mwy\";copy:m+0;export:@\"./facade.mwy\";<T>:{{n:{value};-><int32>}}"
        );
        let output = case(
            &source,
            &[
                ("data.mwy", "base<uint8>:255;->base;->label:\"ready\""),
                ("facade.mwy", "m:@\"./data.mwy\";->width:m+0"),
            ],
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E107\""), "{error}");
    }
}

#[test]
pub(crate) fn mixed_primary_copies_require_evidence_and_preserve_runtime_capture_gates() {
    for (source, data, code) in [
        (
            "m:@\"./data.mwy\";copy:m+0;<T>:{n:copy;-><int32>}",
            "d:@\"debug\";->{->4;d.print(\"must not run\")};->label:\"ready\"",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";copy:m+0;f<int32>:(){->copy}",
            "->4;->label:\"ready\"",
            "B001",
        ),
        (
            "row:{->4;->n:2};copy:row+0;<T>:{n:copy;-><int32>}",
            "->4",
            "E211",
        ),
    ] {
        let output = case(source, &[("data.mwy", data)]).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}

#[test]
pub(crate) fn mixed_primary_required_reads_keep_arithmetic_annotations_and_record_identity() {
    case(
        "m:@\"./data.mwy\";alias:m;d:@\"debug\";<T>:{n<uint8>:(alias);step:m+1;mask:(@\"bits\").not(m);-><int32[n+step]>};v<T>:[3,7];<Row>:{->m<>};row<Row>:{->4;->label:\"copy\";->width:2};<U>:{-><int32[m+alias.width]>};u<U>:[9];d.print(v[2]);d.print(row.label);d.print(u[1]);d.print(alias.label);d.print(m+1)",
        &[("data.mwy", "base<uint8>:4;->base;->label:\"ready\";->width<uint8>:2")],
    ).runs(b"7\ncopy\n9\nready\n5\n");
}

#[test]
pub(crate) fn mixed_primary_required_reads_work_in_functions_and_scoped_imports() {
    case(
        "m:@\"./data.mwy\";d:@\"debug\";f<int32>:(){<T>:{n<uint8>:m;-><int32[n]>};v<T>:[7];->v[1]};g<int32>:(){m:@\"./data.mwy\";<T>:{-><int32[m+0]>};v<T>:[9];->v[1]};d.print(f());d.print(g())",
        &[("data.mwy", "base<uint8>:4;->base;->label:\"ready\"")],
    ).runs(b"7\n9\n");
}

#[test]
pub(crate) fn mixed_primary_required_reads_preserve_widths_privacy_and_capture_gates() {
    for (body, code) in [
        ("<T>:{n:m+1;-><int32>}", "E107"),
        ("<T>:{n<uint8>:m;n2:n+1;-><int32>}", "E107"),
        ("<T>:{n<int32>:m;-><int32>}", "E207"),
        ("<T>:{n:m+wide;-><int32>}", "E213"),
        ("<T>:{n:m;-><int32>}", "E211"),
        ("<T>:{-><int32[m]>}", "E104"),
        ("<T>:{n<(m<>)>:m;-><int32>}", "B001"),
        ("<T>:{n:m.private;-><int32>}", "E201"),
        ("<T>:{n:m.label;-><int32>}", "B001"),
        ("f<uint8>:(){<T>:{n<uint8>:m;-><int32>};->m+0}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";wide<uint16>:0;{body}");
        let output = case(
            &source,
            &[("data.mwy", "private<uint8>:255;->private;->label:\"ready\"")],
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{body}: {error}"
        );
    }
}

#[test]
pub(crate) fn mixed_primary_required_reads_reject_unproven_initializers() {
    for data in [
        "d:@\"debug\";->{->4;d.print(\"must not run\")};->label:\"ready\"",
        "value:=4;->value;->label:\"ready\"",
        "f<int32>:(){->4};->f();->label:\"ready\"",
        "|true|->4;->label:\"ready\"",
    ] {
        for binding in ["n:m+0", "n<int32>:m"] {
            let source = format!("m:@\"./data.mwy\";<T>:{{{binding};-><int32>}}");
            let output = case(&source, &[("data.mwy", data)]).command("run", &["--json"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(error.contains("\"code\":\"E211\""), "{data}: {error}");
        }
    }
}

#[test]
pub(crate) fn mixed_primary_imports_separate_named_effects_and_initialize_diamonds_once() {
    let case = case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";d:@\"debug\";<T>:{n<uint8>:a;-><int32[n+b.width]>};v<T>:[7];d.print(\"entry\");d.print(v[1]);d.print(a.label)",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");base<uint8>:4;->base;->label:{d.print(\"label\");->\"ready\"};d.print(\"tail\")",
            ),
            (
                "a.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"a\");->m+0;->label:m.label",
            ),
            (
                "b.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"b\");->width:m+0",
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
    case.runs(b"data\nlabel\ntail\na\nb\nentry\n7\nready\n");
}

#[test]
pub(crate) fn mixed_primary_imports_charge_original_tail_work_through_reexports() {
    let tail = (1..40)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("->{{->4;v0:1;{tail}}};->label:\"ready\"");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m+0;->label:m.label"),
    ];
    let output = case(
        "m:@\"./facade.mwy\";<T>:{n<int32>:m;-><int32[n]>};<U>:{-><int32[m+0]>}",
        &files,
    )
    .command("check", &["--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for values in ["a:m+0;b:m+0", "a<int32>:m;b<int32>:m", "a:copy;b:copy"] {
        let source = format!("m:@\"./facade.mwy\";copy:m+0;<T>:{{{values};-><int32>}}");
        let output = case(&source, &files).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"B001\""), "{error}");
        assert!(error.contains("computed type bootstrap budget"), "{error}");
    }
}

#[test]
pub(crate) fn mixed_primary_imports_preserve_dependency_errors_and_refuse_transitive_effects() {
    for (data, code) in [
        (
            "#é🙂#\nbase<uint8>:255;->{->4;unused<uint8>:base+1};->label:\"ready\"",
            "E107",
        ),
        (
            "d:@\"debug\";->{->4;d.print(\"must not run\")};->label:\"ready\"",
            "E211",
        ),
    ] {
        let case = case(
            "m:@\"./facade.mwy\";<T>:{n<int32>:m;-><int32[n]>}",
            &[
                ("data.mwy", data),
                (
                    "facade.mwy",
                    "m:@\"./data.mwy\";copy:m+0;->copy;->label:m.label",
                ),
            ],
        );
        for action in ["check", "build", "run"] {
            let output = case.command(action, &["--json"]);
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
                    error.contains(&format!("\"start\":{}", data.find("base+1").unwrap())),
                    "{error}"
                );
            }
        }
    }
}

#[test]
pub(crate) fn mixed_primary_imports_keep_runtime_failure_before_dependent_initialization() {
    let case = case(
        "m:@\"./facade.mwy\";<T>:{n<int32>:m;-><int32[n]>};d:@\"debug\";d.print(\"entry must not run\")",
        &[
            (
                "data.mwy",
                "d:@\"debug\";->4;->label:{stop<boolean>:(){->true};d.print(\"init\");|stop()|d.panic(\"stop\");->\"ready\"}",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"facade must not run\");->m+0;->label:m.label",
            ),
        ],
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
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}
