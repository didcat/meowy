use super::{ExampleMode, Model, Result};
use crate::{
    ast::Span,
    diagnostic::Diagnostic,
    driver::{self, Action, Options, Scratch},
};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub(crate) fn execute(opts: &Options, source: &str, entry: &Path) -> i32 {
    let result = (|| -> Result<()> {
        let (_, docs) = super::checked(source, true).map_err(|mut errors| errors.remove(0))?;
        let mut model = docs.expect("documentation collection");
        if opts.require_public {
            model.require_public()?;
        }
        let (checked, ran) = examples(&mut model, opts)?;
        if opts.action == Action::DocBuild {
            let output = opts
                .output
                .as_ref()
                .expect("validated documentation output");
            let page = super::render::page(
                &model,
                &entry.file_name().unwrap_or_default().to_string_lossy(),
            );
            publish(output, entry, &page)?;
        }
        if !opts.quiet && !opts.json {
            println!(
                "documentation: {} declarations; {checked} examples checked; {ran} run",
                model.entries.len().saturating_sub(1)
            );
        }
        Ok(())
    })();
    match result {
        Ok(()) => 0,
        Err(error) => {
            driver::report(opts, source, &error);
            1
        }
    }
}

pub(crate) fn examples(model: &mut Model, opts: &Options) -> Result<(usize, usize)> {
    let total = model
        .entries
        .iter()
        .map(|entry| entry.examples.len())
        .sum::<usize>();
    if total > 256 {
        return Err(Model::budget(Span::default()));
    }
    let mut ran = 0;
    for entry in &mut model.entries {
        for example in &mut entry.examples {
            let program = crate::compile(&example.source);
            let program = match (&example.mode, program) {
                (ExampleMode::Reject(expected), Err(errors))
                    if errors.first().is_some_and(|error| error.code == expected) =>
                {
                    continue;
                }
                (ExampleMode::Reject(expected), Ok(_)) => {
                    return Err(Diagnostic::new(
                        "E804",
                        format!("example expected {expected}, but checking succeeded"),
                        example.span,
                    ));
                }
                (_, Err(errors)) => {
                    let error = &errors[0];
                    let line = example.source[..error.span.start.min(example.source.len())]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    let message = format!(
                        "example line {line}, bytes {}..{}: {}: {}",
                        error.span.start, error.span.end, error.code, error.message
                    );
                    return Err(if error.code == "B001" {
                        Diagnostic::unsupported(message, example.span)
                    } else {
                        Diagnostic::new("E804", message, example.span)
                    });
                }
                (_, Ok(program)) => program,
            };
            if opts.run_examples && matches!(example.mode, ExampleMode::Run) {
                let scratch = Scratch::new(&std::env::temp_dir()).map_err(|error| {
                    Diagnostic::new(
                        "E804",
                        format!("cannot prepare example: {error}"),
                        example.span,
                    )
                })?;
                let ir = crate::backend::emit_ir(&program).map_err(|error| {
                    Diagnostic::new(
                        "F001",
                        format!("example lowering failed: {error}"),
                        example.span,
                    )
                })?;
                let binary = scratch.path.join("example");
                driver::build(&ir, &binary, opts)
                    .map_err(|error| Diagnostic::new("B002", error, example.span))?;
                let output = run_example(&binary, &scratch.path, opts.example_timeout)
                    .map_err(|message| Diagnostic::new("E804", message, example.span))?;
                if let Some(expected) = &example.expected
                    && output != expected.as_bytes()
                {
                    return Err(Diagnostic::new(
                        "E804",
                        "example stdout differs from its output fence",
                        example.span,
                    ));
                }
                example.ran = true;
                ran += 1;
            }
        }
    }
    Ok((total, ran))
}

pub(crate) fn capture(mut input: impl Read, overflow: Arc<AtomicBool>) -> std::io::Result<Vec<u8>> {
    const LIMIT: usize = 1024 * 1024;
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let size = input.read(&mut buffer)?;
        if size == 0 {
            return Ok(output);
        }
        let keep = size.min(LIMIT.saturating_sub(output.len()));
        output.extend_from_slice(&buffer[..keep]);
        if keep != size {
            overflow.store(true, Ordering::Relaxed);
        }
    }
}

pub(crate) fn run_example(
    binary: &Path,
    directory: &Path,
    timeout: u64,
) -> std::result::Result<Vec<u8>, String> {
    let mut child = Command::new(binary)
        .current_dir(directory)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run example: {error}"))?;
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let flag = overflow.clone();
    let out = std::thread::spawn(move || capture(stdout, flag));
    let flag = overflow.clone();
    let err = std::thread::spawn(move || capture(stderr, flag));
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let mut failure = None;
    let status = loop {
        if overflow.load(Ordering::Relaxed) || Instant::now() >= deadline {
            failure = Some(
                if overflow.load(Ordering::Relaxed) {
                    "example output limit exceeded"
                } else {
                    "example time limit exceeded"
                }
                .to_string(),
            );
            let _ = child.kill();
            break child.wait();
        }
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error);
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "example stdout reader failed")?
        .map_err(|error| error.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "example stderr reader failed")?
        .map_err(|error| error.to_string())?;
    if let Some(failure) = failure {
        return Err(failure);
    }
    if overflow.load(Ordering::Relaxed) {
        return Err("example output limit exceeded".into());
    }
    let status = status.map_err(|error| error.to_string())?;
    if !status.success() || !stderr.is_empty() {
        return Err(format!(
            "example failed ({status}): {}",
            String::from_utf8_lossy(&stderr)
        ));
    }
    Ok(stdout)
}

pub(crate) fn publish(output: &Path, entry: &Path, page: &str) -> Result<()> {
    let fail = |message: String| Diagnostic::new("E805", message, Span::default());
    let output = driver::absolute(output).map_err(|error| fail(error.to_string()))?;
    if output
        .extension()
        .is_some_and(|ext| ext == "mwy" || ext == "replay")
    {
        return Err(fail(
            "documentation output is a protected source path".into(),
        ));
    }
    if let Ok(meta) = fs::symlink_metadata(&output) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(fail("documentation output must be a real directory".into()));
        }
        for item in fs::read_dir(&output).map_err(|error| fail(error.to_string()))? {
            let item = item.map_err(|error| fail(error.to_string()))?;
            if item.file_name() != "index.html" {
                return Err(fail("documentation output contains unrelated files".into()));
            }
            driver::protect(&item.path(), entry).map_err(fail)?;
            let meta = item.metadata().map_err(|error| fail(error.to_string()))?;
            if meta.len() > 16 * 1024 * 1024 {
                return Err(fail(
                    "existing documentation output exceeds the size limit".into(),
                ));
            }
            let data = fs::read(item.path()).map_err(|error| fail(error.to_string()))?;
            if !data.starts_with(super::render::MARKER.as_bytes()) {
                return Err(fail("refusing to replace an unowned index.html".into()));
            }
        }
    }
    let index = output.join("index.html");
    driver::protect(&index, entry).map_err(fail)?;
    let scratch = Scratch::new(output.parent().unwrap_or(Path::new(".")))
        .map_err(|error| fail(error.to_string()))?;
    let staged = scratch.path.join("index.html");
    fs::write(&staged, page).map_err(|error| fail(error.to_string()))?;
    fs::create_dir_all(&output).map_err(|error| fail(error.to_string()))?;
    fs::rename(staged, index).map_err(|error| fail(error.to_string()))?;
    Ok(())
}
