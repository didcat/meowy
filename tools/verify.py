import argparse
import contextlib
import os
import re
import shlex
import signal
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TARGET = "x86_64-unknown-linux-gnu"
PYTHON_CHECKS = ("tooling regressions", "runtime harness regressions", "compiler harness regressions")


@dataclass(frozen=True)
class Check:
    name: str
    args: tuple[str, ...]


def plan(editor=None, compiler=False, strict=False, runtime=False):
    python = (sys.executable, "-B", "-X", "utf8", "-E")
    checks = [
        Check("tooling regressions", python + ("-m", "unittest", "discover", "-s", "tools/tests", "-p", "test_*.py")),
        Check("local documentation links", python + ("tools/check_links.py",)),
        Check("conformance catalog metadata", python + ("docs/conformance/check.py",)),
        Check("artifact schemas and example identities", python + ("docs/schemas/validate.py",)),
    ]
    if editor in ("vim", "both"):
        checks.append(Check("Vim runtime", ("vim", "-Nu", "NONE", "-i", "NONE", "-n", "-es", "-S", "editor/nvim/tests/run.vim")))
    if editor in ("nvim", "both"):
        checks.append(Check("Neovim runtime", ("env", "NVIM_LOG_FILE=" + os.devnull, "nvim", "--headless", "-u", "NONE", "-i", "NONE", "-n", "-S", "editor/nvim/tests/run.vim")))
    if runtime:
        checks.extend([
            Check("runtime harness regressions", python + ("-m", "unittest", "discover", "-s", "runtime/tests", "-p", "test_*.py")),
            Check("native cleanup, stacks, contexts and scheduling", python + ("runtime/check.py",)),
        ])
    if compiler:
        cargo = ("--locked", "--manifest-path", "compiler/Cargo.toml", "--target", TARGET, "--target-dir", "compiler/target")
        checks.extend([
            Check("Rust formatting", ("cargo", "fmt", "--manifest-path", "compiler/Cargo.toml", "--check")),
            Check("Rust lint", ("cargo", "clippy") + cargo + ("--all-targets", "--", "-D", "warnings")),
            Check("Rust and native regression tests", ("cargo", "test") + cargo),
            Check("compiler harness regressions", python + ("-m", "unittest", "discover", "-s", "compiler/tests", "-p", "test_*.py")),
            Check("bootstrap compiler build", ("cargo", "build") + cargo),
            Check("compiler conformance execution", python + ("compiler/tests/conformance.py", "--compiler", str(ROOT / "compiler/target" / TARGET / "debug/meowy")) + (("--strict",) if strict else ())),
        ])
    return checks


def run(check, root=ROOT, timeout=300, log=None):
    command = shlex.join(check.args)
    print(f"CHECK {check.name}" if log else f"CHECK {check.name}: {command}", flush=True)
    with contextlib.ExitStack() as stack:
        try:
            output = stack.enter_context(log.open("w", encoding="utf-8")) if log else None
            if output:
                output.write(f"Command: {command}\n")
                output.flush()
            process = subprocess.Popen(check.args, cwd=root, start_new_session=True,
                                       stdout=output, stderr=subprocess.STDOUT if output else None)
        except OSError as error:
            print(f"FAIL {check.name}: {error}", flush=True)
            return False
        try:
            code = process.wait(timeout=timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt) as error:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            if isinstance(error, KeyboardInterrupt):
                raise
            print(f"FAIL {check.name}: exceeded {timeout:g} seconds", flush=True)
            return False
    print(f"{'PASS' if code == 0 else 'FAIL'} {check.name}: exit {code}", flush=True)
    return code == 0


def python_result(output):
    match = re.search(r"^Ran (\d+) tests? in [^\n]+\n\n(OK(?: \([^\n]*\))?|FAILED(?: \([^\n]*\))?|NO TESTS RAN)$", output, re.MULTILINE)
    return (int(match[1]), match[2]) if match else None


def details(name, output):
    if name in PYTHON_CHECKS:
        result = python_result(output)
        if not result:
            return []
        count, status = result
        return [f"{count:,} Python {'test' if count == 1 else 'tests'} run; {status}"]
    if name == "Rust and native regression tests":
        rows = []
        suite = None
        labels = ("passed", "failed", "ignored", "measured", "filtered out")
        names = {"unittests src/lib.rs": "Library", "unittests src/main.rs": "Binary", "tests/native.rs": "Native"}
        for line in output.splitlines():
            header = re.match(r"\s*Running (.+?) \(", line)
            if header:
                suite = names.get(header[1], header[1])
            elif line.strip().startswith("Doc-tests "):
                suite = line.strip()
            result = re.match(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in ", line)
            if suite and result:
                counts = "; ".join(f"{int(count):,} {label}" for count, label in zip(result.groups(), labels))
                rows.append(f"{suite}: {counts}")
                suite = None
        return rows
    if name == "native cleanup, stacks, contexts and scheduling":
        profiles = {}
        for match in re.finditer(r"^PASS .+: (debug|release|sanitized); (\d+) cases\b", output, re.MULTILINE):
            suites, cases = profiles.get(match[1], (0, 0))
            profiles[match[1]] = (suites + 1, cases + int(match[2]))
        return [f"{profile}: {cases:,} cases passed across {suites:,} {'suite' if suites == 1 else 'suites'}" for profile, (suites, cases) in profiles.items()]
    if name == "compiler conformance execution":
        match = re.search(r"^(\d+) passed; (\d+) unsupported; (\d+) failed \(debug and release\)\.$", output, re.MULTILINE)
        if match:
            passed, unsupported, failed = map(int, match.groups())
            return [f"{passed:,} required conformance passed; {unsupported:,} unsupported; {failed:,} failed (debug and release)."]
        return []
    prefixes = {
        "local documentation links": ("Checked ",),
        "conformance catalog metadata": ("Validated ",),
        "artifact schemas and example identities": ("Validated ",),
    }
    return [line for line in output.splitlines() if line.startswith(prefixes.get(name, ()))]


def summary(checks, results):
    width = max(len(check.name) for check in checks)
    python_counts = []
    print("\nVerification results:")
    for index, check in enumerate(checks):
        if index >= len(results):
            print(f"  {'NOT RUN':7}  {check.name:<{width}}")
            continue
        passed, elapsed, log = results[index]
        print(f"  {'PASS' if passed else 'FAIL':7}  {check.name:<{width}}  {elapsed:6.1f}s")
        if log and not passed:
            print(f"           Log: {log}")
        if log:
            try:
                output = log.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for line in details(check.name, output):
                print(f"           {line}")
            if check.name in PYTHON_CHECKS:
                result = python_result(output)
                if result:
                    python_counts.append(result[0])
    if python_counts:
        count = sum(python_counts)
        suites = len(python_counts)
        print(f"\nPython total: {count:,} {'test' if count == 1 else 'tests'} run across {suites} completed {'suite' if suites == 1 else 'suites'}.")


def main():
    parser = argparse.ArgumentParser(description="Verify repository contracts; compiler execution and editor runtimes are opt-in.")
    parser.add_argument("--editor", choices=("vim", "nvim", "both"))
    parser.add_argument("--compiler", action="store_true")
    parser.add_argument("--runtime", action="store_true", help="Check native cleanup, guarded stacks, contexts and bounded scheduling in debug, release and sanitizer builds.")
    parser.add_argument("--strict", action="store_true", help="Require every compiler conformance case; requires --compiler or --all.")
    parser.add_argument("--all", action="store_true", help="Include editor, compiler and native runtime verification.")
    parser.add_argument("--list", action="store_true", help="Print selected commands without running them.")
    parser.add_argument("--keep-going", action="store_true", help="Run remaining checks after failures and report every result.")
    parser.add_argument("--log-dir", type=Path, help="Keep detailed output in this directory and show concise progress; relative paths use the repository root.")
    parser.add_argument("--timeout", type=float, default=300, help="Maximum seconds for each command (default: 300).")
    args = parser.parse_args()
    if args.strict and not (args.compiler or args.all):
        parser.error("--strict requires --compiler or --all")
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be a finite positive number")
    compiler = args.compiler or args.all
    editor = "both" if args.all else args.editor
    runtime = args.runtime or args.all
    checks = plan(editor, compiler, args.strict, runtime)
    if args.list:
        for check in checks:
            print(f"{check.name}: {shlex.join(check.args)}")
        print("Plan only; no checks were run.")
        return 0
    if not compiler:
        print("Scope: repository contracts; no meowy source will be compiled or executed.", flush=True)
    elif not args.strict:
        print("Scope: repository contracts and compiler bootstrap; unsupported language cases remain visible.", flush=True)
    else:
        print("Scope: repository contracts and full reference catalog execution.", flush=True)
    if runtime:
        print("Native cleanup, guarded stacks, contexts, child waits, owned values and explicit scope close are selected; automatic cancellation/scope-exit joins and DWARF unwinding remain unqualified.", flush=True)
    if not editor:
        print("Editor runtimes are not selected; use --editor both to include them.", flush=True)
    log_dir = (ROOT / args.log_dir).resolve() if args.log_dir else None
    if log_dir:
        try:
            log_dir.mkdir(parents=True, exist_ok=True)
        except OSError as error:
            parser.error(str(error))
        print(f"Detailed logs: {log_dir}", flush=True)
    results = []
    for index, check in enumerate(checks):
        log = log_dir / f"{index + 1:02d}-{check.name.replace(' ', '-').replace(',', '')}.log" if log_dir else None
        started = time.monotonic()
        passed = run(check, timeout=args.timeout, log=log)
        results.append((passed, time.monotonic() - started, log))
        if not passed and not args.keep_going:
            print("Verification stopped at the failed check; subsequent checks were not run.")
            break
    summary(checks, results)
    failed = sum(not passed for passed, _, _ in results)
    if failed:
        print(f"{len(results) - failed} passed; {failed} failed; {len(checks) - len(results)} not run.")
        return 1
    print(f"All {len(checks)} selected checks passed. This does not qualify the complete v0.0.1 release.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
