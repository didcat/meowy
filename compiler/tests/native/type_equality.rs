use super::Case;

#[test]
pub(crate) fn type_equality_selects_normalized_types_in_both_profiles() {
    Case::new("d:@\"debug\";c:@\"core\";m:@\"memory\";kind<Type>:<int32>;<T>:{same:kind==c.int32;record:<{a<int32>;b<boolean>}> == <{b<boolean>;a<int32>}>;union:<int32><null><never> == <null><int32>;nominal:m.AllocationFailure==<m.AllocationFailure>;other:m.Allocator!=m.AllocationFailure;|same&&record&&union&&nominal&&other|-><int32[4]>};v<T>:[7];d.print(v[1])").runs(b"7\n");
    Case::new("d:@\"debug\";f<int32>:(arg<uint8>){<T>:{same:arg<> == <uint8>;other:<({n:2;-><int32[n]>})> == <int32[2]>;skip:true||(<int32[1/0]> == <Missing>);|same&&other&&skip|-><int32>};v<T>:9;->v};d.print(f(3))").runs(b"9\n");
}

#[test]
pub(crate) fn type_equality_leaves_runtime_values_and_type_helpers_gated() {
    for source in [
        "flag:<int32> == <int32>",
        "kind<Type>:<int32>;flag:kind==kind",
        "make<Type>:(){-><int32>};<T>:{flag:make()==<int32>;-><int32>}",
    ] {
        let output = Case::new(source).command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"B001\""));
    }
}

#[test]
pub(crate) fn type_equality_preserves_facade_identity_privacy_and_startup() {
    let files = [
        (
            "types.mwy",
            "d:@\"debug\";d.print(\"types\");private<Type>:<uint8>;->kind<Type>:<int32>",
        ),
        (
            "facade.mwy",
            "m:@\"./types.mwy\";d:@\"debug\";d.print(\"facade\");->kind<Type>:m.kind;-><Item>:m.kind",
        ),
    ];
    let case = super::file_modules::case(
        "m:@\"./facade.mwy\";s:@\"./types.mwy\";alias:m;f<int32>:(){<T>:{same:alias.kind==s.kind;named:<m.Item> == m.kind;|same&&named|-><int32>};v<T>:7;->v};d:@\"debug\";d.print(f())",
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
    }
    case.runs(b"types\nfacade\n7\n");
    for expr in ["m.private==<uint8>", "false&&(m.private==<uint8>)"] {
        let output = super::file_modules::case(
            &format!("m:@\"./types.mwy\";<T>:{{flag:{expr};-><int32>}}"),
            &files,
        )
        .command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E201\""));
    }
}

#[test]
pub(crate) fn type_equality_preserves_input_error_paths_through_facades() {
    let data = "#é🙂#\nraw:{->n<uint8>:255};->bad:raw.n+1";
    for expr in ["<int32[m.bad]> == <int32>", "<int32> != <int32[m.bad]>"] {
        let case = super::file_modules::case(
            &format!("m:@\"./facade.mwy\";<T>:{{flag:{expr};-><int32>}}"),
            &[("data.mwy", data), ("facade.mwy", "m:@\"./data.mwy\";->m")],
        );
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(error.contains("\"code\":\"E107\""), "{error}");
            assert!(
                error.contains(&format!("\"start\":{}", data.find("raw.n+1").unwrap())),
                "{error}"
            );
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

#[test]
pub(crate) fn type_equality_charges_repeated_inputs_and_resets_independent_roots() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("private:{{->4;v0:1;{tail}}};->width:private");
    for (expr, accepted) in [
        ("<int32[m.width]> == <int32[m.width]>", false),
        ("true||(<int32[m.width]> == <int32[m.width]>)", true),
        ("false&&(<int32[m.width]> != <int32[m.width]>)", true),
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
            for profile in ["debug", "release"] {
                let output = case.command("check", &["--profile", profile, "--json"]);
                let error = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.success(), accepted, "{expr}: {error}");
                if !accepted {
                    assert!(error.contains("\"code\":\"B001\""), "{error}");
                    assert!(error.contains("budget exhausted"), "{error}");
                }
            }
        }
    }
}
