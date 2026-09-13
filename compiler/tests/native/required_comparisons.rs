use super::file_modules::case;

#[test]
pub(crate) fn required_comparisons_support_arithmetic_fields_primaries_and_functions() {
    case(
        "m:@\"./facade.mwy\";<T>:{a:m.width+1==5;b:1+2<m;c:~m.width>=250;flag:(a&&b)||c;->flag<>};v<T>:false;f<boolean>:(){p:@\"./facade.mwy\";<U>:{a:p>0;b:p.row.n<=4;c:p+0==4;->c<>};v<U>:true;->v};d:@\"debug\";d.print(v);d.print(f());d.print(m.label)",
        &[("data.mwy", "base<uint8>:4;->base;->width:base;->row:{->n:base};->label:\"ready\""), ("facade.mwy", "m:@\"./data.mwy\";->m")],
    ).runs(b"false\ntrue\nready\n");
    case("value<int8>:-128;<T>:{a:-128==value;b:value<=127;c:(value+1)>value;->c<>};v<T>:true;d:@\"debug\";d.print(v)", &[]).runs(b"true\n");
}

#[test]
pub(crate) fn required_comparisons_check_skipped_widths_and_keep_evidence_gates() {
    for (body, code) in [
        ("<T>:{flag:m.width==wide;->flag<>}", "E213"),
        ("<T>:{flag:false&&(m.width==256);->flag<>}", "E216"),
        ("<T>:{flag:true||(256<m.width);->flag<>}", "E216"),
        ("<T>:{flag:-1<m.width;->flag<>}", "E222"),
        ("<T>:{flag:m.width+252==0;->flag<>}", "E107"),
        ("<T>:{flag:m.bad==0;->flag<>}", "E211"),
        ("<T>:{flag:false&&(missing==0);->flag<>}", "E201"),
        ("<T>:{flag:m==m;->flag<>}", "B001"),
        ("<T>:{flag:1.0<2.0;->flag<>}", "B001"),
        ("f<uint8>:(){<T>:{flag:m>0;->flag<>};->m+0}", "B001"),
    ] {
        let source = format!("m:@\"./data.mwy\";wide<uint16>:4;{body}");
        let output = case(
            &source,
            &[(
                "data.mwy",
                "d:@\"debug\";base<uint8>:4;->base;->width:base;->bad:{d.print(9);->1}",
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
    case("f<boolean>:(runtime<uint8>){<T>:{a:false&&(runtime>0);b:true||(runtime+1==0);->a<>};v<T>:false;->v};d:@\"debug\";d.print(f(255))", &[]).runs(b"false\n");
}

#[test]
pub(crate) fn required_comparisons_charge_repeated_inputs_and_reset_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data =
        format!("private:{{->4;v0:1;{tail}}};->private;->width:private;->row:{{->n:private}}");
    let files = [
        ("data.mwy", data.as_str()),
        ("facade.mwy", "m:@\"./data.mwy\";->m"),
    ];
    for (value, heavy) in [
        ("m>0", true),
        ("m.width==m.width", true),
        ("m.row.n+1>=5", true),
        ("false&&(m.width>0)", false),
        ("true||(m.row.n+1==0)", false),
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
pub(crate) fn required_comparisons_keep_dependency_errors_and_runtime_initialization() {
    let data = "#é🙂#\nd:@\"debug\";d.print(\"init\");row:{->a<uint8>:255;->b<uint8>:254};->a:row.a+1;->b:row.b+2";
    let files = [("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")];
    for (value, failed) in [
        ("m.a<m.b", "row.a+1"),
        ("m.b>=m.a", "row.b+2"),
        ("1==m.a", "row.a+1"),
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
        "m:@\"./facade.mwy\";<T>:{flag:false&&(m.a>0);->flag<>}",
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
pub(crate) fn required_comparisons_check_silently_and_keep_diamond_startup() {
    let case = case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";<T>:{flag:a+1>b.width;skip:false&&(a.bad>0);->flag<>};v<T>:false;d:@\"debug\";d.print(\"entry\");d.print(v)",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->4;->width:4;->bad:{d.print(\"bad\");->1}",
            ),
            ("a.mwy", "m:@\"./data.mwy\";d:@\"debug\";d.print(\"a\");->m"),
            (
                "b.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"b\");->width:m.width",
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

#[test]
pub(crate) fn integer_block_comparisons_execute_selected_and_short_circuited_results() {
    for (op, capacity) in [
        ("==", 2),
        ("!=", 4),
        ("<", 4),
        (">", 2),
        ("<=", 4),
        (">=", 2),
    ] {
        let source = format!(
            "<T>:{{flag:({{->2}}){op}({{->3}});skip:false&&(({{->missing()}})==({{->1/0}}));|flag|-><int32[4]>;|!flag|-><int32[2]>}};v<T>:[3,7];d:@\"debug\";d.print(v[2])"
        );
        case(&source, &[]).runs(b"7\n");
        let values = vec!["1"; capacity + 1].join(",");
        let output =
            case(&format!("{source};extra<T>:[{values}]"), &[]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E103\""));
    }
}
