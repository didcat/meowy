import argparse
import json
import os
import pathlib
import re
import signal
import subprocess
import tempfile
from conformance_fixtures import stage_case


ROOT = pathlib.Path(__file__).resolve().parents[2]
CATALOG = ROOT / "docs/conformance/cases.json"
SUPPORT = ROOT / "compiler/tests/conformance_support.json"
PANIC_CODES = set(re.findall(r"`(P\d{3})`", (ROOT / "docs/reference/diagnostic-codes.md").read_text()))
PANIC_RECORD = re.compile(rb'panic\[(P[0-9]{3})\]: [^\r\n]* at (?:"(?:[^"\\\r\n]|\\.)*" )?bytes ([0-9]+)\.\.([0-9]+)\n')


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


def check_run(result, expected, profile):
    if result.stdout != expected["stdout"].encode():
        raise AssertionError(f"{profile}: unexpected runtime stdout {result.stdout!r}")
    if "panic" not in expected:
        if result.returncode != 0 or result.stderr:
            raise AssertionError(f"{profile}: run exit {result.returncode}, stderr={result.stderr!r}")
        return
    code = expected["panic"]
    if not isinstance(code, str) or code not in PANIC_CODES or type(expected.get("exit")) is not int or expected["exit"] != 1:
        raise AssertionError("invalid runtime panic expectation")
    try:
        result.stderr.decode("utf-8")
    except UnicodeDecodeError as error:
        raise AssertionError("malformed panic encoding") from error
    record = PANIC_RECORD.fullmatch(result.stderr)
    if (result.returncode != expected["exit"] or record is None or
            record[1].decode() != code or int(record[2]) > int(record[3])):
        raise AssertionError(f"{profile}: expected {code}/exit 1, got exit {result.returncode}, stderr={result.stderr!r}")


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
            check_run(result, expected, profile)
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
    version = catalog["version"]
    if (type(version) is not int or version not in (1, 2, 3) or
            (version == 1 and any("companions" in case for case in catalog["cases"])) or
            (version < 3 and any("panic" in case["expected"] for case in catalog["cases"]))):
        raise ValueError("unsupported conformance catalog format")
    gaps = load_support(catalog)
    passed = 0
    pending = 0
    failed = 0
    for case in catalog["cases"]:
        try:
            with tempfile.TemporaryDirectory(prefix="meowy-conformance-") as temp:
                source = stage_case(case, CATALOG.parent, pathlib.Path(temp))
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
