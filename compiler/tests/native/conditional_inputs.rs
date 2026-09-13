use super::Case;

#[test]
pub(crate) fn conditional_record_inputs_select_named_nested_and_composed_fields() {
    for condition in ["true", "!false", "true&&(!false||false)"] {
        let source = format!(
            "d:@\"debug\";row:{{|{condition}|->width<uint8>:4;|false|->width<uint8>:9;|true|->part:{{|true|->n<uint8>:2}}}};copy:{{|{condition}|->row}};n:copy.width;<T>:{{-><int32[n+copy.part.n]>}};v<T>:[7];d.print(v[1]);d.print(copy.width);d.print(copy.part.n)"
        );
        Case::new(&source).runs(b"7\n4\n2\n");
    }
}

#[test]
pub(crate) fn conditional_record_inputs_skip_unevaluated_effects_and_conditions() {
    for body in [
        "|false|d.print(1);->n:4",
        "|false|unused<uint8>:255+1;->n:4",
        "->n:4;|false|d.print(1)",
        "|false|{d.print(1);unused:=2};->n:4",
        "|true| |!false|->n:4",
        "|true| |false|d.print(1);->n:4",
        "|false&&effect()|d.print(1);->n:4",
        "|true| |false&&effect()|d.print(1);->n:4",
        "|true||true| |false|d.print(1);->n:4",
        "|true| |true| |true|->n:4",
        "|true| |true| |true| |true|->n:4",
    ] {
        let source = format!(
            "d:@\"debug\";effect<boolean>:(){{d.print(9);->true}};row:{{{body}}};<T>:{{-><int32[row.n]>}};v<T>:[7];d.print(v[1])"
        );
        Case::new(&source).runs(b"7\n");
    }
    Case::new("d:@\"debug\";effect<boolean>:(){d.print(9);->true};row:{|true||effect()|->n:4};<T>:{-><int32[row.n]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn conditional_record_inputs_reject_selected_effects_and_folded_runtime_reads() {
    for body in [
        "|true|d.print(1);->n:4",
        "->n:4;|true|d.print(1)",
        "|true|unused:=1;->n:4",
        "|true&&effect()|d.print(1);->n:4",
        "|false||effect()|d.print(1);->n:4",
        "|flag|d.print(1);->n:4",
        "|1.0<2.0|unused:1;->n:4",
        "|true|->part:{->n:4};|true|->bad:{->n:1;d.print(1)};->n:4",
    ] {
        let source = format!(
            "d:@\"debug\";flag:=false;effect<boolean>:(){{d.print(9);->true}};row:{{{body}}};<T>:{{n:row.n;-><int32>}}"
        );
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn conditional_record_inputs_retain_selected_sibling_error_spans() {
    for body in [
        "|true|->good<uint8>:4;|true|->bad<uint8>:255+1",
        "->good<uint8>:4;|true|unused<uint8>:255+1;->bad<uint8>:2",
    ] {
        let source = format!(
            "|false|{{row<{{good<uint8>;bad<uint8>}}>:{{{body}}};<T>:{{n:row.good;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("255+1").unwrap());
    }
}

#[test]
pub(crate) fn conditional_record_inputs_bound_active_branch_depth() {
    for (depth, code) in [(31, None), (32, Some("E211"))] {
        let guards = "|true| ".repeat(depth);
        let source = format!("row:{{{guards}->n:4}};<T>:{{n:row.n;-><int32>}}");
        let result = meowy::compile(&source);
        if let Some(code) = code {
            assert_eq!(result.unwrap_err()[0].code, code);
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}

#[test]
pub(crate) fn conditional_record_inputs_charge_selected_work_at_every_forwarded_read() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    for selected in ["true", "false"] {
        let data = format!("row:{{->n:4;|{selected}|unused:{{v0:1;{tail}->1}}}};->row");
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{n:m.n;-><int32[n]>}};"))
            .collect::<String>();
        let repeated = (0..20).map(|id| format!("n{id}:m.n;")).collect::<String>();
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
            if selected == "true" {
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
pub(crate) fn conditional_record_inputs_keep_staging_startup_and_module_export_gates() {
    let case = super::file_modules::case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";d:@\"debug\";part:a.part;copy:{|true|->part};<T>:{-><int32[copy.n+b.part.n]>};v<T>:[7];d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");row:{|true|->part:{|!false|->n<uint8>:4};|false|d.print(\"skipped\")};->row;d.print(\"ready\")",
            ),
            ("a.mwy", "m:@\"./data.mwy\";d:@\"debug\";->m;d.print(\"a\")"),
            ("b.mwy", "m:@\"./data.mwy\";d:@\"debug\";->m;d.print(\"b\")"),
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
    case.runs(b"data\nready\na\nb\n7\n");
    for data in ["|true|->n:4", "row:{|true|->n:4};|true|->row"] {
        let output = super::file_modules::case(
            "m:@\"./data.mwy\";<T>:{n:m.n;-><int32>}",
            &[("data.mwy", data)],
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
    }
}

#[test]
pub(crate) fn conditional_record_inputs_keep_runtime_body_effects_without_required_reads() {
    Case::new("d:@\"debug\";effect<boolean>:(){d.print(\"condition\");->true};row:{|effect()|d.print(\"body\");->n:4;|false|d.print(\"skipped\")};d.print(row.n)")
        .runs(b"condition\nbody\n4\n");
}

#[test]
pub(crate) fn comparison_record_inputs_use_exact_eligible_integer_operands() {
    for condition in [
        "n==4",
        "n!=3",
        "n<5",
        "n<=4",
        "n>3",
        "n>=4",
        "n+1==5&&n>0",
        "!(n<4)",
    ] {
        let source = format!(
            "d:@\"debug\";base<uint8>:4;row:{{n:base;|{condition}|->width<uint8>:4;|!({condition})|->width<uint8>:2}};<T>:{{-><int32[row.width]>}};v<T>:[7];d.print(v[1]);d.print(row.width)"
        );
        Case::new(&source).runs(b"7\n4\n");
    }
    Case::new("d:@\"debug\";base:{->n<uint64>:18446744073709551615};row:{|!(base.n>9223372036854775808)|d.print(99);->width:4};<T>:{-><int32[row.width]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
}

#[test]
pub(crate) fn comparison_record_inputs_preserve_first_predicate_failure_paths() {
    for condition in [
        "n+1==0",
        "(n+1==0)&&effect()",
        "(n+1==0)||effect()",
        "n+1==get()",
    ] {
        let source = format!(
            "effect<boolean>:(){{->true}};get<uint8>:(){{->0}};|false|{{n<uint8>:255;row<{{part<{{good<uint8>}}>}}>:{{|true| |{condition}|->part:{{->good<uint8>:4}}}};copy:row.part;<T>:{{v:copy.good;-><int32>}}}}"
        );
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("n+1").unwrap());
    }
}

#[test]
pub(crate) fn comparison_record_inputs_do_not_admit_runtime_operands() {
    for prefix in [
        "n:=4",
        "get<int32>:(){->4};n:get()",
        "d:@\"debug\";n:{d.print(1);->4}",
    ] {
        let source =
            format!("{prefix};row:{{|n==4|unused:1;->width:4}};<T>:{{n:row.width;-><int32>}}");
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn boolean_record_inputs_keep_aliases_and_function_type_reads() {
    Case::new("d:@\"debug\";ready:false;alias:!ready;row:{|ready|d.print(99);|alias|->n:4;|!alias|->n:2};f<int32>:(){<T>:{-><int32[row.n]>};v<T>:[7];->v[1]};d.print(f())").runs(b"7\n");
}

#[test]
pub(crate) fn boolean_record_inputs_preserve_aliased_and_unused_predicate_failures() {
    for body in [
        "flag:n+1==0;alias:!flag;row<{good<uint8>}>:{|alias|->good<uint8>:4}",
        "row<{good<uint8>}>:{unused:n+1==0;->good<uint8>:4}",
        "flag:n+1==0;row<{good<uint8>}>:{|false| |flag|->good<uint8>:2;|flag|->good<uint8>:4}",
    ] {
        let source = format!("|false|{{n<uint8>:255;{body};<T>:{{v:row.good;-><int32>}}}}");
        let error = meowy::compile(&source).unwrap_err().remove(0);
        assert_eq!(error.code, "E107", "{source}: {error:?}");
        assert_eq!(error.span.start, source.find("n+1").unwrap());
    }
}

#[test]
pub(crate) fn boolean_record_inputs_keep_mutable_effectful_and_capture_boundaries() {
    for prefix in [
        "flag:=false;alias:flag",
        "d:@\"debug\";flag:{d.print(1);->false};alias:flag",
        "f<boolean>:(){->false};flag:f();alias:flag",
    ] {
        let source = format!("{prefix};row:{{|alias|unused:1;->n:4}};<T>:{{n:row.n;-><int32>}}");
        let output = Case::new(&source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
    for (source, code) in [
        ("flag:true;f<boolean>:(){->flag}", "B001"),
        (
            "row:{flag:=true;|flag|unused:1;->n:4};<T>:{n:row.n;-><int32>}",
            "E211",
        ),
        ("row:{->flag:=true;->n:4};<T>:{n:row.n;-><int32>}", "E211"),
        ("flag:true;<T>:{value:=flag;-><int32>}", "B001"),
    ] {
        assert_eq!(
            meowy::compile(source).unwrap_err()[0].code,
            code,
            "{source}"
        );
    }
}

#[test]
pub(crate) fn boolean_record_inputs_keep_scoped_scratch_and_compositions() {
    Case::new("d:@\"debug\";base:{->n<uint8>:4};ready:base.n==4;alias:ready;row:{n:4;ok:n==4;copy:ok;|alias&&copy|->width:4;|!(alias&&copy)|->width:2};part:{|ready|->row;|!ready|->{->width:2}};<T>:{-><int32[part.width]>};v<T>:[7];d.print(v[1]);d.print(part.width)").runs(b"7\n4\n");
}

#[test]
pub(crate) fn boolean_record_inputs_keep_branch_scope_and_check_unused_scratch() {
    Case::new("d:@\"debug\";row:{flag:false;|true|flag:true;|flag|d.print(99);->n:4};<T>:{-><int32[row.n]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
    for source in [
        "f<boolean>:(){->false};row:{->n:4;unused:f()};<T>:{n:row.n;-><int32>}",
        "row:{unused:1.0<2.0;->n:4};<T>:{n:row.n;-><int32>}",
        "d:@\"debug\";row:{flag:true;|flag|d.print(99);->n:4};<T>:{n:row.n;-><int32>}",
    ] {
        let output = Case::new(source).command("run", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("\"code\":\"E211\""), "{source}: {error}");
    }
}

#[test]
pub(crate) fn predicate_record_inputs_charge_transitive_alias_work_only_when_evaluated() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    for condition in ["alias", "false&&alias", "true||alias"] {
        let data = format!(
            "n:{{v0:1;{tail}->4}};flag:n==4;alias:flag;row:{{|{condition}|unused:1;->width:4}};->row"
        );
        let files = [
            ("data.mwy", data.as_str()),
            ("facade.mwy", "m:@\"./data.mwy\";->m"),
        ];
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{n:m.width;-><int32[n]>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("n{id}:m.width;"))
            .collect::<String>();
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
            if condition == "alias" {
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
pub(crate) fn predicate_record_inputs_keep_module_staging_and_boolean_export_gates() {
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";d:@\"debug\";<T>:{-><int32[m.width]>};v<T>:[7];d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->n<uint8>:4;->flag:true",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";flag:m.n==4;alias:flag;row:{local:alias;|local|->width:4;|!local|->width:2};->row;d.print(\"facade\")",
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
    let output = super::file_modules::case(
        "m:@\"./data.mwy\";flag:m.flag;row:{|flag|unused:1;->width:4};<T>:{n:row.width;-><int32>}",
        &[("data.mwy", "|true|->flag:true")],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
}
