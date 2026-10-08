"""Checks that the README and the export-kit workflow keep their contracts.

The workflow checks read the YAML text directly, because the CI quality job
runs this suite on a bare Python without PyYAML.
"""

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent.parent
REPO_URL = "https://github.com/noisefactorllc/noisemaker-for-rust"


def top_level_block(text, key):
    """Return the lines under a top-level YAML key, up to the next top-level key."""
    lines = text.splitlines()
    start = lines.index(f"{key}:")
    block = []
    for line in lines[start + 1 :]:
        if line and not line.startswith((" ", "#")):
            break
        block.append(line)
    return block


def nested_block(lines, key, indent):
    """Return the lines under `key:` at the given indent, up to its next sibling."""
    prefix = " " * indent
    start = lines.index(f"{prefix}{key}:")
    block = []
    for line in lines[start + 1 :]:
        stripped = line.lstrip(" ")
        if stripped and not stripped.startswith("#") and len(line) - len(stripped) <= indent:
            break
        block.append(line)
    return block


class ReadmeTest(unittest.TestCase):
    def test_readme_links_the_live_compatibility_report_and_gap_issues(self):
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertTrue(f"{REPO_URL}/issues/7" in readme, "README must link the compatibility report issue")
        self.assertTrue(f"{REPO_URL}/issues?q=label%3Agap" in readme, "README must link the issues labelled gap")
        self.assertNotRegex(readme, r"docs/COMPATIBILITY\.md|docs/COMPLETION_GAPS\.md")


class ExportKitWorkflowTest(unittest.TestCase):
    def setUp(self):
        self.text = (ROOT / ".github" / "workflows" / "export-kit.yml").read_text(encoding="utf-8")

    def test_a_src_push_to_main_runs_rendered_parity(self):
        on = top_level_block(self.text, "on")
        self.assertIn("  push:", on, "export-kit.yml must run on push")
        push = nested_block(on, "push", 2)
        self.assertTrue(any(re.fullmatch(r"\s*branches:\s*\[main\]\s*", line) for line in push))
        paths = [m.group(1) for line in push if (m := re.fullmatch(r"\s*-\s*[\"']?([^\"']+)[\"']?\s*", line))]
        self.assertIn("src/**", paths, "a src/** push must trigger the rendered parity")

    def test_rendered_parity_compares_every_case_against_the_pinned_oracle(self):
        jobs = top_level_block(self.text, "jobs")
        parity = nested_block(jobs, "rendered-parity", 2)
        self.assertFalse(
            any(re.match(r"\s{4}if:", line) for line in parity),
            "rendered-parity must not be skipped on any trigger",
        )
        body = "\n".join(parity)
        self.assertRegex(body, r"repository: noisefactorllc/noisemaker-for-cpu\n\s+ref: [0-9a-f]{40}\n")
        self.assertIn("scripts/parity.py", body)
        self.assertNotIn("--only", body, "the gate compares the whole catalog")

    def test_the_kit_dispatch_runs_only_after_parity_and_never_from_a_push(self):
        jobs = top_level_block(self.text, "jobs")
        dispatch = nested_block(jobs, "dispatch", 2)
        self.assertIn("    needs: rendered-parity", dispatch)
        condition = next(line for line in dispatch if re.match(r"\s{4}if:", line))
        self.assertIn("github.ref == 'refs/heads/main'", condition)
        self.assertIn("github.event_name != 'push'", condition)


if __name__ == "__main__":
    unittest.main()
