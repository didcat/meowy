use super::file_modules::case;

#[test]
pub(crate) fn exported_types_preserve_transparent_facades_and_computed_type_values() {
    case("d:@\"debug\";api:@\"./api.mwy\";other:@\"./other.mwy\";p<other.Point>:{->x:3;->y:4};q<api.Point>:api.copy(p);d.print(q.x);d.print(q.y)", &[
        ("types.mwy", "d:@\"debug\";d.print(\"types\");<Private>:<{x<int32>;y<int32>}>;-><Point>:<Private>;->copy<Point>:(p<Point>){->p}"),
        ("api.mwy", "types:@\"./types.mwy\";token:<types.Point>;-><Point>:token;->copy<(Point)->Point>:types.copy"),
        ("other.mwy", "-><Point>:<{y<int32>;x<int32>}>"),
    ]).runs(b"types\n3\n4\n");
}

#[test]
pub(crate) fn exported_types_preserve_callable_signatures_in_the_type_namespace() {
    case(
        "api:@\"./api.mwy\";d:@\"debug\";<Local>:<api.Callback>;d.print(api.Callback(7))",
        &[
            (
                "ops.mwy",
                "-><Callback>:<(int32)->int32>;->inc<int32>:(n<int32>){->n+1}",
            ),
            (
                "api.mwy",
                "ops:@\"./ops.mwy\";-><Callback>:<ops.Callback>;->Callback<Callback>:ops.inc",
            ),
        ],
    )
    .runs(b"8\n");
}

#[test]
pub(crate) fn exported_types_keep_nominal_foundation_types_and_reference_rules() {
    case("api:@\"./api.mwy\";memory:@\"memory\";d:@\"debug\";handle<api.Handle>:memory.heap;copy:api.copy(handle);|copy<memory.Allocator>|d.print(\"heap\");x:=7;p<api.View>:&x;d.print(*p);x=8", &[
        ("api.mwy", "memory:@\"memory\";-><Handle>:<memory.Allocator>;-><View>:<&int32>;->copy<Handle>:(value<Handle>){->value}"),
    ]).runs(b"heap\n7\n");
    for (entry, types, code) in [
        (
            "m:@\"./types.mwy\";m.copy({->fake:1})",
            "memory:@\"memory\";-><Handle>:<memory.Allocator>;->copy<Handle>:(value<Handle>){->value}",
            "E212",
        ),
        (
            "m:@\"./types.mwy\";x:=7;p<m.View>:&x;x=8;v:*p",
            "-><View>:<&int32>",
            "E302",
        ),
        (
            "m:@\"./types.mwy\"",
            "-><View>:<&int32>;x:7;->view<View>:&x",
            "B001",
        ),
    ] {
        let case = case(entry, &[("types.mwy", types)]);
        let output = case.command("check", &["--json"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&format!("\"code\":\"{code}\"")),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
pub(crate) fn exported_types_do_not_erase_record_permissions_or_public_signatures() {
    for (entry, types, code) in [
        (
            "m:@\"./types.mwy\";p:{->n:7};m.copy(p)",
            "-><Row>:<{n<int32>:=}>;->copy<Row>:(value<Row>){->value}",
            "E212",
        ),
        (
            "m:@\"./types.mwy\";p<m.Row>:{->n:7};p.n=8",
            "-><Row>:<{n<int32>}>",
            "E305",
        ),
        (
            "m:@\"./types.mwy\";f<m.Call>:m.inc",
            "-><Call>:<(int32)->int32>;->inc<int32>:(x<int32>){->x+1}",
            "B001",
        ),
        (
            "m:@\"./types.mwy\"",
            "-><Call>:<(int8)->int32>;f<int32>:(x<int32>){->x};->alias<Call>:f",
            "E207",
        ),
    ] {
        let case = case(entry, &[("types.mwy", types)]);
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--json", "--profile", profile]);
            assert_eq!(output.status.code(), Some(1));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(&format!("\"code\":\"{code}\"")),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
pub(crate) fn exported_types_example_uses_a_typed_geometry_facade() {
    case(
        include_str!("../../examples/type-modules/main.mwy"),
        &[
            (
                "api.mwy",
                include_str!("../../examples/type-modules/api.mwy"),
            ),
            (
                "geometry.mwy",
                include_str!("../../examples/type-modules/geometry.mwy"),
            ),
        ],
    )
    .runs(b"geometry\n10\n0\n");
}

#[test]
pub(crate) fn exported_types_documentation_keeps_attachment_signatures_and_public_policy() {
    let source = "#!| Uses [[<Count>]]. |!#\n#| A public integer. |#\n-><Count>:<int32>";
    let case = super::Case::new(source);
    let checked = super::documentation::doc(&case, "check", &["--standalone", "--require-public"]);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let output = case.path.join("docs");
    let built = super::documentation::doc(
        &case,
        "build",
        &[
            "--standalone",
            "--require-public",
            "--output",
            output.to_str().unwrap(),
        ],
    );
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let page = std::fs::read_to_string(output.join("index.html")).unwrap();
    assert!(page.contains("Count"));
    assert!(page.contains("int32"));
    let undocumented = super::Case::new("-><Count>:<int32>");
    let output = super::documentation::doc(
        &undocumented,
        "check",
        &["--standalone", "--require-public", "--json"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"code\":\"E803\""));
}

#[test]
pub(crate) fn proof_descriptor_aliases_cross_facades_without_runtime_payloads() {
    case(
        r#"m:@"./facade.mwy";<Local>:<m.Outcome>;d:@"debug";d.print(m.Outcome)"#,
        &[
            ("types.mwy", r#"p:@"proof";d:@"debug";d.print("types");<Private>:<p.Result>;-><Outcome>:<Private>"#),
            ("facade.mwy", r#"m:@"./types.mwy";-><Outcome>:<m.Outcome>;->Outcome:7"#),
        ],
    ).runs(b"types\n7\n");
    for (entry, code) in [
        (r#"m:@"./facade.mwy";value<m.Outcome>:null"#, "E223"),
        (r#"m:@"./facade.mwy";<Hidden>:<m.Private>"#, "E202"),
    ] {
        let case = case(
            entry,
            &[
                (
                    "types.mwy",
                    r#"p:@"proof";<Private>:<p.Result>;-><Outcome>:<Private>"#,
                ),
                ("facade.mwy", r#"m:@"./types.mwy";-><Outcome>:<m.Outcome>"#),
            ],
        );
        for profile in ["debug", "release"] {
            let output = case.command("check", &["--profile", profile, "--json"]);
            let error = String::from_utf8_lossy(&output.stderr);
            assert_eq!(output.status.code(), Some(1));
            assert!(error.contains(&format!("\"code\":\"{code}\"")), "{error}");
            assert!(
                error.contains(&format!("\"path\":\"{}\"", case.source.display())),
                "{error}"
            );
            if code == "E223" {
                assert!(error.contains("proof.Result"), "{error}");
            }
        }
    }
}
