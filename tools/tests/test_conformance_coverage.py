import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("conformance_coverage", ROOT / "docs/conformance/coverage.py")
coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage)


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.catalog = json.loads((ROOT / "docs/conformance/cases.json").read_text())
        self.data = json.loads((ROOT / "docs/conformance/documents.json").read_text())

    def test_inventory_maps_every_current_reference_document(self):
        coverage.validate_documents(ROOT, self.catalog, self.data)
        gaps = coverage.load_support(self.catalog)
        report = coverage.render(self.catalog, self.data, gaps)
        self.assertIn("not line/branch coverage", report)
        for case in self.catalog["cases"]:
            self.assertIn(f"[{case['id']}]", report)
        self.assertIn("Blocked:", report)

    def test_missing_document_changed_contract_and_missing_gap_fail(self):
        for change in [lambda data: data["documents"].pop("syntax.md"),
                       lambda data: data["documents"]["syntax.md"].update(sha256="stale"),
                       lambda data: data["documents"]["syntax.md"].update(gap="")]:
            data = copy.deepcopy(self.data)
            change(data)
            with self.assertRaises(AssertionError):
                coverage.validate_documents(ROOT, self.catalog, data)

    def test_evidence_requires_known_kind_existing_local_test_file(self):
        for evidence in ["native:missing.rs", "native:../outside.rs", "unknown:STATUS.md", "native:STATUS.md"]:
            with self.subTest(evidence=evidence), self.assertRaises(AssertionError):
                data = copy.deepcopy(self.data)
                data["documents"]["syntax.md"]["evidence"] = [evidence]
                coverage.validate_documents(ROOT, self.catalog, data)

    def test_stale_case_reference_or_anchor_fails(self):
        for reference in ["../reference/missing.md", "../reference/syntax.md#missing", "../README.md"]:
            with self.subTest(reference=reference), self.assertRaises(AssertionError):
                catalog = copy.deepcopy(self.catalog)
                catalog["cases"][0]["reference"] = reference
                coverage.validate_documents(ROOT, catalog, self.data)

    def test_proof_matrix_rejects_missing_changed_and_unlinked_obligations(self):
        proof = json.loads((ROOT / "docs/conformance/proof-obligations.json").read_text())
        coverage.validate_proof(ROOT, self.catalog, proof)
        for change in [lambda data: data["obligations"].pop(),
                       lambda data: data["obligations"][0].update(required="weaker result"),
                       lambda data: data["obligations"][0].update(cases=["missing_case"]),
                       lambda data: data["obligations"][0].update(gap="")]:
            data = copy.deepcopy(proof)
            change(data)
            with self.assertRaises(AssertionError):
                coverage.validate_proof(ROOT, self.catalog, data)

    def test_report_must_be_regenerated_after_reviewed_inputs_change(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            base = root / "docs/conformance"
            base.mkdir(parents=True)
            refs = root / "docs/reference"
            refs.mkdir()
            (refs / "rule.md").write_text("# Rule\n")
            (refs / "stdlib").mkdir()
            text = "## Qualification obligations\n\nQualification must also preserve runtime behavior.\n\nRelease support requires more evidence.\n"
            (refs / "stdlib/proof.md").write_text(text)
            data = {"version": 1, "documents": {"rule.md": {
                "sha256": coverage.hashlib.sha256((refs / "rule.md").read_bytes()).hexdigest(),
                "evidence": [], "gap": "Unqualified"}}}
            data["documents"]["stdlib/proof.md"] = {"sha256": coverage.hashlib.sha256(text.encode()).hexdigest(), "evidence": [], "gap": "Unqualified"}
            proof = {"version": 1, "obligations": [{"rule": rule, "required": required, "cases": [], "gap": "Unqualified"} for rule, required in coverage.proof_rules(text)]}
            (base / "proof-obligations.json").write_text(json.dumps(proof))
            (base / "documents.json").write_text(json.dumps(data))
            (base / "cases.json").write_text(json.dumps({"cases": []}))
            support = root / "compiler/tests"
            support.mkdir(parents=True)
            (support / "conformance_support.json").write_text('{"version":1,"unsupported":{}}')
            coverage.check(root, write=True)
            coverage.check(root)
            (base / "COVERAGE.md").write_text("stale")
            with self.assertRaisesRegex(AssertionError, "stale coverage"):
                coverage.check(root)


if __name__ == "__main__":
    unittest.main()
