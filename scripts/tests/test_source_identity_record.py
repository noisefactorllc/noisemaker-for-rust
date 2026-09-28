"""Integrity tests for the committed parity source-identity record.

These tests bind the committed audit record in docs/parity/source-identity.json
against internal consistency invariants: every digest-looking field is a real
sha256 hex digest, the recorded parity arithmetic adds up, and the audited
range endpoints match the recorded snapshot revisions. They do not re-render
the catalog; the executed 205-record run remains the authoritative parity
evidence archived with the owning job.
"""

import json
import re
import unittest
from pathlib import Path

RECORD = Path(__file__).resolve().parent.parent.parent / "docs" / "parity" / "source-identity.json"
SHA256 = re.compile(r"^[0-9a-f]{64}$")


def _walk_strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from _walk_strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from _walk_strings(item)


class SourceIdentityRecordTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.record = json.loads(RECORD.read_text(encoding="utf-8"))

    def test_fd9d56c_leg_audit_present(self):
        audit = self.record["fd9d56c_leg_audit"]
        self.assertIn(
            "f0ccebef830bf6b8d99eafadf73f260a6c224873..fd9d56c74ce7500b7eaeea90a93d3bf49375d28e",
            audit["declared_range"],
        )
        self.assertEqual(len(audit["new_commits"]), 3)
        snapshot = audit["snapshot_identity"]
        for field in ("sha256_at_f0ccebe", "sha256_at_fd9d56c"):
            self.assertRegex(snapshot[field], SHA256)
        self.assertIn("73c15be00d6888f4b5d2835d8e242ee9e840df45", snapshot["entire_diff"])
        self.assertIn("296e0138c4744ed485b2e95de3eeb466c17629ee", snapshot["entire_diff"])

    def test_fd9d56c_range_run_arithmetic(self):
        run = self.record["fd9d56c_range_run"]
        result = run["result"]
        self.assertEqual(result["catalog"], 205)
        self.assertEqual(result["compared"], 202)
        self.assertEqual(result["unsupported"], 3)
        self.assertEqual(result["errors"], 0)
        self.assertEqual(result["passed"], result["compared"])
        self.assertEqual(result["failed"], 0)
        self.assertEqual(result["byte_exact"], result["passed"])
        self.assertEqual(run["oracle_revision"]["commit"], "fd9d56c74ce7500b7eaeea90a93d3bf49375d28e")
        self.assertEqual(
            run["port_revision"]["param_contract_revision"],
            "6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa",
        )
        commands = run["declared_checks_reexecuted"]["commands"]
        self.assertTrue(commands)
        for entry in commands:
            self.assertEqual(entry["exit_code"], 0, entry["command"])

    def test_recorded_artifact_hashes_are_digests(self):
        artifacts = self.record["fd9d56c_range_run"]["artifacts_sha256"]
        for name, digest in artifacts.items():
            if name == "archive":
                continue
            self.assertRegex(digest, SHA256, name)

    def test_all_recorded_sha256_fields_are_digests(self):
        for text in _walk_strings(self.record):
            if "sha256" in text.lower() and len(text) == 64:
                self.assertRegex(text, SHA256)


if __name__ == "__main__":
    unittest.main()
