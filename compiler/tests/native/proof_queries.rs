use super::{Case, file_modules::case};

#[test]
pub(crate) fn pending_proof_queries_keep_file_origins_and_never_start_programs() {
    let source = "#é🙂#\np:@\"proof\";r:p.can_copy<uint32>();copy:r";
    let case = case(
        "m:@\"./query.mwy\";d:@\"debug\";d.print(\"entry\")",
        &[("query.mwy", source)],
    );
    for profile in ["debug", "release"] {
        for action in ["check", "build", "run"] {
            let output = case.command(action, &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(error.contains("\"code\":\"B001\""), "{error}");
            assert!(error.contains("proof.can_copy evaluation"), "{error}");
            assert!(
                error.contains(&format!("\"start\":{}", source.find("p.can_copy").unwrap())),
                "{error}"
            );
            assert!(
                error.contains(&case.path.join("query.mwy").display().to_string()),
                "{error}"
            );
        }
    }
}

#[test]
pub(crate) fn pending_proof_type_arguments_discover_imports_and_preserve_errors() {
    for (file, code) in [
        (
            r#"d:@"debug";d.print("startup");->kind<Type>:<uint32>"#,
            "B001",
        ),
        (r#"bad<int32>:false;->kind<Type>:<uint32>"#, "E207"),
    ] {
        let case = case(
            r#"p:@"proof";r:p.can_copy<((@"./type.mwy").kind)>()"#,
            &[("type.mwy", file)],
        );
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(error.contains(&format!("\"code\":\"{code}\"")), "{error}");
            if code == "E207" {
                assert!(
                    error.contains(&case.path.join("type.mwy").display().to_string()),
                    "{error}"
                );
            } else {
                assert!(error.contains("proof.can_copy evaluation"), "{error}");
            }
        }
    }
    let missing = Case::new(r#"p:@"proof";r:p.can_copy<((@"./missing.mwy").kind)>()"#);
    let output = missing.command("check", &["--json"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E501\""));
}

#[test]
pub(crate) fn pending_proof_queries_preserve_ordinary_failures_in_checked_bodies() {
    for source in [
        r#"p:@"proof";r:p.can_copy<uint32>();x:=7;view:&x;x=8;copy:*view"#,
        r#"p:@"proof";f:(){r:p.can_copy<uint32>();x:=7;view:&x;x=8;copy:*view}"#,
    ] {
        let case = Case::new(source);
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            assert_eq!(output.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E302\""));
        }
    }
}

#[test]
pub(crate) fn pending_query_budgets_preserve_facade_targets_and_first_call_origins() {
    for ty in ["m.Row", "m.Result", "Type", "({-><uint8[({->2})]>})"] {
        let source = format!(
            r#"m:@"./facade.mwy";p:@"proof";query:p.can_copy;first:query<{ty}>();copy<p.Result>:first;second:query<never>()"#
        );
        let case = case(
            &source,
            &[
                (
                    "facade.mwy",
                    r#"m:@"./types.mwy";-><Row>:<m.Row>;-><Result>:<m.Result>"#,
                ),
                (
                    "types.mwy",
                    r#"p:@"proof";d:@"debug";d.print("startup");-><Row>:<{n<uint32>}>;-><Result>:<p.Result>"#,
                ),
            ],
        );
        for profile in ["debug", "release"] {
            let output = case.command("run", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(output.stdout.is_empty());
            assert!(error.contains("\"code\":\"B001\""), "{error}");
            assert!(error.contains("proof.can_copy evaluation"), "{error}");
            assert!(
                error.contains(&format!("\"start\":{}", source.find("query<").unwrap())),
                "{error}"
            );
        }
    }
}

#[test]
pub(crate) fn pending_query_budgets_preserve_argument_modes_and_dependency_errors() {
    for (ty, code, token) in [
        ("uint8[({->2})]", "B001", "({->2})"),
        ("({-><uint8[1/0]>})", "E107", "1/0"),
        ("Missing", "E202", "Missing"),
    ] {
        let source = format!(r#"p:@"proof";d:@"debug";d.print("startup");r:p.can_copy<{ty}>()"#);
        let case = case(r#"m:@"./query.mwy""#, &[("query.mwy", &source)]);
        for profile in ["debug", "release"] {
            for action in ["check", "build", "run"] {
                let output = case.command(action, &["--profile", profile, "--json"]);
                let error = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty());
                assert!(error.contains(&format!("\"code\":\"{code}\"")), "{error}");
                assert!(
                    error.contains(&case.path.join("query.mwy").display().to_string()),
                    "{error}"
                );
                let start = source.find(token).unwrap();
                assert!(error.contains(&format!("\"start\":{start}")), "{error}");
                assert!(
                    error.contains(&format!("\"end\":{}", start + token.len())),
                    "{error}"
                );
            }
        }
    }
}
