use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const VERSION: &str = "23.1.1";
pub(crate) const CLANG: &str = "/usr/bin/clang";
pub(crate) const CXX: &str = "/usr/bin/clang++";
pub(crate) const LLD: &str = "/usr/bin/ld.lld";
pub(crate) const LLVM: &str = "/usr/bin/llvm-config";
pub(crate) const AR: &str = "/usr/bin/llvm-ar";

pub(crate) fn output(tool: &str, args: &[&str]) -> String {
    let result = Command::new(tool)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("cannot run pinned bootstrap tool {tool}: {error}"));
    assert!(
        result.status.success(),
        "{tool} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout)
        .expect("bootstrap tool output must be UTF-8")
        .trim()
        .to_owned()
}

pub(crate) fn run(tool: &str, args: &[String]) {
    let result = Command::new(tool)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("cannot run pinned bootstrap tool {tool}: {error}"));
    assert!(
        result.status.success(),
        "{tool} failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

pub(crate) fn archive(dir: &Path, name: &str, object: &Path) -> PathBuf {
    let path = dir.join(format!("lib{name}.a"));
    run(
        AR,
        &[
            "rcsD".into(),
            path.display().to_string(),
            object.display().to_string(),
        ],
    );
    path
}

pub(crate) fn main() {
    assert_eq!(
        env::var("TARGET").as_deref(),
        Ok("x86_64-unknown-linux-gnu"),
        "the bootstrap supports Linux x86-64 only"
    );
    assert_eq!(
        output(LLVM, &["--version"]),
        VERSION,
        "LLVM version differs from the bootstrap pin"
    );
    for tool in [CLANG, CXX, LLD, AR] {
        let version = output(tool, &["--version"]);
        assert!(
            version
                .lines()
                .take(2)
                .any(|line| line.split_whitespace().any(|word| word == VERSION)),
            "{tool} must be version {VERSION}, found {version}"
        );
        println!("cargo:rerun-if-changed={tool}");
    }
    println!("cargo:rerun-if-changed={LLVM}");
    println!("cargo:rerun-if-changed=native/bridge.cpp");
    println!("cargo:rerun-if-changed=native/runtime.cpp");
    println!("cargo:rerun-if-changed=build.rs");
    let dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let bridge = dir.join("bridge.o");
    let mut args: Vec<String> = output(LLVM, &["--cxxflags"])
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    args.extend(
        [
            "-std=c++20",
            "-fno-exceptions",
            "-fno-rtti",
            "-fPIC",
            "-O2",
            "-c",
            "native/bridge.cpp",
            "-o",
        ]
        .map(str::to_owned),
    );
    args.push(bridge.display().to_string());
    run(CXX, &args);
    archive(&dir, "meowy_bridge", &bridge);
    let runtime = dir.join("runtime.o");
    let mut args: Vec<String> = [
        "-std=c++20",
        "--target=x86_64-unknown-linux-gnu",
        "-march=x86-64",
        "-mtune=generic",
        "-fno-exceptions",
        "-fno-rtti",
        "-fno-stack-protector",
        "-ffunction-sections",
        "-fdata-sections",
        "-fPIC",
        "-O2",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-I../runtime/include",
        "-c",
        "native/runtime.cpp",
        "-o",
    ]
    .map(str::to_owned)
    .to_vec();
    args.push(runtime.display().to_string());
    run(CXX, &args);
    let runtime = archive(&dir, "meowy_runtime", &runtime);
    for header in ["cleanup.hpp", "generated.hpp", "owned.hpp", "strings.hpp"] {
        println!("cargo:rerun-if-changed=../runtime/include/meowy/{header}");
    }
    let source = args
        .iter()
        .position(|arg| arg == "native/runtime.cpp")
        .unwrap();
    let dest = args.len() - 1;
    args.extend(["-I../runtime/include".into()]);
    for name in ["cleanup", "owned", "generated", "strings"] {
        let path = format!("../runtime/src/{name}.cpp");
        println!("cargo:rerun-if-changed={path}");
        let object = dir.join(format!("{name}.o"));
        args[source] = path;
        args[dest] = object.display().to_string();
        run(CXX, &args);
        run(
            AR,
            &[
                "rcsD".into(),
                runtime.display().to_string(),
                object.display().to_string(),
            ],
        );
    }

    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static=meowy_bridge");
    for flag in output(LLVM, &["--ldflags", "--libs", "--system-libs"]).split_whitespace() {
        if let Some(path) = flag.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={path}");
        } else if let Some(name) = flag.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={name}");
        } else {
            panic!("unsupported llvm-config linker flag {flag}");
        }
    }
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-env=MEOWY_CLANG={CLANG}");
    println!("cargo:rustc-env=MEOWY_LLD={LLD}");
    println!("cargo:rustc-env=MEOWY_LLVM_VERSION={VERSION}");
    println!(
        "cargo:rustc-env=MEOWY_RUNTIME_ARCHIVE={}",
        runtime.display()
    );
}
