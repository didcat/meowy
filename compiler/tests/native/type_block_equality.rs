use super::Case;

#[test]
pub(crate) fn block_type_equality_selects_types_in_both_profiles() {
    Case::new("d:@\"debug\";kind<Type>:<int32>;<T>:{same:({n<uint8>:2;-><int32[n]>})==({-><(kind)[2]>});other:({-><uint8>})!=<int32>;|same&&other|-><int32[2]>};v<T>:[3,7];d.print(v[2])").runs(b"7\n");
    Case::new("d:@\"debug\";f<int32>:(arg<uint8>){<T>:{same:({local:arg<>;->local})==<uint8>;other:<int32> == ({|same|->{-><int32>};|!same|-><boolean>});|other|-><int32>};v<T>:9;->v};d.print(f(3))").runs(b"9\n");
}

#[test]
pub(crate) fn block_type_equality_preserves_imported_identity_and_startup() {
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";source:@\"./types.mwy\";<T>:{same:({kind<Type>:m.kind;->kind})==({->source.kind});|same|-><int32>};v<T>:7;d:@\"debug\";d.print(v)",
        &[
            (
                "types.mwy",
                "d:@\"debug\";d.print(\"types\");private<Type>:<uint8>;->kind<Type>:<int32>",
            ),
            (
                "facade.mwy",
                "m:@\"./types.mwy\";d:@\"debug\";d.print(\"facade\");->kind<Type>:m.kind",
            ),
        ],
    );
    for profile in ["debug", "release"] {
        let output = case.command("check", &["--profile", profile]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
    case.runs(b"types\nfacade\n7\n");
    let output = super::file_modules::case(
        "m:@\"./types.mwy\";<T>:{same:({->m.private})==<uint8>;-><int32>}",
        &[("types.mwy", "private<Type>:<uint8>;->kind<Type>:<int32>")],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E201\""));
}

#[test]
pub(crate) fn block_type_equality_keeps_first_source_error_through_facades() {
    let data = "#é🙂#\nraw:{->n<uint8>:255};->bad:raw.n+1";
    for (expr, original) in [
        ("({-><int32[m.bad]>})==({-><int32[1/0]>})", true),
        ("({-><int32[1/0]>})!=({-><int32[m.bad]>})", false),
        ("<int32> == ({-><int32>;tail:m.bad})", true),
        ("({-><int32>;tail:1/0})==({-><int32[m.bad]>})", false),
    ] {
        let entry = format!("m:@\"./facade.mwy\";<T>:{{flag:{expr};-><int32>}}");
        let case = super::file_modules::case(
            &entry,
            &[("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")],
        );
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(error.contains("\"code\":\"E107\""), "{expr}: {error}");
            let start = if original {
                data.find("raw.n+1").unwrap()
            } else {
                entry.find("1/0").unwrap()
            };
            assert!(
                error.contains(&format!("\"start\":{start}")),
                "{expr}: {error}"
            );
            if original {
                assert!(
                    error.contains(&format!(
                        "\"path\":\"{}\"",
                        case.path.join("data.mwy").display()
                    )),
                    "{error}"
                );
            }
        }
    }
}

#[test]
pub(crate) fn block_type_equality_retains_input_costs_and_independent_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("->width:{{->4;v0:1;{tail}}}");
    for (expr, accepted) in [
        ("({-><int32[m.width]>})==({-><int32[m.width]>})", false),
        (
            "true||(({-><int32[m.width]>})==({-><int32[m.width]>}))",
            true,
        ),
    ] {
        let roots = (0..20)
            .map(|id| format!("kind{id}<Type>:{{flag:{expr};-><int32>}};"))
            .collect::<String>();
        let nested = (0..20)
            .map(|id| format!("flag{id}:{expr};"))
            .collect::<String>();
        for (body, accepted) in [
            (roots, true),
            (format!("kind<Type>:{{{nested}-><int32>}}"), accepted),
        ] {
            let case = super::file_modules::case(
                &format!("m:@\"./data.mwy\";{body}"),
                &[("data.mwy", &data)],
            );
            let output = case.command("check", &["--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.success(), accepted, "{expr}: {error}");
            if !accepted {
                assert!(error.contains("\"code\":\"B001\""), "{error}");
                assert!(error.contains("budget exhausted"), "{error}");
            }
        }
    }
}

#[test]
pub(crate) fn block_type_equality_keeps_runtime_types_and_helpers_unavailable() {
    for source in [
        "flag:({-><int32>})==({-><int32>})",
        "kind<Type>:<int32>;flag:kind==({-><int32>})",
        "make<Type>:(){-><int32>};<T>:{flag:({->make()})==<int32>;-><int32>}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        let error = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1));
        assert!(error.contains("\"code\":\"B001\""), "{source}: {error}");
    }
}
