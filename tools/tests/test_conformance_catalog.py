import copy
import importlib.util
import tempfile
import unittest
from pathlib import Path


SPEC = importlib.util.spec_from_file_location("conformance_catalog", Path(__file__).resolve().parents[2] / "docs/conformance/check.py")
catalog = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(catalog)


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name) / "docs/conformance"
        (self.base / "sources").mkdir(parents=True)
        refs = self.base.parent / "reference"
        refs.mkdir()
        (refs / "diagnostic-codes.md").write_text("# Diagnostics\n`E207`\n")
        (refs / "syntax.md").write_text("# Syntax\n## Rule\n## Rule\n```\n## Fake\n```\n")
        (self.base / "sources/example.mwy").write_text("x:1\n")
        self.data = {"version": 1, "language_contract": 1, "target": "x86_64-unknown-linux-gnu", "cases": [
            {"id": "example", "phase": "check", "source": "sources/example.mwy", "expected": {"accepted": True}, "reference": "../reference/syntax.md#rule"}]}

    def test_accept_reject_run_and_duplicate_heading_references(self):
        for phase, expected in [("check", {"accepted": True}), ("check", {"accepted": False, "code": "E207"}), ("run", {"accepted": True, "stdout": "1\n"})]:
            data = copy.deepcopy(self.data)
            data["cases"][0].update(phase=phase, expected=expected, reference="../reference/syntax.md#rule-1")
            self.assertEqual(catalog.validate(data, self.base), 1)

    def test_catalog_shape_version_target_and_duplicate_ids_fail(self):
        for change in [lambda data: data.update(version=3), lambda data: data.update(target="other"),
                       lambda data: data.update(extra=True), lambda data: data["cases"].append(copy.deepcopy(data["cases"][0]))]:
            data = copy.deepcopy(self.data)
            change(data)
            with self.assertRaises(AssertionError):
                catalog.validate(data, self.base)

    def test_wrong_phase_expected_shape_and_unknown_code_fail(self):
        for update in [{"phase": "build"}, {"id": "invalid-id"}, {"expected": {"accepted": 1}},
                       {"expected": {"accepted": False, "code": "E999"}},
                       {"phase": "run", "expected": {"accepted": True}},
                       {"phase": "run", "expected": {"accepted": True, "stdout": 1}},
                       {"phase": "run", "expected": {"accepted": False, "code": "E207"}}]:
            data = copy.deepcopy(self.data)
            data["cases"][0].update(update)
            with self.subTest(update=update), self.assertRaises(AssertionError):
                catalog.validate(data, self.base)

    def test_sources_cannot_escape_or_be_empty(self):
        for source in ["../reference/syntax.md", "sources/../../outside.mwy"]:
            data = copy.deepcopy(self.data)
            data["cases"][0]["source"] = source
            with self.assertRaises(AssertionError):
                catalog.validate(data, self.base)
        (self.base / "sources/example.mwy").write_text(" \n")
        with self.assertRaises(AssertionError):
            catalog.validate(self.data, self.base)

    def test_unlisted_nested_sources_fail(self):
        (self.base / "sources/nested").mkdir()
        (self.base / "sources/nested/orphan.mwy").write_text("x:2\n")
        with self.assertRaisesRegex(AssertionError, "Unlisted"):
            catalog.validate(self.data, self.base)

    def test_version_two_requires_declared_companions_and_keeps_version_one_closed(self):
        (self.base / "sources/helper.mwy").write_text("->n:2\n")
        self.data["cases"][0]["companions"] = ["sources/helper.mwy"]
        with self.assertRaises(AssertionError):
            catalog.validate(self.data, self.base)
        self.data["version"] = 2
        self.assertEqual(catalog.validate(self.data, self.base), 1)
        self.data["cases"][0]["companions"].append("sources/helper.mwy")
        with self.assertRaisesRegex(AssertionError, "duplicate"):
            catalog.validate(self.data, self.base)

    def test_missing_and_fenced_heading_references_fail(self):
        for reference in ["../reference/missing.md", "../reference/syntax.md#fake", "../../outside.md"]:
            data = copy.deepcopy(self.data)
            data["cases"][0]["reference"] = reference
            with self.subTest(reference=reference), self.assertRaises(AssertionError):
                catalog.validate(data, self.base)


if __name__ == "__main__":
    unittest.main()
