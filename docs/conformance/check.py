"""Validate fixture metadata; deliberately does not execute meowy source."""

import json
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from check_links import anchors


def validate(catalog, base):
    assert set(catalog) == {"version", "language_contract", "target", "cases"}
    assert catalog["version"] == catalog["language_contract"] == 1
    assert catalog["target"] == "x86_64-unknown-linux-gnu"
    codes = set(
        re.findall(
            r"`(E\d{3})`", (base.parent / "reference/diagnostic-codes.md").read_text()
        )
    )
    seen = set()
    sources = set()
    for case in catalog["cases"]:
        assert set(case) == {"id", "phase", "source", "expected", "reference"}, case
        assert re.fullmatch(r"[a-z][a-z0-9_]*", case["id"]), case
        assert case["id"] not in seen, case["id"]
        seen.add(case["id"])
        assert case["phase"] in {"check", "run"}, case
        source = (base / case["source"]).resolve()
        assert source.is_relative_to(base / "sources") and source.suffix == ".mwy", case
        assert source.read_text(encoding="utf-8").strip(), case
        sources.add(source)
        expected = case["expected"]
        assert type(expected["accepted"]) is bool, case
        if not expected["accepted"]:
            assert case["phase"] == "check" and set(expected) == {
                "accepted",
                "code",
            }, case
            assert expected["code"] in codes, case
        elif case["phase"] == "run":
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
