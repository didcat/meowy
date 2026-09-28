"""Validate fixture metadata; deliberately does not execute meowy source."""

import json
import re
from pathlib import Path


def main():
    base = Path(__file__).resolve().parent
    catalog = json.loads((base / "cases.json").read_text())
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
            headings = re.findall(r"^#{1,6} (.+)$", reference.read_text(), re.MULTILINE)
            anchors = {
                re.sub(r"[^\w\- ]", "", title.lower()).replace(" ", "-")
                for title in headings
            }
            assert anchor in anchors, (case["id"], anchor)
    assert sources == set((base / "sources").glob("*.mwy")), "Unlisted source fixture"
    print(f"Validated {len(seen)} fixture records; no meowy source was executed.")
    from coverage import check
    check(base.parents[1])


if __name__ == "__main__":
    main()
