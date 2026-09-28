import tempfile
import unittest
from pathlib import Path

from conformance_fixtures import case_files


class FixtureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.bundle = self.base / "sources/case"
        (self.bundle / "sub").mkdir(parents=True)
        (self.bundle / "main.mwy").write_text('m:@"./sub/helper.mwy"\n')
        (self.bundle / "sub/helper.mwy").write_bytes(b"#\xc3\xa9#\r\n->n:2\r\n")
        (self.bundle / "empty.mwy").write_text("")
        self.case = {"source": "sources/case/main.mwy", "companions": ["sources/case/sub/helper.mwy", "sources/case/empty.mwy"]}

    def test_declared_files_keep_relative_layout_and_empty_modules(self):
        files = case_files(self.case, self.base)
        self.assertEqual([str(target) for _, target in files], ["main.mwy", "sub/helper.mwy", "empty.mwy"])
        self.assertEqual(files[1][0].read_bytes(), b"#\xc3\xa9#\r\n->n:2\r\n")
        self.assertEqual(len(case_files({"source": self.case["source"]}, self.base)), 1)

    def test_noncanonical_escaping_missing_and_wrong_kind_sources_fail(self):
        for name in ["/tmp/main.mwy", "../main.mwy", "sources/case/../case/main.mwy", "sources//case/main.mwy", "sources/./case/main.mwy", "sources\\case\\main.mwy", "sources/case/sub", "sources/case/missing.mwy", 7]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                case_files({"source": name}, self.base)

    def test_duplicate_entry_and_duplicate_companion_declarations_fail(self):
        for companions in [[self.case["source"]], [self.case["companions"][0]] * 2, [], "helper.mwy", [7]]:
            with self.subTest(companions=companions), self.assertRaises(ValueError):
                case_files({"source": self.case["source"], "companions": companions}, self.base)

    def test_companions_cannot_leave_the_entry_bundle(self):
        (self.base / "sources/other.mwy").write_text("->n:3")
        with self.assertRaisesRegex(ValueError, "outside entry"):
            case_files({"source": self.case["source"], "companions": ["sources/other.mwy"]}, self.base)

    def test_symlinked_files_and_directories_are_rejected(self):
        (self.bundle / "alias.mwy").symlink_to(self.bundle / "sub/helper.mwy")
        (self.bundle / "linked").symlink_to(self.bundle / "sub", target_is_directory=True)
        for name in ["sources/case/alias.mwy", "sources/case/linked/helper.mwy"]:
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "symlinked"):
                case_files({"source": name}, self.base)


if __name__ == "__main__":
    unittest.main()
