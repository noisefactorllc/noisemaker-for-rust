"""Tests for scripts/parity-summary: argument handling and report shaping.

The full run needs a built binary and an oracle checkout, so the harness-level
behavior is exercised with the real parity module and doctored reports instead
of a whole-port render; the entrypoint itself is exercised end-to-end by the
automated review's gate against a provisioned oracle.
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
    """The revision gate fails closed; the entrypoint still fails without an oracle."""

    @staticmethod
    def _load_entrypoint():
        import importlib.machinery
        import importlib.util

        loader = importlib.machinery.SourceFileLoader("parity_summary_entrypoint", str(ENTRYPOINT))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        module = importlib.util.module_from_spec(spec)
        loader.exec_module(module)
        return module

    def test_revision_mismatch_fails_closed(self) -> None:
        import argparse
        import tempfile
        from pathlib import Path

        module = self._load_entrypoint()
        contract_revision = json.loads(
            (ROOT / "scripts" / "upstream-param-contract.json").read_text()
        )["revision"]
        # An agreeing revision produces no warning and the run proceeds.
        self.assertIsNone(module._revision_warning(contract_revision, contract_revision))
        with tempfile.TemporaryDirectory() as temp:
            checkout = Path(temp)
            (checkout / "bin").mkdir()
            (checkout / "src" / "effects" / "generated").mkdir(parents=True)
            (checkout / "bin" / "noisemaker-cpu.js").write_text("export {};\n")
            (checkout / "src" / "effects" / "generated" / "upstream-snapshot.js").write_text(
                "export const UPSTREAM_REVISION = \"" + "0" * 40 + "\"\n"
                "export const sourceEffectIds = Object.freeze([\"synth/noise\"])\n"
            )
            args = ["--js", str(checkout / "bin" / "noisemaker-cpu.js")]
            # The stale oracle must abort the gate before any rendering: a
            # passing summary bound to the wrong revision is worse than no run.
            with self.assertRaises(SystemExit) as raised:
                module.main(args)
            self.assertIn("does not match the published", str(raised.exception))

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


class OracleResolutionTest(unittest.TestCase):
    """A checkout head that moved past the contract revision resolves at it."""

    @staticmethod
    def _load_entrypoint():
        import importlib.machinery
        import importlib.util

        loader = importlib.machinery.SourceFileLoader("parity_summary_resolution", str(ENTRYPOINT))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        module = importlib.util.module_from_spec(spec)
        loader.exec_module(module)
        return module

    @staticmethod
    def _commit_checkout(root: Path, revision: str, message: str) -> None:
        import subprocess

        subprocess.run(["git", "init", "-q"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
        (root / "bin").mkdir(exist_ok=True)
        generated = root / "src" / "effects" / "generated"
        generated.mkdir(parents=True, exist_ok=True)
        (root / "bin" / "noisemaker-cpu.js").write_text("export {};\n")
        (generated / "upstream-snapshot.js").write_text(
            f'export const UPSTREAM_REVISION = "{revision}"\n'
            'export const sourceEffectIds = Object.freeze(["synth/noise"])\n'
        )
        subprocess.run(["git", "add", "-A"], cwd=root, check=True)
        subprocess.run(["git", "commit", "-q", "-m", message], cwd=root, check=True)

    def test_contract_commit_finds_the_declaring_revision(self) -> None:
        import subprocess
        import tempfile

        module = self._load_entrypoint()
        with tempfile.TemporaryDirectory() as temp:
            checkout = Path(temp) / "checkout"
            checkout.mkdir()
            self._commit_checkout(checkout, "a" * 40, "older")
            self._commit_checkout(checkout, "b" * 40, "contract")
            self._commit_checkout(checkout, "c" * 40, "head")
            found = module._contract_commit(checkout, "b" * 40)
            self.assertIsNotNone(found)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"], cwd=checkout, check=True,
                stdout=subprocess.PIPE,
            ).stdout.decode().strip()
            self.assertNotEqual(found, head)
            shown = subprocess.run(
                ["git", "show", f"{found}:src/effects/generated/upstream-snapshot.js"],
                cwd=checkout, check=True, stdout=subprocess.PIPE,
            ).stdout.decode()
            self.assertIn(f'export const UPSTREAM_REVISION = "{"b" * 40}"', shown)

    def test_contract_commit_returns_none_without_the_revision(self) -> None:
        import tempfile

        module = self._load_entrypoint()
        with tempfile.TemporaryDirectory() as temp:
            checkout = Path(temp) / "checkout"
            checkout.mkdir()
            self._commit_checkout(checkout, "d" * 40, "only")
            self.assertIsNone(module._contract_commit(checkout, "b" * 40))

    def test_extract_oracle_yields_a_usable_directory(self) -> None:
        import subprocess
        import tempfile

        module = self._load_entrypoint()
        with tempfile.TemporaryDirectory() as temp:
            checkout = Path(temp) / "checkout"
            checkout.mkdir()
            self._commit_checkout(checkout, "a" * 40, "older")
            self._commit_checkout(checkout, "b" * 40, "contract")
            self._commit_checkout(checkout, "c" * 40, "head")
            commit = module._contract_commit(checkout, "b" * 40)
            self.assertIsNotNone(commit)
            module._ORACLE_EXTRACT_ROOT = Path(temp) / "extract"
            resolved = module._extract_oracle(checkout, commit)
            self.assertEqual(resolved, Path(temp) / "extract" / commit[:12])
            self.assertEqual(module._oracle_revision(resolved), "b" * 40)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"], cwd=checkout, check=True,
                stdout=subprocess.PIPE,
            ).stdout.decode().strip()
            # A second call reuses the extraction without re-archiving.
            self.assertEqual(module._extract_oracle(checkout, commit), resolved)
            self.assertEqual(module._oracle_revision(resolved), "b" * 40)
            self.assertNotEqual(resolved, head)

    def test_checkout_resolution_prefers_the_contract_revision(self) -> None:
        import argparse
        import subprocess
        import tempfile

        module = self._load_entrypoint()
        with tempfile.TemporaryDirectory() as temp:
            checkout = Path(temp) / "checkout"
            checkout.mkdir()
            self._commit_checkout(checkout, "a" * 40, "older")
            self._commit_checkout(checkout, "b" * 40, "contract")
            self._commit_checkout(checkout, "c" * 40, "head")
            contract = json.loads(
                (ROOT / "scripts" / "upstream-param-contract.json").read_text()
            )["revision"]
            self._commit_checkout(checkout, contract, "published contract")
            module._ORACLE_EXTRACT_ROOT = Path(temp) / "extract"
            resolved = module._oracle_checkout(
                argparse.Namespace(js=Path(checkout / "bin" / "noisemaker-cpu.js"))
            )
            self.assertEqual(module._oracle_revision(resolved), contract)
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"], cwd=checkout, check=True,
                stdout=subprocess.PIPE,
            ).stdout.decode().strip()
            self.assertNotEqual(resolved, head)


if __name__ == "__main__":
    unittest.main()
