use super::file_modules::case;

#[test]
pub(crate) fn type_exports_preserve_identity_through_explicit_facades_and_function_scopes() {
    case(
        "m:@\"./facade.mwy\";alias:m;copy<Type>:alias.items;v<(copy)>:[3,7];f<int32>:(){local<m.Kind>:m.element;v<(local)>:9;->v};d:@\"debug\";d.print(v[2]);d.print(f());d.print(m.get())",
        &[
            ("types.mwy", "c:@\"core\";private<c.Type>:<int32>;->element<c.Type>:private;->items<Type>:{-><(element)[4]>};-><Kind>:<c.Type>;->get<int32>:(){->11}"),
            ("facade.mwy", "m:@\"./types.mwy\";-><Kind>:<m.Kind>;->element<m.Kind>:m.element;->items<Type>:m.items;->get<()->int32>:m.get"),
        ],
    ).runs(b"7\n9\n11\n");
    case(
        "a:@\"./a.mwy\";b:@\"./b.mwy\";x<(a.kind)>:255;y<(b.kind)>:256;d:@\"debug\";d.print(x);d.print(y)",
        &[("a.mwy", "->kind<Type>:<uint8>"), ("b.mwy", "->kind<Type>:<uint16>")],
    ).runs(b"255\n256\n");
    case(
        "m:@\"./facade.mwy\";v<m.Items>:[7];d:@\"debug\";d.print(v[1])",
        &[
            ("types.mwy", "->items<Type>:<int32[4]>"),
            ("facade.mwy", "m:@\"./types.mwy\";-><Items>:m.items"),
        ],
    )
    .runs(b"7\n");
}

#[test]
pub(crate) fn type_exports_require_explicit_reexports_and_keep_private_value_names() {
    for (entry, facade, code) in [
        ("kind<Type>:m.element", "->source", "E201"),
        (
            "kind<Type>:m.private",
            "->element<Type>:source.element",
            "E201",
        ),
        ("kind<Type>:m.element", "copy:source.element", "E201"),
        ("v<m.element>:7", "->element<Type>:source.element", "E202"),
        ("kind<Type>:m.element", "->element:source.element", "B001"),
        (
            "kind<Type>:m.element",
            "->element<uint8>:source.element",
            "B001",
        ),
        (
            "kind<Type>:m.element",
            "->element<Type>:=source.element",
            "B001",
        ),
        (
            "kind<Type>:m.element",
            "|true|->element<Type>:source.element",
            "B001",
        ),
        (
            "kind<Type>:m.element",
            "->element<Type>:source.private",
            "E201",
        ),
    ] {
        let case = case(
            &format!("m:@\"./facade.mwy\";{entry}"),
            &[
                (
                    "types.mwy",
                    "private<Type>:<uint8>;->element<Type>:<int32>;->n:7",
                ),
                ("facade.mwy", &format!("source:@\"./types.mwy\";{facade}")),
            ],
        );
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1), "{facade}");
            assert!(
                error.contains(&format!("\"code\":\"{code}\"")),
                "{facade}: {error}"
            );
        }
    }
}

#[test]
pub(crate) fn type_exports_check_facade_collisions_and_shadowable_annotations() {
    for facade in [
        "->kind<Type>:m.kind;->kind<Type>:m.kind",
        "->kind<Type>:m.kind;->kind<int32>:(){->7}",
        "->kind<int32>:(){->7};->kind<Type>:m.kind",
        "->kind<Type>:m.kind;->data",
        "->data;->kind<Type>:m.kind",
    ] {
        let output = case(
            "m:@\"./facade.mwy\"",
            &[
                ("types.mwy", "->kind<Type>:<int32>"),
                ("data.mwy", "->kind:7"),
                (
                    "facade.mwy",
                    &format!("m:@\"./types.mwy\";data:@\"./data.mwy\";{facade}"),
                ),
            ],
        )
        .command("check", &["--json"]);
        let error = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1));
        assert!(error.contains("\"code\":\"E205\""), "{facade}: {error}");
    }
    case(
        "m:@\"./facade.mwy\";d:@\"debug\";d.print(m.n)",
        &[
            ("fake.mwy", "-><Type>:<uint8>"),
            ("facade.mwy", "m:@\"./fake.mwy\";->n<m.Type>:4"),
        ],
    )
    .runs(b"4\n");
}

#[test]
pub(crate) fn type_exports_preserve_silent_checks_and_single_dependency_startup() {
    let case = case(
        "m:@\"./facade.mwy\";s:@\"./types.mwy\";d:@\"debug\";d.print(\"entry\");v<(m.kind)>:7;d.print(v)",
        &[
            (
                "types.mwy",
                "d:@\"debug\";d.print(\"types\");->kind<Type>:<int32>",
            ),
            (
                "facade.mwy",
                "m:@\"./types.mwy\";d:@\"debug\";d.print(\"facade\");->kind<Type>:m.kind",
            ),
        ],
    );
    for profile in ["debug", "release"] {
        for action in ["check", "build"] {
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
    case.runs(b"types\nfacade\nentry\n7\n");
}

#[test]
pub(crate) fn type_exports_do_not_remove_failing_module_initializers() {
    let case = case(
        "m:@\"./facade.mwy\";d:@\"debug\";d.print(\"entry\");v<(m.kind)>:7",
        &[
            (
                "types.mwy",
                "d:@\"debug\";->kind<Type>:<int32>;d.print(\"types\");d.panic(\"stop\")",
            ),
            (
                "facade.mwy",
                "m:@\"./types.mwy\";->kind<Type>:m.kind;d:@\"debug\";d.print(\"facade\")",
            ),
        ],
    );
    for profile in ["debug", "release"] {
        let output = case.command("check", &["--profile", profile]);
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
        let output = case.command("run", &["--profile", profile]);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"types\n");
        assert!(String::from_utf8_lossy(&output.stderr).starts_with("panic[P006]: stop"));
    }
}

#[test]
pub(crate) fn type_exports_retain_original_required_input_errors_across_facades() {
    let data = "#é🙂#\nraw:{->n<uint8>:255};->bad:raw.n+1";
    for body in [
        "->kind<Type>:<int32[m.bad]>",
        "->kind<Type>:{-><int32>;tail:m.bad}",
    ] {
        let case = case(
            "m:@\"./facade.mwy\";v<(m.kind)>:7",
            &[
                ("data.mwy", data),
                ("input.mwy", "m:@\"./data.mwy\";->m"),
                ("types.mwy", &format!("m:@\"./input.mwy\";{body}")),
                ("facade.mwy", "m:@\"./types.mwy\";->kind<Type>:m.kind"),
            ],
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
pub(crate) fn type_exports_reset_roots_and_share_nested_initializer_work() {
    let tail = (1..18)
        .map(|id| format!("v{id}:v{}+1;", id - 1))
        .collect::<String>();
    let data = format!("private:{{->4;v0:1;{tail}}};->width:private");
    let roots = (0..20)
        .map(|id| format!("->kind{id}<Type>:{{n:m.width;-><int32[n]>}};"))
        .collect::<String>();
    let nested = roots.replace("->kind", "kind");
    for (body, accepted) in [
        (roots, true),
        (format!("->all<Type>:{{{nested}-><int32>}}"), false),
    ] {
        let case = case(&format!("m:@\"./data.mwy\";{body}"), &[("data.mwy", &data)]);
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            if accepted {
                assert!(output.status.success(), "{error}");
            } else {
                assert_eq!(output.status.code(), Some(1));
                assert!(error.contains("\"code\":\"B001\""), "{error}");
                assert!(error.contains("computed type bootstrap budget"), "{error}");
            }
        }
    }
}
