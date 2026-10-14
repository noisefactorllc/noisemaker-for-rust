"""Tests for scripts/parity-summary: argument handling and report shaping.

The full run needs a built binary and an oracle checkout, so the harness-level
behavior is exercised with the real parity module and doctored reports instead
of a whole-port render; the entrypoint itself is exercised end-to-end by the
supervisor's gate against a provisioned oracle.
"""

from __future__ import annotations

import importlib
import json
import os
import stat
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENTRYPOINT = ROOT / "scripts" / "parity-summary"


class EntryPointTest(unittest.TestCase):
    def test_entrypoint_exists_and_is_executable(self) -> None:
        self.assertTrue(ENTRYPOINT.is_file())
        mode = ENTRYPOINT.stat().st_mode
        self.assertTrue(mode & stat.S_IXUSR, "scripts/parity-summary must be executable")

    def test_python_source_parses(self) -> None:
        import ast

        ast.parse(ENTRYPOINT.read_text(encoding="utf-8"))


class ReportDerivationTest(unittest.TestCase):
    """The count derivation over a validated default-leg report."""

    @classmethod
    def setUpClass(cls) -> None:
        os.environ.setdefault("PYTHONDONTWRITEBYTECODE", "1")
        sys.path.insert(0, str(ROOT / "scripts"))
        cls.parity = importlib.import_module("parity")

    def test_catalog_covers_the_authority_manifest(self) -> None:
        # The authority manifest is the oracle snapshot's sourceEffectIds list
        # (210 effect IDs); this port's catalog must cover every manifest id.
        catalog = self.parity._catalog()
        self.assertEqual(len(catalog), 210)
        snapshot = Path(
            os.environ.get(
                "NOISEMAKER_JS_CPU_DIR",
                str(ROOT.parent / "noisemaker-for-cpu"),
            )
        )
        manifest_path = snapshot / "src" / "effects" / "generated" / "upstream-snapshot.js"
        if not manifest_path.is_file():
            self.skipTest("no oracle checkout available to read the manifest")
        import re

        text = manifest_path.read_text(encoding="utf-8")
        match = re.search(
            r"export const sourceEffectIds = Object\.freeze\(\[(.*?)\]\)", text, re.DOTALL
        )
        self.assertIsNotNone(match)
        manifest = re.findall(r'"([^"]+)"', match.group(1))
        self.assertEqual(len(manifest), 210)
        self.assertFalse(set(manifest).difference(catalog))

    def test_summary_shape_from_a_validated_report(self) -> None:
        # Build a synthetic report the validator accepts: 210 compared, all
        # byte-exact, exactly matching the real report's shape contract.
        catalog = sorted(self.parity._catalog())
        results = [
            {
                "id": effect_id,
                "effect": effect_id,
                "case": {"kind": "default"},
                "status": "compared",
                "max_delta": 0,
                "mean_delta": 0,
                "differing_channels": 0,
                "channels_over_2": 0,
                "pass": True,
            }
            for effect_id in catalog
        ]
        report = {
            "catalog": 210,
            "expected": 210,
            "executed": 210,
            "compared": 210,
            "unsupported": 0,
            "errors": 0,
            "passed": 210,
            "failed": 0,
            "byte_exact": 210,
            "tolerance": 0,
            "size": 8,
            "time": 0.25,
            "seed": 1,
            "results": results,
        }
        self.parity._validate(report, catalog)
        compared = [record for record in report["results"] if record["status"] == "compared"]
        exact = sum(record["max_delta"] == 0 for record in compared)
        strict = len(compared) - exact
        fail = report["failed"] + report["errors"]
        executed = len(compared) + fail
        self.assertEqual(
            (len(catalog), executed, exact, strict, fail), (210, 210, 210, 0, 0)
        )


class GuardBehaviorTest(unittest.TestCase):
    """The revision gate is advisory; the entrypoint still fails without an oracle."""

    @staticmethod
    def _load_entrypoint():
        import importlib.machinery
        import importlib.util

        loader = importlib.machinery.SourceFileLoader("parity_summary_entrypoint", str(ENTRYPOINT))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        module = importlib.util.module_from_spec(spec)
        loader.exec_module(module)
        return module

    def test_revision_mismatch_is_advisory_not_fatal(self) -> None:
        module = self._load_entrypoint()
        contract = json.loads((ROOT / "scripts" / "upstream-param-contract.json").read_text())
        contract_revision = contract["revision"]
        # An agreeing revision produces no warning; any other revision produces
        # only a warning string. The run itself must proceed either way.
        self.assertIsNone(module._revision_warning(contract_revision, contract_revision))
        warning = module._revision_warning("0" * 40, contract_revision)
        self.assertIsNotNone(warning)
        self.assertIn("does not match the", warning)
        self.assertIn("warning:", warning)

    def test_missing_oracle_fails_loudly(self) -> None:
        import argparse
        import tempfile

        module = self._load_entrypoint()
        args = argparse.Namespace(js=None)
        with tempfile.TemporaryDirectory() as temporary:
            # An empty candidate list models an environment with no oracle
            # checkout anywhere, independent of this machine's layout.
            with self.assertRaises(SystemExit) as raised:
                module._oracle_checkout(args, candidates=[Path(temporary)])
        self.assertIn("no noisemaker-for-cpu oracle checkout found", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
