import argparse
import json
import os
import pathlib
import signal
import subprocess
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[2]
CATALOG = ROOT / "docs/conformance/cases.json"
SUPPORT = ROOT / "compiler/tests/conformance_support.json"


def load_support(catalog, path=SUPPORT):
    data = json.loads(path.read_text())
    if not isinstance(data, dict) or set(data) != {"version", "unsupported"} or data["version"] != 1:
        raise ValueError("invalid conformance support manifest")
    gaps = data["unsupported"]
    ids = {case["id"] for case in catalog["cases"]}
    if not isinstance(gaps, dict) or not set(gaps) <= ids:
        raise ValueError("unknown unsupported case")
    if any(not isinstance(reason, str) or not reason.strip() for reason in gaps.values()):
        raise ValueError("unsupported cases need a diagnostic reason")
    return gaps



def invoke(compiler, action, source, profile):
    args = [str(compiler), action, str(source), "--standalone", "--quiet", "--json",
            "--profile", profile]
    process = subprocess.Popen(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    try:
        stdout, stderr = process.communicate(timeout=20)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.communicate()
        raise
    return subprocess.CompletedProcess(args, process.returncode, stdout, stderr)


def check_case(compiler, case, source, blocked=None):
    expected = case["expected"]
    unsupported = None
    previous = None
    for profile in ("debug", "release"):
        result = invoke(compiler, "check", source, profile)
        if result.stdout:
            raise AssertionError("checking wrote application stdout")
        messages = [json.loads(line) for line in result.stderr.decode().splitlines()]
        if any(not isinstance(message, dict) or not isinstance(message.get("code"), str)
               or not isinstance(message.get("message"), str) for message in messages):
            raise AssertionError("malformed compiler diagnostic")
        codes = [message["code"] for message in messages]
        if result.returncode not in (0, 1) or any(code.startswith(("B", "F", "P", "T")) and code != "B001" for code in codes):
            raise AssertionError(f"infrastructure failure: exit {result.returncode}, {codes}")
        pending = result.returncode == 1 and bool(codes) and codes[0] == "B001"
        if previous is not None and previous != pending:
            raise AssertionError("profiles disagree about support")
        previous = pending
        if pending:
            if blocked is None:
                raise AssertionError("required case became unsupported")
            if codes != ["B001"] or messages[0]["message"] != blocked:
                raise AssertionError("unsupported diagnostic changed")
            unsupported = blocked
            continue
        if not expected["accepted"]:
            if result.returncode != 1 or not codes or codes[0] != expected["code"]:
                raise AssertionError(f"{profile}: expected {expected['code']}, got exit {result.returncode}, {codes}")
            continue
        if result.returncode != 0 or codes:
            raise AssertionError(f"{profile}: accepted source failed with {codes}")
        if case["phase"] == "run":
            result = invoke(compiler, "run", source, profile)
            if result.returncode != 0 or result.stdout != expected["stdout"].encode() or result.stderr:
                raise AssertionError(f"{profile}: run exit {result.returncode}, stdout={result.stdout!r}, stderr={result.stderr!r}")
    if blocked is not None and unsupported is None:
        raise AssertionError("case now passes; remove its unsupported exception")
    return unsupported


def main():
    parser = argparse.ArgumentParser(description="Execute real compiler conformance; bootstrap gaps are reported separately.")
    parser.add_argument("--compiler", type=pathlib.Path, default=ROOT / "compiler/target/debug/meowy")
    parser.add_argument("--strict", action="store_true", help="Fail unless the entire reference catalog passes.")
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    catalog = json.loads(CATALOG.read_text())
    gaps = load_support(catalog)
    passed = 0
    pending = 0
    failed = 0
    with tempfile.TemporaryDirectory(prefix="meowy-conformance-") as temp:
        for case in catalog["cases"]:
            source = pathlib.Path(temp) / (case["id"] + ".mwy")
            source.write_bytes((CATALOG.parent / case["source"]).read_bytes())
            try:
                gap = check_case(compiler, case, source, gaps.get(case["id"]))
                if gap is not None:
                    pending += 1
                    print(f"UNSUPPORTED {case['id']}: {gap}")
                else:
                    passed += 1
                    print(f"PASS {case['id']}")
            except (AssertionError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
                failed += 1
                print(f"FAIL {case['id']}: {error}")
    print(f"{passed} passed; {pending} unsupported; {failed} failed (debug and release).")
    if pending:
        print("This is a bootstrap result. The full language conformance gate has NOT passed.")
    return 1 if failed or (args.strict and pending) else 0


if __name__ == "__main__":
    raise SystemExit(main())
