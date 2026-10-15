use super::Case;
use std::fs;
use std::process::Command;

#[test]
pub fn checking_never_runs_application_effects() {
    let case = Case::new("d:@\"debug\";d.print(\"must not run\");d.panic(\"must not panic\")");
    let result = case.command("check", &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stdout.is_empty());
    assert!(!case.path.join("build").exists());
}

#[test]
pub fn rejection_codes_and_original_byte_spans_are_stable() {
    for (source, code) in [
        ("x<int8>:-(128)", "E216"),
        ("value:?", "E001"),
        ("value:1__0", "E201"),
        ("x:missing", "E201"),
        ("x:1;x:2", "E203"),
        ("x:{->1;->2}", "E205"),
        ("f<int32>:(){}", "E204"),
        ("x<int8>:127+1", "E107"),
        ("x<int32>:1;y<int64>:2;z:x+y", "E213"),
        ("|1|{}", "E215"),
        ("f<int32>:(){->1};v:f==f", "E222"),
    ] {
        let result = Case::new(source).command("check", &["--json"]);
        assert_eq!(result.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains(&format!("\"code\":\"{code}\"")),
            "{source}: {stderr}"
        );
        assert!(result.stdout.is_empty());
    }
    let result = Case::new("#é#\r\nx:1__0").command("check", &["--json"]);
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("\"start\":8"), "{stderr}");
    assert!(stderr.contains("\"line\":2"), "{stderr}");
}

#[test]
pub fn output_is_an_elf_and_remains_intact_when_rebuild_fails() {
    let case = Case::new("d:@\"debug\";d.print(42)");
    let output = case.path.join("app");
    let result = case.command(
        "build",
        &[
            "--output",
            output.to_str().unwrap(),
            "--emit-llvm",
            "app.ll",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(&output).unwrap();
    assert_eq!(&bytes[..4], b"\x7fELF");
    assert!(
        fs::read_to_string(case.path.join("app.ll"))
            .unwrap()
            .contains("target triple")
    );
    fs::write(&case.source, "x:unknown").unwrap();
    assert_eq!(
        case.command("build", &["--output", output.to_str().unwrap()])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let failed = case.command("run", &[]);
    assert_eq!(failed.status.code(), Some(1));
    assert!(failed.stdout.is_empty());
}

#[test]
pub fn protected_inputs_and_usage_errors_are_rejected() {
    let case = Case::new("x:1");
    assert_eq!(
        case.command("build", &["--output", case.source.to_str().unwrap()])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(fs::read_to_string(&case.source).unwrap(), "x:1");
    assert_eq!(
        case.command("build", &["--output", "mod.lock"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        case.command("check", &["--output", "app"]).status.code(),
        Some(2)
    );
    let result = case.command(
        "check",
        &["--target", "aarch64-unknown-linux-gnu", "--json"],
    );
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("E507"));
    #[cfg(unix)]
    {
        let hard = case.path.join("hard");
        fs::hard_link(&case.source, &hard).unwrap();
        assert_eq!(
            case.command("build", &["--output", hard.to_str().unwrap()])
                .status
                .code(),
            Some(2)
        );
        let sym = case.path.join("sym");
        std::os::unix::fs::symlink(&case.source, &sym).unwrap();
        assert_eq!(
            case.command("build", &["--output", sym.to_str().unwrap()])
                .status
                .code(),
            Some(2)
        );
    }
}

#[test]
pub fn unavailable_features_and_manifests_are_explicit() {
    let case = Case::new("values<int32[]>:[]");
    let result = case.command("check", &["--json"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("B001"));
    fs::write(&case.source, "x:1").unwrap();
    fs::write(case.path.join("mod.mwy"), "->build:{->entry:\"main.mwy\"}").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_meowy"))
        .arg("check")
        .arg(&case.source)
        .arg("--json")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stderr).contains("B001"));
    assert!(case.command("check", &[]).status.success());
}

#[test]
pub fn invalid_utf8_is_a_source_error() {
    let case = Case::new("");
    fs::write(&case.source, [b'x', b':', 0xff]).unwrap();
    let result = case.command("check", &["--json"]);
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("\"code\":\"E001\""));
    assert!(stderr.contains("\"start\":2"));
}
