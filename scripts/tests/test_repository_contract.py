"""Checks that the README keeps its contract with the repository's live reports."""

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent.parent
REPO_URL = "https://github.com/noisefactorllc/noisemaker-for-rust"


class ReadmeTest(unittest.TestCase):
    def test_readme_links_the_live_compatibility_report_and_gap_issues(self):
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertTrue(f"{REPO_URL}/issues/7" in readme, "README must link the compatibility report issue")
        self.assertTrue(f"{REPO_URL}/issues?q=label%3Agap" in readme, "README must link the issues labelled gap")
        self.assertNotRegex(readme, r"docs/COMPATIBILITY\.md|docs/COMPLETION_GAPS\.md")


if __name__ == "__main__":
    unittest.main()
