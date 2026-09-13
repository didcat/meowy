use super::file_modules::case;

#[test]
pub(crate) fn required_scalar_blocks_construct_lists_with_scoped_matchers() {
    case(
        "<Items>:{ready<boolean>:{flag:false;->!flag};count<uint8>:{base<uint8>:{->2};|ready|->{->base*2};|!ready|->2;unused:7};-><int32[count]>};v<Items>:[3,7];d:@\"debug\";d.print(v[2])",
        &[],
    ).runs(b"7\n");
    case(
        "<T>:{<Byte>:<uint8>;n<Byte>:{inner:{|true|-><int32>};copy<Byte>:{->4};->copy};flag<boolean>:{|n>=4|->false;|n<4|->true};|flag|-><string>;|!flag|-><int32[n]>};v<T>:[9];d:@\"debug\";d.print(v[1])",
        &[],
    ).runs(b"9\n");
}

#[test]
pub(crate) fn required_scalar_blocks_read_module_inputs_without_runtime_captures() {
    case(
        "m:@\"./data.mwy\";f<int32>:(){<T>:{count<uint8>:{->m.width};flag<boolean>:{->m.enabled};|flag|-><int32[count]>;|!flag|-><string>};v<T>:[7];->v[1]};d:@\"debug\";d.print(f())",
        &[("data.mwy", "->width<uint8>:4;->enabled:true")],
    ).runs(b"7\n");
    for source in [
        "f<int32>:(v<uint8>){<T>:{n<uint8>:{->v};-><int32[n]>};->1}",
        "mutable:=true;<T>:{n<boolean>:{->mutable};->n<>}",
    ] {
        let output = case(source, &[]).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E211\""));
    }
}

#[test]
pub(crate) fn required_scalar_blocks_charge_sources_and_tails_per_root() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!(
        "private:{{->4;v0:1;{tail}}};->width:private;->flag:private>0;->row:{{->n:private}}"
    );
    let files = [("data.mwy", data.as_str())];
    for (ty, body, heavy) in [
        ("int32", "->m.width", true),
        ("boolean", "->m.flag", true),
        ("int32", "->{->m.row.n}", true),
        ("int32", "->4;unused:m.width", true),
        ("int32", "|false|unused:m.width;->4", false),
    ] {
        let separate = (0..20)
            .map(|id| format!("<T{id}>:{{n<{ty}>:{{{body}}};-><int32>}};"))
            .collect::<String>();
        let repeated = (0..20)
            .map(|id| format!("v{id}<{ty}>:{{{body}}};"))
            .collect::<String>();
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
pub(crate) fn required_scalar_blocks_keep_dependency_errors_and_skipped_reads() {
    let data =
        "#é🙂#\nd:@\"debug\";d.print(\"init\");row:{->n<uint8>:255};->bad:row.n+1;->flag:bad==0";
    let files = [("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")];
    for binding in [
        "n<uint8>:{->m.bad}",
        "n<boolean>:{->m.flag}",
        "n<uint8>:{->4;unused:m.bad}",
    ] {
        let source = format!("m:@\"./facade.mwy\";<T>:{{{binding};-><int32>}}");
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
        "m:@\"./facade.mwy\";<T>:{n<uint8>:{|false|->m.bad;->4};-><int32[n]>}",
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
pub(crate) fn required_scalar_blocks_export_types_without_executing_module_effects() {
    let case = case(
        "m:@\"./types.mwy\";v<m.Items>:[3,7];d:@\"debug\";d.print(\"entry\");d.print(v[2])",
        &[
            (
                "data.mwy",
                "d:@\"debug\";d.print(\"data\");->width<uint8>:4;->enabled:true;->bad:{d.print(\"bad\");->false}",
            ),
            (
                "types.mwy",
                "m:@\"./data.mwy\";d:@\"debug\";d.print(\"types\");-><Items>:{ready<boolean>:{->m.enabled};count<uint8>:{|ready|->{->m.width};|!ready|->2};-><int32[count]>}",
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
    super::file_modules::case(
        "<T>:{minimum<int8>:{->-128};->minimum<>};v<T>:-128;d:@\"debug\";d.print(v)",
        &[],
    )
    .runs(b"-128\n");
}
