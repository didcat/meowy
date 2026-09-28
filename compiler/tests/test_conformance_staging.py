import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import conformance
from conformance_fixtures import stage_case


class StagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.bundle = self.base / "sources/first"
        (self.bundle / "sub").mkdir(parents=True)
        (self.bundle / "main.mwy").write_bytes(b'm:@"./sub/helper.mwy"\r\n')
        (self.bundle / "sub/helper.mwy").write_bytes(b"#\xc3\xa9#\r\n->n:2\r\n")
        (self.bundle / "unlisted.mwy").write_text("must not be staged")
        self.case = {"id": "first", "source": "sources/first/main.mwy", "companions": ["sources/first/sub/helper.mwy"], "phase": "check", "expected": {"accepted": True}}
        self.dest = self.base / "staged"
        self.dest.mkdir()

    def test_staging_keeps_bytes_layout_and_only_declared_files(self):
        source = stage_case(self.case, self.base, self.dest)
        self.assertEqual(source, self.dest / "main.mwy")
        self.assertEqual(source.read_bytes(), (self.bundle / "main.mwy").read_bytes())
        self.assertEqual((self.dest / "sub/helper.mwy").read_bytes(), (self.bundle / "sub/helper.mwy").read_bytes())
        self.assertFalse((self.dest / "unlisted.mwy").exists())

    def test_invalid_assets_fail_before_any_staged_write(self):
        for companions in [["sources/first/missing.mwy"], self.case["companions"] * 2, [self.case["source"]]]:
            with self.subTest(companions=companions), self.assertRaises(ValueError):
                stage_case({**self.case, "companions": companions}, self.base, self.dest)
            self.assertEqual(list(self.dest.iterdir()), [])

    def test_staging_never_overwrites_an_occupied_directory(self):
        existing = self.dest / "main.mwy"
        existing.write_bytes(b"preserve")
        with self.assertRaisesRegex(ValueError, "empty directory"):
            stage_case(self.case, self.base, self.dest)
        self.assertEqual(existing.read_bytes(), b"preserve")

    def run_catalog(self, cases, inspect, version=2):
        path = self.base / "cases.json"
        path.write_text(json.dumps({"version": version, "cases": cases}))
        with mock.patch.object(conformance, "CATALOG", path), mock.patch.object(conformance, "load_support", return_value={}), mock.patch.object(conformance, "check_case", side_effect=inspect), mock.patch("sys.argv", ["conformance.py"]), contextlib.redirect_stdout(io.StringIO()) as output:
            code = conformance.main()
        return code, output.getvalue()

    def test_runner_isolates_cases_and_cleans_each_directory(self):
        second = self.base / "sources/second"
        second.mkdir()
        (second / "main.mwy").write_text("x:1")
        other = {**self.case, "id": "second", "source": "sources/second/main.mwy"}
        other.pop("companions")
        directories = []

        def inspect(compiler, case, source, blocked):
            if directories:
                self.assertFalse(directories[0].exists())
            directories.append(source.parent)
            self.assertEqual((source.parent / "sub/helper.mwy").exists(), case["id"] == "first")
            self.assertIsNone(blocked)

        code, output = self.run_catalog([self.case, other], inspect)
        self.assertEqual(code, 0)
        self.assertIn("2 passed; 0 unsupported; 0 failed", output)
        self.assertNotEqual(*directories)
        self.assertTrue(all(not path.exists() for path in directories))

    def test_missing_declared_asset_is_failure_and_does_not_skip_later_cases(self):
        broken = {**self.case, "id": "broken", "companions": ["sources/first/missing.mwy"]}
        inspect = mock.Mock(return_value=None)
        code, output = self.run_catalog([broken, self.case], inspect)
        self.assertEqual(code, 1)
        self.assertIn("FAIL broken: missing fixture", output)
        self.assertIn("1 passed; 0 unsupported; 1 failed", output)
        self.assertEqual(inspect.call_count, 1)

    def test_runner_keeps_legacy_cases_and_rejects_unsupported_catalog_versions(self):
        single = {**self.case}
        single.pop("companions")
        self.assertEqual(self.run_catalog([single], mock.Mock(return_value=None), version=1)[0], 0)
        for version in [1, 3]:
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, "catalog format"):
                self.run_catalog([self.case], mock.Mock(), version=version)


if __name__ == "__main__":
    unittest.main()
