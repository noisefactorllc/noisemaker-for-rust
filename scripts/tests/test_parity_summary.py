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
import subprocess
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
    """The count derivation over a validated sweep report."""

    @classmethod
    def setUpClass(cls) -> None:
        os.environ.setdefault("PYTHONDONTWRITEBYTECODE", "1")
        sys.path.insert(0, str(ROOT / "scripts"))
        cls.parity = importlib.import_module("parity")

    def test_sweep_plan_denominator_is_697(self) -> None:
        plan = self.parity._sweep_plan(self.parity._catalog())
        self.assertEqual(sum(map(len, plan.values())), 697)

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
            "sweep": {
                "variants": 2,
                "frames": 3,
                "times": [0.1, 0.4, 0.7],
                "stateful_effects": 32,
                "feedback": "previous-output-surface",
            },
            "results": results,
        }
        plan = self.parity._sweep_plan(self.parity._catalog())
        # Restrict the plan to defaults only: a synthetic default-only report
        # validates against the default-only sub-plan.
        defaults_plan = {
            effect_id: [case for case in cases if case["kind"] == "default"]
            for effect_id, cases in plan.items()
        }
        self.parity._validate_sweep(report, defaults_plan)
        compared = [record for record in report["results"] if record["status"] == "compared"]
        exact = sum(record["max_delta"] == 0 for record in compared)
        strict = len(compared) - exact
        self.assertEqual((report["expected"], len(compared), exact, strict), (210, 210, 210, 0))


class GuardBehaviorTest(unittest.TestCase):
    """The entrypoint fails loudly on a mismatched oracle revision."""

    def test_revision_mismatch_fails(self) -> None:
        import shutil
        import tempfile

        source = ROOT.parent / "noisemaker-for-cpu"
        snapshot = source / "src" / "effects" / "generated" / "upstream-snapshot.js"
        cli = source / "bin" / "noisemaker-cpu.js"
        if not (snapshot.is_file() and cli.is_file()):
            self.skipTest("no oracle checkout available to copy")
        with tempfile.TemporaryDirectory() as temporary:
            checkout = Path(temporary) / "noisemaker-cpu-doctored"
            shutil.copytree(source, checkout, symlinks=True)
            doctored = checkout / "src" / "effects" / "generated" / "upstream-snapshot.js"
            text = doctored.read_text(encoding="utf-8")
            contract = json.loads((ROOT / "scripts" / "upstream-param-contract.json").read_text())
            revision = contract["revision"]
            self.assertIn(revision, text, "test assumes the provisioned oracle matches the contract")
            doctored.write_text(text.replace(revision, "0" * 40, 1), encoding="utf-8")
            environment = dict(os.environ)
            environment["NOISEMAKER_JS_CPU_DIR"] = str(checkout)
            environment["PYTHONDONTWRITEBYTECODE"] = "1"
            completed = subprocess.run(
                [str(ENTRYPOINT)], capture_output=True, text=True, env=environment, timeout=120
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("does not match the", completed.stderr + completed.stdout)


if __name__ == "__main__":
    unittest.main()
