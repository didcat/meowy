import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify


class VerifyTests(unittest.TestCase):
    def test_default_plan_does_not_execute_compiler_or_editor(self):
        checks = verify.plan()
        self.assertEqual(4, len(checks))
        self.assertTrue(all(check.args[0] == sys.executable for check in checks))
        self.assertTrue(all("compiler/" not in arg for check in checks for arg in check.args))
        self.assertTrue(all("-E" in check.args for check in checks))

    def test_selected_tools_and_strict_catalog(self):
        checks = verify.plan("both", True, True)
        self.assertIn("Vim runtime", [check.name for check in checks])
        self.assertIn("Neovim runtime", [check.name for check in checks])
        self.assertEqual("--strict", checks[-1].args[-1])
        self.assertLess([check.name for check in checks].index("bootstrap compiler build"), [check.name for check in checks].index("compiler conformance execution"))

    def test_cargo_build_and_harness_share_explicit_target_location(self):
        with patch.dict(os.environ, {"CARGO_TARGET_DIR": "/elsewhere", "CARGO_BUILD_TARGET": "other-target"}):
            checks = verify.plan(compiler=True)
        build = next(check for check in checks if check.name == "bootstrap compiler build")
        target = build.args[build.args.index("--target") + 1]
        base = build.args[build.args.index("--target-dir") + 1]
        self.assertEqual(str(verify.ROOT / base / target / "debug/meowy"), checks[-1].args[-1])

    def test_runtime_checks_are_explicit_and_all_includes_them(self):
        defaults = verify.plan()
        selected = verify.plan(runtime=True)
        self.assertFalse(any("runtime/" in arg for check in defaults for arg in check.args))
        self.assertEqual(2, len(selected) - len(defaults))
        self.assertEqual("runtime/check.py", selected[-1].args[-1])
        result = subprocess.run([sys.executable, "-B", str(verify.ROOT / "tools/verify.py"), "--all", "--list"], capture_output=True, text=True)
        self.assertEqual(0, result.returncode)
        self.assertIn("runtime/check.py", result.stdout)
        self.assertIn("runtime/tests", result.stdout)

    def test_subprocess_uses_repository_directory(self):
        with tempfile.TemporaryDirectory() as temp, contextlib.redirect_stdout(io.StringIO()):
            root = Path(temp)
            check = verify.Check("working directory", (sys.executable, "-c", "from pathlib import Path; Path('result').write_text('done')"))
            self.assertTrue(verify.run(check, root=root))
            self.assertEqual("done", (root / "result").read_text())

    def test_failure_missing_tool_and_timeout_fail(self):
        checks = [
            verify.Check("failed process", (sys.executable, "-c", "raise SystemExit(7)")),
            verify.Check("missing process", ("/nonexistent/meowy-verification-tool",)),
            verify.Check("timeout", (sys.executable, "-c", "import time; time.sleep(10)")),
        ]
        with contextlib.redirect_stdout(io.StringIO()):
            for check in checks:
                with self.subTest(check=check.name):
                    self.assertFalse(verify.run(check, timeout=0.1))

    def test_optimized_environment_does_not_disable_validator_assertions(self):
        check = verify.Check("assertions", verify.plan()[0].args[:5] + ("-c", "import sys; sys.exit(0 if __debug__ else 1)"))
        with patch.dict(os.environ, {"PYTHONOPTIMIZE": "2"}), contextlib.redirect_stdout(io.StringIO()):
            self.assertTrue(verify.run(check))

    def test_list_is_read_only_and_strict_requires_compiler(self):
        for args, code in [(["--all", "--strict", "--list"], 0), (["--strict"], 2), (["--timeout", "nan"], 2)]:
            with self.subTest(args=args):
                result = subprocess.run([sys.executable, "-B", str(verify.ROOT / "tools/verify.py"), *args], capture_output=True, text=True, cwd="/tmp")
                self.assertEqual(code, result.returncode)
                if code == 0:
                    self.assertIn("Plan only; no checks were run.", result.stdout)
                    self.assertIn("--strict", result.stdout)

    def test_verification_stops_after_failure(self):
        output = io.StringIO()
        with patch.object(sys, "argv", ["verify.py"]), patch.object(verify, "run", return_value=False) as run, contextlib.redirect_stdout(output):
            self.assertEqual(1, verify.main())
            self.assertEqual(1, run.call_count)
        self.assertIn("NOT RUN", output.getvalue())
        self.assertIn("0 passed; 1 failed; 3 not run.", output.getvalue())

    def test_logs_keep_stdout_stderr_and_failed_exit_visible(self):
        with tempfile.TemporaryDirectory() as temp:
            log = Path(temp) / "check.log"
            check = verify.Check("logged failure", (sys.executable, "-c", "import sys; print('out'); print('err', file=sys.stderr); sys.exit(7)"))
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertFalse(verify.run(check, log=log))
            self.assertIn("FAIL logged failure: exit 7", output.getvalue())
            self.assertIn("\nout\n", log.read_text())
            self.assertIn("\nerr\n", log.read_text())
            self.assertIn("Command:", log.read_text())

    def test_complete_run_reports_failures_and_runs_later_checks(self):
        checks = [verify.Check("first", ("first",)), verify.Check("second", ("second",))]
        output = io.StringIO()
        with tempfile.TemporaryDirectory() as temp, patch.object(sys, "argv", ["verify.py", "--keep-going", "--log-dir", temp]), patch.object(verify, "plan", return_value=checks), patch.object(verify, "run", side_effect=[False, True]) as run, contextlib.redirect_stdout(output):
            self.assertEqual(1, verify.main())
            self.assertEqual(2, run.call_count)
            self.assertEqual(Path(temp) / "02-second.log", run.call_args.kwargs["log"])
        self.assertIn("1 passed; 1 failed; 0 not run.", output.getvalue())
        self.assertIn("01-first.log", output.getvalue())

    def test_logged_conformance_preserves_known_gap_counts(self):
        check = verify.Check("compiler conformance execution", (sys.executable, "-c", "print('495 passed; 19 unsupported; 0 failed (debug and release).')"))
        output = io.StringIO()
        with tempfile.TemporaryDirectory() as temp, patch.object(sys, "argv", ["verify.py", "--compiler", "--log-dir", temp]), patch.object(verify, "plan", return_value=[check]), contextlib.redirect_stdout(output):
            self.assertEqual(0, verify.main())
        self.assertIn("495 passed; 19 unsupported; 0 failed (debug and release).", output.getvalue())
        self.assertIn("does not qualify the complete v0.0.1 release", output.getvalue())


if __name__ == "__main__":
    unittest.main()
