use super::Case;
use std::{
    fs,
    process::{Command, Output},
};

pub fn doc(case: &Case, action: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_meowy"))
        .args(["doc", action])
        .arg(&case.source)
        .args(args)
        .output()
        .unwrap()
}

pub const SOURCE: &str = "#!| An API for [[add]]. |!#\n#||\nAdds [[left]] and [[right]].\n```meowy run\nd:@\"debug\";d.print(5)\n```\n```output\n5\n```\n||#\nadd<int32>:(#| Left. |#left<int32>,#| Right. |#right<int32>){->left+right}";

#[test]
pub fn documentation_commands_check_build_and_run_examples_in_both_profiles() {
    let case = Case::new(SOURCE);
    for profile in ["debug", "release"] {
        let checked = doc(&case, "check", &["--profile", profile, "--require-public"]);
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
        assert!(String::from_utf8_lossy(&checked.stdout).contains("1 examples checked; 0 run"));
        let run = doc(&case, "check", &["--profile", profile, "--run-examples"]);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        assert!(String::from_utf8_lossy(&run.stdout).contains("1 run"));
    }
    let output = case.path.join("site");
    let built = doc(&case, "build", &["--output", output.to_str().unwrap()]);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let first = fs::read(output.join("index.html")).unwrap();
    assert!(String::from_utf8_lossy(&first).contains("not executed"));
    assert!(
        doc(&case, "build", &["--output", output.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(first, fs::read(output.join("index.html")).unwrap());
}

#[test]
pub fn documentation_never_executes_application_initialization_or_implicit_examples() {
    let case = Case::new(
        "d:@\"debug\";d.panic(\"application must not run\");#||\n```meowy run\nd:@\"debug\";d.panic(\"example must not run\")\n```\n||#x:1",
    );
    let output = doc(&case, "check", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("must not run"));
}

#[test]
pub fn documentation_reports_attachment_links_and_public_policy() {
    for (source, code, args) in [
        ("#| orphan |#", "E801", vec![]),
        ("#| [[missing]] |#x:1", "E802", vec![]),
        ("x:1", "E803", vec!["--require-public"]),
    ] {
        let output = doc(&Case::new(source), "check", &args);
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains(&format!("error[{code}]")));
    }
}

#[test]
pub fn documentation_reject_examples_require_the_exact_language_failure() {
    let case = Case::new("#||\n```meowy reject=E207\nx<int32>:\"wrong\"\n```\n||#x:1");
    assert!(doc(&case, "check", &[]).status.success());
    let wrong = Case::new("#||\n```meowy reject=E207\nx:1\n```\n||#x:1");
    let output = doc(&wrong, "check", &[]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("E804"));
    let unsupported = Case::new("#||\n```meowy reject=E207\nh:@\"http\"\n```\n||#x:1");
    let output = doc(&unsupported, "check", &[]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("B001"));
    assert!(!output.status.success());
}

#[test]
pub fn documentation_run_examples_check_output_and_time_limits() {
    let wrong = Case::new(&SOURCE.replace("\n5\n", "\n6\n"));
    assert!(doc(&wrong, "check", &[]).status.success());
    let output = doc(&wrong, "check", &["--run-examples"]);
    assert!(String::from_utf8_lossy(&output.stderr).contains("stdout differs"));
    let looped = Case::new("#||\n```meowy run\n'loop{'loop.restart()}\n```\n||#x:1");
    let output = doc(
        &looped,
        "check",
        &["--run-examples", "--example-timeout-ms", "25"],
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("time limit"));
}

#[test]
pub fn documentation_run_examples_bound_captured_output() {
    let code = format!(
        "#||\n```meowy run\nd:@\"debug\";i:=0;'loop{{d.print(\"{}\");i=i+1;|i<300|'loop.restart()}}\n```\n||#x:1",
        "x".repeat(4096)
    );
    let output = doc(&Case::new(&code), "check", &["--run-examples"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("output limit"));
}

#[test]
pub fn documentation_publication_preserves_unrelated_and_previous_output() {
    let case = Case::new(SOURCE);
    let output = case.path.join("site");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("index.html"), "user page").unwrap();
    assert!(
        !doc(&case, "build", &["--output", output.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(output.join("index.html")).unwrap(),
        "user page"
    );
    let other = case.path.join("owned");
    assert!(
        doc(&case, "build", &["--output", other.to_str().unwrap()])
            .status
            .success()
    );
    let prior = fs::read(other.join("index.html")).unwrap();
    fs::write(&case.source, "#| [[missing]] |#x:1").unwrap();
    assert!(
        !doc(&case, "build", &["--output", other.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(prior, fs::read(other.join("index.html")).unwrap());
    assert!(
        !doc(&case, "build", &["--output", case.source.to_str().unwrap()])
            .status
            .success()
    );
}

#[test]
pub fn documentation_cli_keeps_unavailable_modes_and_assets_explicit() {
    let case = Case::new(SOURCE);
    assert_eq!(doc(&case, "build", &[]).status.code(), Some(2));
    assert_eq!(
        doc(&case, "build", &["--output", "ignored", "--run-examples"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        doc(&case, "check", &["--example-timeout-ms", "0"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        doc(&case, "build", &["--output", "ignored", "--assets", "."])
            .status
            .code(),
        Some(2)
    );
}

#[test]
pub fn proof_descriptor_documentation_preserves_nominal_alias_names() {
    let case = Case::new(
        r#"p:@"proof";#| Outcome. |#<Outcome>:<p.Result>;#| Guaranteed. |#<Guaranteed>:<p.Always>;#| Alias of [[<Outcome>]]. |#<Again>:<Outcome>"#,
    );
    let output = case.path.join("site");
    let built = doc(&case, "build", &["--output", output.to_str().unwrap()]);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let page = fs::read_to_string(output.join("index.html")).unwrap();
    assert!(page.contains("proof.Result"));
    assert!(page.contains("proof.Always"));
    assert!(page.contains("Again"));
}
