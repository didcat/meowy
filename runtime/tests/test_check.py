import subprocess
import json
import shutil
import signal
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check


class CheckTests(unittest.TestCase):
    def test_arbitrary_failure_does_not_count_as_fatal_panic(self):
        for code in (0, 1, 3, -signal.SIGSEGV):
            with self.subTest(code=code):
                result = subprocess.CompletedProcess(["binary"], code, "", "panic[P008]")
                with self.assertRaises(RuntimeError):
                    check.check_fatal(result, False)

    def test_abort_requires_complete_cleanup_evidence(self):
        result = subprocess.CompletedProcess(["binary"], -signal.SIGABRT, "", "panic[P008]")
        with self.assertRaisesRegex(RuntimeError, "evidence differs"):
            check.check_fatal(result, True)

    def test_original_panic_is_not_lost(self):
        error = ("release-trigger\npanic[P008]: panic during cleanup\n"
                 "original: panic P006: body failed\ncleanup: newer\nsecond: P006: release failed\n")
        result = subprocess.CompletedProcess(["binary"], -signal.SIGABRT, "", error)
        check.check_fatal(result, True)
        with self.assertRaises(RuntimeError):
            check.check_fatal(result, False)

    def test_truncated_panic_requires_both_owned_messages_and_lengths(self):
        error = ("panic[P008]: panic during cleanup\n"
                 f"original: panic P006: {'b' * 256} [truncated from 300 bytes]\n"
                 "cleanup: truncated messages\n"
                 f"second: P006: {'c' * 256} [truncated from 260 bytes]\n")
        result = subprocess.CompletedProcess(["binary"], -signal.SIGABRT, "", error)
        check.check_truncated_panic(result)
        for code, out, text in ((0, "", error), (-signal.SIGSEGV, "", error),
                                (-signal.SIGABRT, "unexpected", error),
                                (-signal.SIGABRT, "", error.replace("300 bytes", "256 bytes")),
                                (-signal.SIGABRT, "", error.replace(" [truncated from 260 bytes]", ""))):
            with self.subTest(code=code, out=out, text=text):
                with self.assertRaises(RuntimeError):
                    check.check_truncated_panic(subprocess.CompletedProcess(["binary"], code, out, text))

    def test_wrong_toolchain_fails_before_build(self):
        result = subprocess.CompletedProcess(["clang"], 0, "clang version 22.1.8\n", "")
        with mock.patch.object(check, "invoke", return_value=result) as invoke:
            with self.assertRaisesRegex(RuntimeError, "requires Clang 23.1.1"):
                check.check("clang", Path("/tmp"))
        invoke.assert_called_once_with(["clang", "--version"])

    def test_timeout_kills_group_and_reaps_process(self):
        process = mock.Mock(pid=1234)
        process.communicate.side_effect = [subprocess.TimeoutExpired("runtime", 60), (b"", b"")]
        with mock.patch.object(check.subprocess, "Popen", return_value=process) as popen:
            with mock.patch.object(check.os, "killpg") as kill:
                with self.assertRaises(subprocess.TimeoutExpired):
                    check.invoke(["runtime"])
        self.assertTrue(popen.call_args.kwargs["start_new_session"])
        kill.assert_called_once_with(1234, signal.SIGKILL)
        self.assertEqual(process.communicate.call_count, 2)

    def test_guard_crash_requires_address_and_protection_evidence(self):
        for code, error in ((1, ""), (-signal.SIGABRT, ""), (-signal.SIGSEGV, "segmentation fault")):
            with self.subTest(code=code, error=error):
                result = subprocess.CompletedProcess(["binary"], code, "", error)
                with self.assertRaises(RuntimeError):
                    check.check_guard(result, False)

    def test_guard_side_must_match_expected_boundary(self):
        error = "guard-low: SEGV_ACCERR at expected address on alternate stack\n"
        result = subprocess.CompletedProcess(["binary"], -signal.SIGSEGV, "", error)
        check.check_guard(result, False)
        with self.assertRaises(RuntimeError):
            check.check_guard(result, True)

    def test_native_suite_requires_count_and_successful_exit(self):
        for code, out, error in ((1, "PASS sample\n1 stack allocation cases passed\n", ""),
                                 (0, "0 stack allocation cases passed\n", ""),
                                 (0, "PASS sample\n1 stack allocation cases passed\n", "sanitizer failure")):
            with self.subTest(code=code, out=out, error=error):
                result = subprocess.CompletedProcess(["binary"], code, out, error)
                with self.assertRaises(RuntimeError):
                    check.check_cases(result, 1, "stack allocation")

    def test_vendored_source_tampering_fails_checksum(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp) / "vendor"
            shutil.copytree(check.ROOT / "vendor/boost-context", directory)
            check.check_vendor(directory)
            with (directory / "jump_x86_64_sysv_elf_gas.S").open("ab") as source:
                source.write(b"\n")
            with self.assertRaisesRegex(RuntimeError, "checksum differs"):
                check.check_vendor(directory)

    def test_vendor_metadata_cannot_disagree_with_source_revision(self):
        changes = (("target", "other"), ("revision", "f" * 40), ("tag", "unversioned"))
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp) / "vendor"
            shutil.copytree(check.ROOT / "vendor/boost-context", directory)
            path = directory / "manifest.json"
            original = path.read_text()
            for field, value in changes:
                with self.subTest(field=field):
                    manifest = json.loads(original)
                    manifest[field] = value
                    path.write_text(json.dumps(manifest))
                    with self.assertRaises(RuntimeError):
                        check.check_vendor(directory)

    def test_asan_fiber_probe_requires_use_after_return_evidence(self):
        for code, error in ((1, ""), (-signal.SIGSEGV, ""),
                            (-signal.SIGABRT, "asan-context: read returned fiber local\n")):
            with self.subTest(code=code):
                result = subprocess.CompletedProcess(["binary"], code, "", error)
                with self.assertRaises(RuntimeError):
                    check.check_asan_lifetime(result)
        result = subprocess.CompletedProcess(["binary"], -signal.SIGABRT, "",
                                             "asan-context: read returned fiber local\n"
                                             "ERROR: AddressSanitizer: stack-use-after-return\n")
        check.check_asan_lifetime(result)

    def test_fatal_scheduler_cleanup_requires_the_expected_operation(self):
        result = subprocess.CompletedProcess(["scheduler"], -signal.SIGABRT, "",
                                             "release-trigger\npanic[P008]: panic during cleanup\n"
                                             "original: panic P006: body failed\ncleanup: task cleanup\n"
                                             "second: P006: release failed\n")
        check.check_fatal(result, True, "task cleanup")
        with self.assertRaises(RuntimeError):
            check.check_fatal(result, True)

    def test_unjoined_child_requires_a_precise_private_protocol_failure(self):
        for code, text in ((1, ""), (-signal.SIGSEGV, ""), (-signal.SIGABRT, "panic[P008]\n")):
            with self.subTest(code=code):
                result = subprocess.CompletedProcess(["scheduler"], code, "", text)
                with self.assertRaises(RuntimeError):
                    check.check_unjoined(result, "body")
        result = subprocess.CompletedProcess(["scheduler"], -signal.SIGABRT, "",
                                             "fatal runtime protocol: body returned with unjoined children\n")
        check.check_unjoined(result, "body")
        with self.assertRaises(RuntimeError):
            check.check_unjoined(result, "cleanup")

    def test_unclosed_scope_probe_is_distinct_from_unjoined_children(self):
        result = subprocess.CompletedProcess(["scheduler"], -signal.SIGABRT, "",
                                             "fatal runtime protocol: body returned with unclosed task scopes\n")
        check.check_unjoined(result, "body", "unclosed task scopes")
        with self.assertRaises(RuntimeError):
            check.check_unjoined(result, "body")


if __name__ == "__main__":
    unittest.main()
