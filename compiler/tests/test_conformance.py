import contextlib
import io
import json
import pathlib
import signal
import subprocess
import unittest
from unittest import mock

import conformance


class HarnessTests(unittest.TestCase):
    def test_capability_diagnostic_does_not_hide_compiler_failure(self):
        case = {"id": "future", "phase": "check", "expected": {"accepted": True}}
        errors = [{"code": "B001", "message": "pending"}, {"code": "F001", "message": "fault"}]
        result = subprocess.CompletedProcess([], 1, b"", "\n".join(map(json.dumps, errors)).encode())
        with mock.patch.object(conformance, "invoke", return_value=result):
            with self.assertRaisesRegex(AssertionError, "infrastructure failure"):
                conformance.check_case(pathlib.Path("compiler"), case, pathlib.Path("source.mwy"))

    def test_empty_capability_message_still_fails_strict_gate(self):
        with mock.patch("sys.argv", ["conformance.py", "--strict"]):
            with mock.patch.object(conformance, "check_case", return_value=""):
                with contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(conformance.main(), 1)
        count = len(json.loads(conformance.CATALOG.read_text())["cases"])
        self.assertIn(f"0 passed; {count} unsupported; 0 failed", output.getvalue())

    def test_profile_support_disagreement_is_rejected(self):
        case = {"id": "future", "phase": "check", "expected": {"accepted": True}}
        result = [subprocess.CompletedProcess([], 0, b"", b""),
                  subprocess.CompletedProcess([], 1, b"", b'{"code":"B001","message":"pending"}')]
        with mock.patch.object(conformance, "invoke", side_effect=result):
            with self.assertRaisesRegex(AssertionError, "profiles disagree"):
                conformance.check_case(pathlib.Path("compiler"), case, pathlib.Path("source.mwy"))

    def test_timeout_kills_process_group_and_reaps_driver(self):
        process = mock.Mock(pid=1234)
        process.communicate.side_effect = [subprocess.TimeoutExpired("compiler", 20), (b"", b"")]
        with mock.patch.object(conformance.subprocess, "Popen", return_value=process) as popen:
            with mock.patch.object(conformance.os, "killpg") as kill:
                with self.assertRaises(subprocess.TimeoutExpired):
                    conformance.invoke(pathlib.Path("compiler"), "run", pathlib.Path("source.mwy"), "debug")
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        kill.assert_called_once_with(1234, signal.SIGKILL)
        self.assertEqual(process.communicate.call_count, 2)


if __name__ == "__main__":
    unittest.main()
