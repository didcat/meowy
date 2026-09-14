use super::Case;

#[test]
pub(crate) fn type_subtraction_executes_constructed_types_and_static_queries() {
    Case::new("d:@\"debug\";<Maybe>:<int32><null>;kind<Type>:<Maybe>!<null>;<Items>:{same:kind==<int32>;|same|-><(kind)[2]>};v<Items>:[3,7];d.print(v[2]);f<int32>:(arg<uint8>){<T>:arg<>!<null>;local<T>:arg;->7};d.print(f(2))").runs(b"7\n7\n");
    Case::new("d:@\"debug\";m:@\"memory\";kind<Type>:<m.AllocationFailure><null>!<null>;<T>:{same:kind==m.AllocationFailure;empty:kind!<m.AllocationFailure> == <never>;|same&&empty|-><int32>};v<T>:9;d.print(v)").runs(b"9\n");
}

#[test]
pub(crate) fn type_subtraction_preserves_facades_and_discovers_removed_operand_imports() {
    super::file_modules::case(
        "m:@\"./facade.mwy\";empty<Type>:<never>!<((@\"./removed.mwy\").kind)>;<T>:{same:m.kind==<int32>;gone:empty==<never>;|same&&gone|-><int32>};v<T>:7;d:@\"debug\";d.print(v)",
        &[
            ("types.mwy", "d:@\"debug\";d.print(\"types\");->kind<Type>:<int32><null>"),
            ("facade.mwy", "m:@\"./types.mwy\";d:@\"debug\";d.print(\"facade\");->kind<Type>:m.kind!<null>"),
            ("removed.mwy", "d:@\"debug\";d.print(\"removed\");->kind<Type>:<boolean>"),
        ],
    ).runs(b"types\nfacade\nremoved\n7\n");
    let output = super::file_modules::case(
        "m:@\"./types.mwy\";kind<Type>:m.private!<null>",
        &[(
            "types.mwy",
            "private<Type>:<int32><null>;->kind<Type>:<int32>",
        )],
    )
    .command("check", &["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E201\""));
}

#[test]
pub(crate) fn type_subtraction_preserves_first_errors_and_reads_removed_types_for_empty_sets() {
    let data = "#é🙂#\nraw:{->n<uint8>:255};->bad:raw.n+1";
    for (expr, original) in [
        ("<int32[m.bad]>!<int32[1/0]>", true),
        ("<int32[1/0]>!<int32[m.bad]>", false),
        ("<never>!<int32[m.bad]>", true),
        ("({-><int32>;tail:1/0})!<int32[m.bad]>", false),
    ] {
        let entry = format!("m:@\"./facade.mwy\";kind<Type>:{expr}");
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
            assert!(error.contains(&format!("\"start\":{start}")), "{error}");
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
pub(crate) fn type_subtraction_retains_repeated_input_work_and_resets_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("->width:{{->4;v0:1;{tail}}}");
    for (expr, accepted) in [
        ("<int32[m.width]>!<int32[m.width]> == <never>", false),
        ("true||(<int32[m.width]>!<int32[m.width]> == <never>)", true),
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
pub(crate) fn type_subtraction_keeps_runtime_results_and_helpers_gated() {
    for source in [
        "kind<Type>:<int32><null>;value:kind!<null>",
        "value:({-><int32><null>})!<null>",
        "make<Type>:(){-><int32><null>};kind<Type>:make()!<null>",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        let error = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1));
        assert!(error.contains("\"code\":\"B001\""), "{source}: {error}");
    }
}
