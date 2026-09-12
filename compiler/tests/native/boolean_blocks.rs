use super::Case;

#[test]
pub(crate) fn boolean_blocks_keep_nested_bindings_aliases_and_tail_work() {
    for block in [
        "{->true}",
        "{base<uint8>:2;test:{step:base+2;->step==4};->test;unused:base+1}",
        "{flag:true;alias:!flag;->!alias;unused:{->false}}",
    ] {
        let source = format!(
            "d:@\"debug\";flag:{block};alias:flag;row:{{|alias|->n:4;|!alias|->n:2}};<T>:{{-><int32[row.n]>}};v<T>:[7];d.print(v[1]);d.print(row.n)"
        );
        Case::new(&source).runs(b"7\n4\n");
    }
    Case::new("d:@\"debug\";row:{flag:{n:4;->n==4};|flag|->width:4;|!flag|->width:2};<T>:{-><int32[row.width]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn boolean_blocks_retain_first_failures_before_and_after_the_primary() {
    for block in [
        "{bad<uint8>:255+1;->true}",
        "{->true;bad<uint8>:255+1}",
        "{flag<boolean>:{->true;bad<uint8>:255+1};->flag}",
        "{bad<uint8>:255+1;unused:get();->true}",
    ] {
        let source = format!(
            "get<boolean>:(){{->true}};|false|{{flag<boolean>:{block};alias:!flag;row<{{part<{{n<uint8>}}>}}>:{{|alias|->part:{{->n<uint8>:4}}}};copy:row.part;<T>:{{n:copy.n;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("255+1").unwrap());
    }
}

#[test]
pub(crate) fn boolean_blocks_keep_effects_mutation_calls_and_unsupported_record_scratch_gated() {
    for block in [
        "{d.print(1);->true}",
        "{->true;d.print(1)}",
        "{unused:=1;->true}",
        "{->true;unused:\"x\"}",
        "{unused:{->n:=4};->true}",
        "{|get()|unused:1;->true}",
        "{->get()}",
        "{->true;unused:get()}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{->true}};flag:{block};row:{{|flag|unused:1;->n:4}};<T>:{{n:row.n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn boolean_blocks_preserve_integer_block_and_required_scratch_boundaries() {
    for (source, code) in [
        ("n:{flag:=true;->4};<T>:{n:n;-><int32>}", "E211"),
        ("flag:{->true};<T>:{flag:flag;-><int32>}", "B001"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
    Case::new("d:@\"debug\";flag:{->true;d.print(\"tail\")};|flag|d.print(\"body\")")
        .runs(b"tail\nbody\n");
}

#[test]
pub(crate) fn boolean_blocks_charge_unused_tail_work_at_every_forwarded_read() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let separate = (0..20)
        .map(|id| format!("<T{id}>:{{n:m.width;-><int32[n]>}};"))
        .collect::<String>();
    let repeated = (0..20)
        .map(|id| format!("n{id}:m.width;"))
        .collect::<String>();
    for heavy in [false, true] {
        let block = if heavy {
            format!("{{->true;v0:1;{tail}}}")
        } else {
            "{->true}".into()
        };
        let data = format!("flag:{block};alias:flag;row:{{|alias|unused:1;->width:4}};->row");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        for profile in ["debug", "release"] {
            let output =
                super::file_modules::case(&format!("m:@\"./facade.mwy\";{separate}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = super::file_modules::case(
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
pub(crate) fn boolean_blocks_keep_imported_integer_evidence_and_silent_staging() {
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{-><int32[m.width]>};v<T>:[7];d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->width<uint8>:4",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";flag:{n:m.width;ready:n==4;->ready;unused:n+1};row:{|flag|->width:4;|!flag|->width:2};->row;d.print(\"facade\")",
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
    case.runs(b"data\nfacade\n7\n");
}

#[test]
pub(crate) fn boolean_blocks_keep_inline_predicates_and_skipped_block_effects() {
    for condition in [
        "{->true}",
        "true||{d.print(99);->false}",
        "!false||{d.print(99);->true}",
    ] {
        let source = format!(
            "d:@\"debug\";row:{{|{condition}|unused:1;->n:4}};<T>:{{-><int32[row.n]>}};v<T>:[7];d.print(v[1])"
        );
        Case::new(&source).runs(b"7\n");
    }
}

#[test]
pub(crate) fn branching_boolean_blocks_select_one_primary_and_preserve_scope() {
    for (pick, width) in [("true", 4), ("false", 2)] {
        let source = format!(
            "d:@\"debug\";pick:{pick};flag:{{|pick|->true;|!pick|->false;unused:4}};row:{{|flag|->width:4;|!flag|->width:2}};<T>:{{-><int32[row.width]>}};v<T>:[7];d.print(v[1]);d.print(row.width)"
        );
        Case::new(&source).runs(format!("7\n{width}\n").as_bytes());
    }
    Case::new("d:@\"debug\";flag:{ready:false;|true|ready:true;|ready|->false;|!ready|->true};row:{|flag|->width:4;|!flag|->width:2};<T>:{-><int32[row.width]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
    for (source, code) in [
        ("flag:{|true|->true;|true|->false}", "E205"),
        ("flag<boolean>:{|false|->true}", "E204"),
        ("flag:{|true|temp:4;->temp==4}", "E201"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn branching_boolean_blocks_skip_effects_and_failures_only_when_unselected() {
    for block in [
        "{|false|d.print(9);->true}",
        "{->true;|false|d.print(9)}",
        "{|true| |false|d.print(9);->true}",
        "{|false|unused:=1;->true}",
        "{|false|bad<uint8>:255+1;->true}",
        "{|true||get()|->true}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{d.print(9);->true}};flag:{block};row:{{|flag|unused:1;->n:4}};<T>:{{-><int32[row.n]>}};v<T>:[7];d.print(v[1])"
        );
        Case::new(&source).runs(b"7\n");
    }
    for block in [
        "{|true|d.print(9);->true}",
        "{->true;|true|d.print(9)}",
        "{|true|unused:=1;->true}",
        "{|get()|unused:1;->true}",
        "{|true|{unused:1};->true}",
    ] {
        let source = format!(
            "d:@\"debug\";get<boolean>:(){{d.print(9);->true}};flag:{block};row:{{|flag|unused:1;->n:4}};<T>:{{n:row.n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn branching_boolean_blocks_retain_first_predicate_body_and_tail_errors() {
    for block in [
        "{|n+1==0|->true}",
        "{|true|bad<uint8>:255+1;->true}",
        "{->true;|true|bad<uint8>:255+1}",
        "{|true| |true|bad<uint8>:255+1;->true}",
        "{|true|->n+1==0}",
    ] {
        let source = format!(
            "|false|{{n<uint8>:255;flag<boolean>:{block};row<{{part<{{n<uint8>}}>}}>:{{|flag|->part:{{->n<uint8>:4}}}};copy:row.part;<T>:{{n:copy.n;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        let start = source.find("n+1").or_else(|| source.find("255+1")).unwrap();
        assert_eq!(error.span.start, start);
    }
}

#[test]
pub(crate) fn branching_boolean_blocks_charge_conditions_and_selected_tail_work() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let separate = (0..20)
        .map(|id| format!("<T{id}>:{{n:m.width;-><int32[n]>}};"))
        .collect::<String>();
    let repeated = (0..20)
        .map(|id| format!("n{id}:m.width;"))
        .collect::<String>();
    for (body, heavy) in [
        (format!("->true;|true|unused:{{v0:1;{tail}->1}}"), true),
        (format!("->true;|false|unused:{{v0:1;{tail}->1}}"), false),
        (format!("->true;|{{v0:1;{tail}->false}}|unused:1"), true),
    ] {
        let data = format!("flag:{{{body}}};row:{{|flag|unused:1;->width:4}};->row");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        for profile in ["debug", "release"] {
            let output =
                super::file_modules::case(&format!("m:@\"./facade.mwy\";{separate}"), &files)
                    .command("check", &["--profile", profile, "--json"]);
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let output = super::file_modules::case(
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
pub(crate) fn branching_boolean_blocks_preserve_module_staging_and_startup_order() {
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{-><int32[m.width]>};v<T>:[7];d.print(\"entry\");d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->width<uint8>:4",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";flag:{n:m.width;pick:n==4;|pick|->true;|!pick|->false;|false|d.print(\"skipped\");unused:n+1};row:{|flag|->width:4;|!flag|->width:2};->row;d.print(\"facade\")",
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
