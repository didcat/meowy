import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parent
VERSION = "23.1.1"
CASES = 14
DIAGNOSTIC_CASES = 7
STACK_CASES = 10
CONTEXT_CASES = 10
SCHEDULER_CASES = 25
OWNED_CASES = 14
GENERATED_CASES = 6
GENERATED_OWNED_CASES = 7
STRING_CASES = 7


def invoke(args, timeout=60, env=None):
    process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               cwd=ROOT, env=env, start_new_session=True)
    try:
        out, err = process.communicate(timeout=timeout)
    except (subprocess.TimeoutExpired, KeyboardInterrupt):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.communicate()
        raise
    return subprocess.CompletedProcess(args, process.returncode,
                                       out.decode("utf-8", errors="replace"),
                                       err.decode("utf-8", errors="replace"))


def require(result, code=0):
    if result.returncode != code:
        raise RuntimeError(f"command failed (exit {result.returncode}): {' '.join(map(str, result.args))}\n"
                           f"{result.stdout}{result.stderr}")


def check_fatal(result, panicked, operation="newer"):
    require(result, -signal.SIGABRT)
    original = "panic P006: body failed" if panicked else "complete"
    expected = ("release-trigger\npanic[P008]: panic during cleanup\n"
                f"original: {original}\ncleanup: {operation}\nsecond: P006: release failed\n")
    if result.stdout or result.stderr != expected:
        raise RuntimeError(f"fatal cleanup evidence differs:\n{result.stdout}{result.stderr}")


def check_truncated_panic(result):
    require(result, -signal.SIGABRT)
    expected = ("panic[P008]: panic during cleanup\n"
                f"original: panic P006: {'b' * 256} [truncated from 300 bytes]\n"
                "cleanup: truncated messages\n"
                f"second: P006: {'c' * 256} [truncated from 260 bytes]\n")
    if result.stdout or result.stderr != expected:
        raise RuntimeError(f"truncated panic evidence differs:\n{result.stdout}{result.stderr}")


def check_guard(result, high):
    require(result, -signal.SIGSEGV)
    side = "high" if high else "low"
    expected = f"guard-{side}: SEGV_ACCERR at expected address on alternate stack\n"
    if result.stdout or result.stderr != expected:
        raise RuntimeError(f"guard fault evidence differs:\n{result.stdout}{result.stderr}")


def check_unjoined(result, phase, pending="unjoined children"):
    require(result, -signal.SIGABRT)
    expected = f"fatal runtime protocol: {phase} returned with {pending}\n"
    if result.stdout or result.stderr != expected:
        raise RuntimeError(f"unjoined child evidence differs:\n{result.stdout}{result.stderr}")


def check_cases(result, count, suite):
    require(result)
    if (result.stderr or not result.stdout.endswith(f"{count} {suite} cases passed\n") or
            result.stdout.count("PASS ") != count):
        raise RuntimeError(f"{suite} case evidence differs:\n{result.stdout}{result.stderr}")


def check_vendor(directory=ROOT / "vendor/boost-context"):
    manifest = json.loads((directory / "manifest.json").read_text())
    paths = {
        "make_x86_64_sysv_elf_gas.S": "src/asm/make_x86_64_sysv_elf_gas.S",
        "jump_x86_64_sysv_elf_gas.S": "src/asm/jump_x86_64_sysv_elf_gas.S",
        "fcontext.hpp": "include/boost/context/detail/fcontext.hpp",
        "LICENSE_1_0.txt": None,
    }
    if (manifest.get("repository") != "https://github.com/boostorg/context" or
            manifest.get("target") != "x86_64-linux-sysv-elf-lp64" or
            not re.fullmatch(r"boost-\d+\.\d+\.\d+", manifest.get("tag", "")) or
            not re.fullmatch(r"[0-9a-f]{40}", manifest.get("revision", "")) or
            set(manifest.get("files", {})) != set(paths)):
        raise RuntimeError("invalid pinned Boost.Context manifest")
    for name, item in manifest["files"].items():
        source = (f"https://raw.githubusercontent.com/boostorg/context/{manifest['revision']}/{paths[name]}"
                  if paths[name] else "https://www.boost.org/LICENSE_1_0.txt")
        if item.get("source") != source:
            raise RuntimeError(f"vendored Boost.Context provenance differs: {name}")
        actual = hashlib.sha256((directory / name).read_bytes()).hexdigest()
        if actual != item.get("sha256"):
            raise RuntimeError(f"vendored Boost.Context checksum differs: {name}")
    return directory


def check_asan_lifetime(result):
    require(result, -signal.SIGABRT)
    if (result.stdout or not result.stderr.startswith("asan-context: read returned fiber local\n") or
            "ERROR: AddressSanitizer: stack-use-after-return" not in result.stderr):
        raise RuntimeError(f"fiber lifetime sanitizer evidence differs:\n{result.stdout}{result.stderr}")


def check(clang, directory, sanitizers=True):
    version = invoke([clang, "--version"])
    require(version)
    if not re.search(rf"\bclang version {re.escape(VERSION)}\b", version.stdout):
        raise RuntimeError(f"runtime prototype requires Clang {VERSION}; got {version.stdout.strip()}")
    vendor = check_vendor()
    print("PASS pinned Boost.Context source and license checksums", flush=True)
    profiles = [("debug", ["-O0", "-g"]), ("release", ["-O2", "-DNDEBUG"])]
    if sanitizers:
        profiles.append(("sanitized", ["-O1", "-g", "-fsanitize=address,undefined",
                                        "-fno-sanitize-recover=all", "-fno-omit-frame-pointer"]))
    env = dict(os.environ, ASAN_OPTIONS="detect_leaks=1:halt_on_error=1:abort_on_error=1:detect_stack_use_after_return=1",
               UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    layout = None
    for name, flags in profiles:
        diagnostic = directory / f"diagnostic-{name}"
        print(f"CHECK runtime diagnostic snapshots: {name}", flush=True)
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "tests/diagnostic.cpp"),
                        "-o", str(diagnostic)]))
        check_cases(invoke([str(diagnostic)], env=env), DIAGNOSTIC_CASES, "diagnostic snapshot")
        check_truncated_panic(invoke([str(diagnostic), "--fatal-truncated"], env=env))
        sizes = invoke([str(diagnostic), "--layout"], env=env)
        require(sizes)
        if (sizes.stderr or not re.fullmatch(
                r"Panic=\d+ TaskOutcome=\d+ TaskSlot=\d+ TaskInfo=\d+ Joined=\d+ ChildFailure=\d+ ScopeClose=\d+\n",
                sizes.stdout) or (layout is not None and layout != sizes.stdout)):
            raise RuntimeError(f"diagnostic storage layout differs:\n{sizes.stdout}{sizes.stderr}")
        layout = sizes.stdout
        print(f"PASS runtime diagnostic snapshots: {name}; {DIAGNOSTIC_CASES} cases, exact fatal truncation; {layout.strip()}", flush=True)
        binary = directory / f"cleanup-{name}"
        args = [clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"),
                *flags, str(ROOT / "src/cleanup.cpp"), str(ROOT / "tests/cleanup.cpp"),
                "-o", str(binary)]
        print(f"CHECK runtime cleanup: {name}", flush=True)
        require(invoke(args))
        result = invoke([str(binary)], env=env)
        check_cases(result, CASES, "cleanup")
        check_fatal(invoke([str(binary), "--fatal-normal"], env=env), False)
        check_fatal(invoke([str(binary), "--fatal-panic"], env=env), True)
        print(f"PASS runtime cleanup: {name}; {CASES} cases and 2 fatal subprocesses", flush=True)
        generated = directory / f"generated-{name}"
        print(f"CHECK generated cleanup bridge: {name}", flush=True)
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/owned.cpp"), str(ROOT / "src/generated.cpp"),
                        str(ROOT / "tests/generated.cpp"), "-o", str(generated)]))
        check_cases(invoke([str(generated)], env=env), GENERATED_CASES, "generated cleanup")
        check_fatal(invoke([str(generated), "--fatal-normal"], env=env), False)
        check_fatal(invoke([str(generated), "--fatal-panic"], env=env), True)
        print(f"PASS generated cleanup bridge: {name}; {GENERATED_CASES} cases and 2 fatal subprocesses", flush=True)
        payload = directory / f"generated-owned-{name}"
        print(f"CHECK generated payload ownership: {name}", flush=True)
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/owned.cpp"), str(ROOT / "src/generated.cpp"),
                        str(ROOT / "tests/generated_owned.cpp"), "-o", str(payload)]))
        check_cases(invoke([str(payload)], env=env), GENERATED_OWNED_CASES, "generated ownership")
        check_fatal(invoke([str(payload), "--fatal-normal"], env=env), False, "owned fixture")
        check_fatal(invoke([str(payload), "--fatal-panic"], env=env), True, "owned fixture")
        print(f"PASS generated payload ownership: {name}; {GENERATED_OWNED_CASES} cases and 2 fatal subprocesses", flush=True)
        strings = directory / f"strings-{name}"
        print(f"CHECK owned strings: {name}", flush=True)
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/owned.cpp"),
                        str(ROOT / "src/generated.cpp"), str(ROOT / "src/strings.cpp"),
                        str(ROOT / "tests/strings.cpp"), "-o", str(strings)]))
        check_cases(invoke([str(strings)], env=env), STRING_CASES, "owned string")
        print(f"PASS owned strings: {name}; {STRING_CASES} cases", flush=True)
        stack = directory / f"stack-{name}"
        args = [clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                "-fno-exceptions", "-fno-rtti", "-I", str(ROOT / "include"),
                *flags, str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/stack_memory.cpp"),
                str(ROOT / "tests/stack_memory.cpp"), "-Wl,--wrap=mmap,--wrap=mprotect,--wrap=munmap",
                "-o", str(stack)]
        print(f"CHECK runtime stack allocation: {name}", flush=True)
        require(invoke(args))
        check_cases(invoke([str(stack)], env=env), STACK_CASES, "stack allocation")
        admission = invoke([str(stack), "--os-admission-failure"], env=env)
        require(admission)
        if admission.stderr or admission.stdout != "PASS kernel admission refusal: ENOMEM without owner\n":
            raise RuntimeError(f"kernel admission evidence differs:\n{admission.stdout}{admission.stderr}")
        guard_env = dict(env, ASAN_OPTIONS=env["ASAN_OPTIONS"] + ":handle_segv=0")
        check_guard(invoke([str(stack), "--guard-low"], env=guard_env), False)
        check_guard(invoke([str(stack), "--guard-high"], env=guard_env), True)
        print(f"PASS runtime stack allocation: {name}; {STACK_CASES} cases, kernel refusal and 2 guard subprocesses", flush=True)
        print(f"CHECK runtime contexts: {name}", flush=True)
        objects = []
        for source in ("make_x86_64_sysv_elf_gas.S", "jump_x86_64_sysv_elf_gas.S"):
            obj = directory / f"{name}-{source}.o"
            require(invoke([clang, "-fcf-protection=none", "-Dmake_fcontext=meowy_make_context_v0",
                            "-Djump_fcontext=meowy_jump_context_v0", "-c", str(vendor / source), "-o", str(obj)]))
            objects.append(str(obj))
        context = directory / f"context-{name}"
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-fcf-protection=none", "-fstack-protector-strong",
                        "-pthread", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/stack_memory.cpp"),
                        str(ROOT / "src/context.cpp"), str(ROOT / "tests/context.cpp"),
                        str(ROOT / "tests/context_registers.S"), *objects, "-o", str(context)]))
        check_cases(invoke([str(context)], env=env), CONTEXT_CASES, "context")
        check_fatal(invoke([str(context), "--fatal-context-cleanup"], env=env), True)
        if name == "sanitized":
            check_asan_lifetime(invoke([str(context), "--asan-use-after-return"], env=env))
        print(f"PASS runtime contexts: {name}; {CONTEXT_CASES} cases and fatal resumed cleanup" +
              ("; returned fiber local is detected by ASan" if name == "sanitized" else ""), flush=True)
        print(f"CHECK runtime scheduler: {name}", flush=True)
        scheduler = directory / f"scheduler-{name}"
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-fcf-protection=none", "-fstack-protector-strong",
                        "-pthread", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/stack_memory.cpp"),
                        str(ROOT / "src/context.cpp"), str(ROOT / "src/scheduler.cpp"),
                        str(ROOT / "src/owned.cpp"),
                        str(ROOT / "src/task_policy.cpp"), str(ROOT / "tests/scheduler.cpp"),
                        "-Wl,--wrap=mmap,--wrap=mprotect,--wrap=munmap", *objects, "-o", str(scheduler)]))
        check_cases(invoke([str(scheduler)], env=env), SCHEDULER_CASES, "scheduler")
        admission = invoke([str(scheduler), "--os-admission-failure"], env=env)
        require(admission)
        if admission.stderr or admission.stdout != "PASS scheduler kernel refusal: ENOMEM, no body, joined failure\n":
            raise RuntimeError(f"scheduler admission evidence differs:\n{admission.stdout}{admission.stderr}")
        check_fatal(invoke([str(scheduler), "--fatal-task-cleanup"], env=env), True, "task cleanup")
        check_unjoined(invoke([str(scheduler), "--unjoined-body"], env=env), "body")
        check_unjoined(invoke([str(scheduler), "--unjoined-cleanup"], env=env), "cleanup")
        check_unjoined(invoke([str(scheduler), "--unclosed-body"], env=env), "body", "unclosed task scopes")
        check_unjoined(invoke([str(scheduler), "--unclosed-cleanup"], env=env), "cleanup", "unclosed task scopes")
        print(f"PASS runtime scheduler: {name}; {SCHEDULER_CASES} cases, kernel refusal, fatal cleanup and child/scope protocol probes", flush=True)
        print(f"CHECK runtime owned values: {name}", flush=True)
        owned = directory / f"owned-{name}"
        require(invoke([clang, "-std=c++20", "-Wall", "-Wextra", "-Wpedantic", "-Werror",
                        "-fno-exceptions", "-fno-rtti", "-fcf-protection=none", "-fstack-protector-strong",
                        "-pthread", "-I", str(ROOT / "include"), *flags,
                        str(ROOT / "src/cleanup.cpp"), str(ROOT / "src/stack_memory.cpp"),
                        str(ROOT / "src/context.cpp"), str(ROOT / "src/scheduler.cpp"),
                        str(ROOT / "src/owned.cpp"), str(ROOT / "src/task_policy.cpp"),
                        str(ROOT / "tests/owned.cpp"), "-Wl,--wrap=mmap,--wrap=mprotect,--wrap=munmap",
                        *objects, "-o", str(owned)]))
        check_cases(invoke([str(owned)], env=env), OWNED_CASES, "owned value")
        check_fatal(invoke([str(owned), "--fatal-owned-cleanup"], env=env), True, "owned capture")
        check_fatal(invoke([str(owned), "--fatal-owned-admission"], env=env), False, "owned resource")
        check_fatal(invoke([str(owned), "--fatal-scope-owned-cleanup"], env=env), False, "owned resource")
        check_fatal(invoke([str(owned), "--fatal-owned-message-lifetime"], env=env), True, "owned capture")
        print(f"PASS runtime owned values: {name}; {OWNED_CASES} cases and 4 fatal owned-cleanup probes", flush=True)
    if not sanitizers:
        print("Sanitizers were explicitly disabled; sanitizer behavior was not checked.")
    print("Bounded single-worker prototype only; automatic cancellation/scope-exit joins, meowy owner lowering and DWARF unwinding remain pending; private generated cleanup ABI is tested.")


def main():
    parser = argparse.ArgumentParser(description="Build and check native cleanup, guarded stacks, pinned contexts and bounded scheduling.")
    parser.add_argument("--clang", default="/usr/bin/clang++", help=f"Clang {VERSION} executable")
    parser.add_argument("--build-dir", type=Path, help="Keep binaries in this directory; default uses temporary storage")
    parser.add_argument("--no-sanitizers", action="store_true", help="Run debug/release only; explicitly omit sanitizer validation")
    args = parser.parse_args()
    try:
        if args.build_dir:
            directory = args.build_dir.resolve()
            directory.mkdir(parents=True, exist_ok=True)
            check(args.clang, directory, not args.no_sanitizers)
        else:
            with tempfile.TemporaryDirectory(prefix="meowy-runtime-") as temp:
                check(args.clang, Path(temp), not args.no_sanitizers)
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"FAIL runtime prototype: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
