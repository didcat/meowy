"""Validate fixture metadata; deliberately does not execute meowy source."""

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path[:0] = [str(ROOT / "tools"), str(ROOT / "compiler/tests")]
from check_links import anchors
from conformance_fixtures import case_files


def validate(catalog, base):
    assert set(catalog) == {"version", "language_contract", "target", "cases"}
    assert type(catalog["version"]) is int and catalog["version"] in (1, 2, 3)
    assert type(catalog["language_contract"]) is int and catalog["language_contract"] == 1
    assert catalog["target"] == "x86_64-unknown-linux-gnu"
    codes = set(
        re.findall(
            r"`(E\d{3})`", (base.parent / "reference/diagnostic-codes.md").read_text()
        )
    )
    panics = set(re.findall(r"`(P\d{3})`", (base.parent / "reference/diagnostic-codes.md").read_text()))
    seen = set()
    sources = set()
    for case in catalog["cases"]:
        fields = {"id", "phase", "source", "expected", "reference"}
        assert set(case) == fields or (catalog["version"] >= 2 and set(case) == fields | {"companions"}), case
        assert re.fullmatch(r"[a-z][a-z0-9_]*", case["id"]), case
        assert case["id"] not in seen, case["id"]
        seen.add(case["id"])
        assert case["phase"] in {"check", "run"}, case
        try:
            sources.update(source for source, _ in case_files(case, base))
        except ValueError as error:
            raise AssertionError(f"{case['id']}: {error}") from error
        expected = case["expected"]
        assert type(expected["accepted"]) is bool, case
        if not expected["accepted"]:
            assert case["phase"] == "check" and set(expected) == {
                "accepted",
                "code",
            }, case
            assert expected["code"] in codes, case
        elif case["phase"] == "run":
            if "panic" in expected:
                assert catalog["version"] == 3 and set(expected) == {"accepted", "stdout", "panic", "exit"}, case
                assert isinstance(expected["panic"], str) and expected["panic"] in panics, case
                assert type(expected["exit"]) is int and expected["exit"] == 1, case
            else:
                assert set(expected) == {"accepted", "stdout"}, case
            assert isinstance(expected["stdout"], str), case
        else:
            assert set(expected) == {"accepted"}, case
        path, separator, anchor = case["reference"].partition("#")
        reference = (base / path).resolve()
        assert reference.is_relative_to(base.parent) and reference.is_file(), case
        if separator:
            assert anchor in anchors(reference.read_text()), (case["id"], anchor)
    assert sources == set((base / "sources").rglob("*.mwy")), "Unlisted source fixture"
    return len(seen)


def main():
    base = Path(__file__).resolve().parent
    count = validate(json.loads((base / "cases.json").read_text()), base)
    print(f"Validated {count} fixture records; no meowy source was executed.")
    from coverage import check
    check(ROOT)


if __name__ == "__main__":
    main()
