import json
import pathlib
import subprocess
import tempfile
import unittest
from unittest import mock

import conformance


class ContractTests(unittest.TestCase):
    @staticmethod
    def result(code=0, messages=(), stdout=b""):
        return subprocess.CompletedProcess([], code, stdout, "\n".join(map(json.dumps, messages)).encode())

    def check(self, results, expected=None, phase="check", blocked=None):
        case = {"id": "new_case", "phase": phase, "expected": expected or {"accepted": True}}
        with mock.patch.object(conformance, "invoke", side_effect=results) as invoke:
            result = conformance.check_case(pathlib.Path("compiler"), case, pathlib.Path("source.mwy"), blocked)
        return result, invoke.call_args_list

    def test_new_cases_are_required_and_unknown_capabilities_fail(self):
        pending = self.result(1, [{"code": "B001", "message": "feature pending"}])
        with self.assertRaisesRegex(AssertionError, "required case"):
            self.check([pending])
        with self.assertRaisesRegex(AssertionError, "diagnostic changed"):
            self.check([pending], blocked="different feature")

    def test_pinned_unsupported_cases_require_both_profiles_and_promotion(self):
        pending = self.result(1, [{"code": "B001", "message": "feature pending"}])
        gap, calls = self.check([pending, pending], blocked="feature pending")
        self.assertEqual(gap, "feature pending")
        self.assertEqual([call.args[-1] for call in calls], ["debug", "release"])
        with self.assertRaisesRegex(AssertionError, "profiles disagree"):
            self.check([pending, self.result()], blocked="feature pending")
        with self.assertRaisesRegex(AssertionError, "remove its unsupported exception"):
            self.check([self.result(), self.result()], blocked="feature pending")

    def test_pending_diagnostic_cannot_hide_other_errors(self):
        for code in ["E207", "B001", "F001"]:
            with self.subTest(code=code), self.assertRaises(AssertionError):
                self.check([self.result(1, [{"code": "B001", "message": "pending"},
                                           {"code": code, "message": "another"}])], blocked="pending")

    def test_rejections_require_the_primary_language_code(self):
        expected = {"accepted": False, "code": "E207"}
        good = self.result(1, [{"code": "E207", "message": "wording may change"}])
        self.assertIsNone(self.check([good, good], expected)[0])
        for bad in [self.result(), self.result(1), self.result(1, [{"code": "E208", "message": "wrong"}])]:
            with self.subTest(bad=bad), self.assertRaises(AssertionError):
                self.check([bad], expected)

    def test_malformed_diagnostics_and_abnormal_exits_fail(self):
        for bad in [self.result(-11), self.result(2), self.result(stdout=b"unexpected"),
                    self.result(1, [None]), self.result(1, [{"code": 12, "message": "bad"}]),
                    self.result(1, [{"code": "E207"}]),
                    subprocess.CompletedProcess([], 1, b"", b"not json")]:
            with self.subTest(bad=bad), self.assertRaises((AssertionError, ValueError)):
                self.check([bad])

    def test_runtime_requires_exact_stdout_no_stderr_and_zero_exit(self):
        expected = {"accepted": True, "stdout": "ok\n"}
        good = self.result(stdout=b"ok\n")
        result, calls = self.check([self.result(), good, self.result(), good], expected, "run")
        self.assertIsNone(result)
        self.assertEqual([call.args[1] for call in calls], ["check", "run", "check", "run"])
        for bad in [self.result(stdout=b"ok"), self.result(1, stdout=b"ok\n"),
                    self.result(messages=[{"code": "B001", "message": "pending"}], stdout=b"ok\n")]:
            with self.subTest(bad=bad), self.assertRaises(AssertionError):
                self.check([self.result(), bad], expected, "run")

    def test_support_manifest_rejects_orphan_empty_and_invalid_exceptions(self):
        catalog = {"cases": [{"id": "known"}]}
        with tempfile.TemporaryDirectory() as temp:
            path = pathlib.Path(temp) / "support.json"
            for data in [{"version": 2, "unsupported": {}},
                         {"version": 1, "unsupported": {"missing": "pending"}},
                         {"version": 1, "unsupported": {"known": ""}},
                         {"version": 1, "unsupported": []}]:
                with self.subTest(data=data), self.assertRaises(ValueError):
                    path.write_text(json.dumps(data))
                    conformance.load_support(catalog, path)
            path.write_text(json.dumps({"version": 1, "unsupported": {"known": "pending"}}))
            self.assertEqual(conformance.load_support(catalog, path), {"known": "pending"})


if __name__ == "__main__":
    unittest.main()
