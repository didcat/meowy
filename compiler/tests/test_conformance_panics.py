import pathlib
import subprocess
import unittest
from unittest import mock

import conformance


class PanicTests(unittest.TestCase):
    @staticmethod
    def result(code=0, stdout=b"", stderr=b""):
        return subprocess.CompletedProcess([], code, stdout, stderr)

    def setUp(self):
        self.expected = {"accepted": True, "stdout": "before\n", "panic": "P002", "exit": 1}
        self.case = {"id": "panic_case", "phase": "run", "expected": self.expected}
        self.panic = self.result(1, b"before\n", b"panic[P002]: overflow at bytes 10..15\n")

    def check(self, results, blocked=None):
        with mock.patch.object(conformance, "invoke", side_effect=results) as invoke:
            gap = conformance.check_case(pathlib.Path("compiler"), self.case, pathlib.Path("source.mwy"), blocked)
        return gap, invoke.call_args_list

    def test_panic_requires_successful_checking_and_matches_each_profile(self):
        located = self.result(1, b"before\n", b'panic[P002]: different wording at "sub/value.mwy" bytes 0..0\n')
        gap, calls = self.check([self.result(), self.panic, self.result(), located])
        self.assertIsNone(gap)
        self.assertEqual([(call.args[1], call.args[-1]) for call in calls], [("check", "debug"), ("run", "debug"), ("check", "release"), ("run", "release")])

    def test_wrong_panic_profile_and_abnormal_exits_fail(self):
        for code in [0, 2, -11, 137, 139]:
            with self.subTest(code=code), self.assertRaises(AssertionError):
                conformance.check_run(self.result(code, self.panic.stdout, self.panic.stderr), self.expected, "debug")
        wrong = self.result(1, b"before\n", b"panic[P001]: bounds at bytes 10..15\n")
        with self.assertRaisesRegex(AssertionError, "release"):
            self.check([self.result(), self.panic, self.result(), wrong])

    def test_exact_output_excludes_missing_or_post_failure_effects(self):
        for output in [b"", b"before", b"before\nafter\n"]:
            with self.subTest(output=output), self.assertRaisesRegex(AssertionError, "stdout"):
                conformance.check_run(self.result(1, output, self.panic.stderr), self.expected, "debug")

    def test_malformed_mixed_or_nonpanic_stderr_cannot_satisfy_expectations(self):
        for stderr in [b"", b'{"code":"B001","message":"pending"}', b'{"code":"F001","message":"fault"}',
                       b"panic[P002]: overflow\n", b"panic[P002]: overflow at bytes 15..10\n",
                       b"panic[P002]: overflow at bytes 10..15", b"error: " + self.panic.stderr,
                       b"panic[P002]: \xff at bytes 10..15\n",
                       self.panic.stderr + b'{"code":"F001","message":"fault"}\n', self.panic.stderr * 2]:
            with self.subTest(stderr=stderr), self.assertRaises(AssertionError):
                conformance.check_run(self.result(1, b"before\n", stderr), self.expected, "debug")

    def test_checking_rejections_and_run_capability_errors_are_not_panics(self):
        pending = self.result(1, stderr=b'{"code":"B001","message":"pending"}')
        error = self.result(1, stderr=b'{"code":"E107","message":"static division"}')
        fault = self.result(1, stderr=b'{"code":"F001","message":"fault"}')
        early = self.result(1, stderr=b'{"code":"P002","message":"not a checking result"}')
        for result in [pending, error, fault, early]:
            with self.assertRaises(AssertionError):
                self.check([result])
        gap, calls = self.check([pending, pending], blocked="pending")
        self.assertEqual(gap, "pending")
        self.assertTrue(all(call.args[1] == "check" for call in calls))
        with self.assertRaises(AssertionError):
            self.check([self.result(), pending])

    def test_runtime_timeouts_propagate_as_failures(self):
        with self.assertRaises(subprocess.TimeoutExpired):
            self.check([self.result(), subprocess.TimeoutExpired("run", 20)])

    def test_direct_runner_rejects_invalid_panic_expectations(self):
        for change in [{"panic": "P999"}, {"panic": "F001"}, {"panic": None}, {"exit": True}, {"exit": 139}]:
            with self.subTest(change=change), self.assertRaises(AssertionError):
                conformance.check_run(self.panic, {**self.expected, **change}, "debug")


if __name__ == "__main__":
    unittest.main()
