use super::file_modules::case;

#[test]
pub(crate) fn boolean_modules_preserve_values_aliases_and_named_reexports() {
    for enabled in [false, true] {
        let data = format!("private:{enabled};->enabled:private;->other:!private");
        case(
            "m:@\"./outer.mwy\";alias:m.enabled;n:{ok:alias!=m.other;ready:{->ok&&m.copy};|ready|->4;|!ready|->2};<T>:{-><int32[n]>};v<T>:[7];d:@\"debug\";d.print(v[1]);d.print(n)",
            &[("data.mwy", &data), ("facade.mwy", "m:@\"./data.mwy\";->m;->copy:m.enabled"), ("outer.mwy", "m:@\"./facade.mwy\";->m")],
        ).runs(format!("7\n{}\n", if enabled { 4 } else { 2 }).as_bytes());
    }
}

#[test]
pub(crate) fn boolean_modules_preserve_initializer_and_module_boundaries() {
    for (data, code) in [
        ("d:@\"debug\";->enabled:{->true;d.print(9)}", "E211"),
        ("get<boolean>:(){->true};->enabled:get()", "E211"),
        ("value:=true;->enabled:value", "E211"),
        ("|true|->enabled:true", "E211"),
        ("->enabled:=true", "B001"),
    ] {
        let output = case(
            "m:@\"./data.mwy\";n:{unused:m.enabled;->4};<T>:{v:n;-><int32>}",
            &[("data.mwy", data)],
        )
        .command("run", &["--json"]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{data}: {error}"
        );
    }
    for (source, data, code) in [
        (
            "m:@\"./data.mwy\";flag:m.private",
            "private:true;->enabled:private",
            "E201",
        ),
        (
            "m:@\"./data.mwy\";f<boolean>:(){->m.enabled}",
            "->enabled:true",
            "B001",
        ),
        (
            "m:@\"./data.mwy\";n:{unused:!m;->4};<T>:{v:n;-><int32>}",
            "->true",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";n:{copy:{->m};->4};<T>:{v:n;-><int32>}",
            "->enabled:true",
            "E211",
        ),
        (
            "m:@\"./data.mwy\";<T>:{flag:m.enabled;-><int32>}",
            "->enabled:true",
            "B001",
        ),
    ] {
        let output = case(source, &[("data.mwy", data)]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {error}"
        );
    }
}

#[test]
pub(crate) fn boolean_modules_charge_forwarded_scalar_and_record_work_on_each_read() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data =
        format!("private:{{->true;v0:1;{tail}}};->enabled:private;->row:{{->enabled:private}}");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m;->copy:m.enabled"),
        ("outer.mwy", "m:@\"./facade.mwy\";->m"),
    ];
    for (value, heavy) in [
        ("m.enabled", true),
        ("m.copy", true),
        ("m.row.enabled", true),
        ("alias", true),
        ("false&&m.enabled", false),
    ] {
        let root = format!("m:@\"./outer.mwy\";alias:m.enabled;n:{{unused:{value};->4}};");
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
pub(crate) fn boolean_modules_keep_independent_exports_and_silent_staging() {
    let case = case(
        "m:@\"./facade.mwy\";d:@\"debug\";n:{flag:m.copy==m.enabled;|flag|->4;|!flag|->2};<T>:{-><int32[n]>};v<T>:[7];d.print(\"entry\");d.print(v[1])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->enabled:true;->bad:{d.print(\"bad\");->false}",
            ),
            (
                "facade.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";->m;->copy:m.enabled;d.print(\"facade\")",
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
    case.runs(b"data\nbad\nfacade\nentry\n7\n");
}

#[test]
pub(crate) fn boolean_modules_report_retained_errors_at_the_original_dependency() {
    let data = "#é#\nd:@\"debug\";d.print(\"must not run\");record:{->width<uint8>:255};->enabled:record.width+1==0";
    let case = case(
        "m:@\"./facade.mwy\";n:{unused:m.enabled;->4};<T>:{v:n;-><int32>}",
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
                error.contains(&format!(
                    "\"start\":{}",
                    data.find("record.width+1").unwrap()
                )),
                "{error}"
            );
        }
    }
}
