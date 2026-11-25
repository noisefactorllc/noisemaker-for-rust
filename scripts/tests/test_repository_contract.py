"""Checks that the README and the export-kit workflow keep their contracts.

The workflow checks read the YAML text directly, because the CI quality job
runs this suite on a bare Python without PyYAML.
"""

from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parent.parent.parent
REPO_URL = "https://github.com/noisefactorllc/noisemaker-for-rust"
# Every checkout of the JavaScript CPU oracle reads its revision from this
# tracked file, through the step that reads it into steps.oracle.
ORACLE_REVISION_FILE = "scripts/cpu-oracle-revision"
ORACLE_PIN = r"repository: noisefactorllc/noisemaker-for-cpu\n\s+ref: \$\{\{ steps\.oracle\.outputs\.revision \}\}\n"
ORACLE_READ = r"id: oracle\n(?:.*\n)*?.*< scripts/cpu-oracle-revision\)\n"


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




def steps(lines):
    """Each `      - ` step of a job, as the text of its lines."""
    found = []
    for line in lines:
        if line.startswith("      - "):
            found.append([line])
        elif found and (line.startswith("        ") or not line.strip()):
            found[-1].append(line)
    return ["\n".join(step) for step in found]

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
        branches = [m.group(1) for line in push if (m := re.fullmatch(r"\s*branches:\s*\[([^\]]*)\]\s*", line))]
        self.assertEqual(len(branches), 1, "export-kit.yml push must list its branches inline")
        self.assertIn("main", [branch.strip() for branch in branches[0].split(",")])
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
        self.assertRegex(body, ORACLE_PIN)
        self.assertRegex(body, ORACLE_READ)
        self.assertIn("scripts/parity.py", body)
        self.assertNotIn("--only", body, "the gate compares the whole catalog")

    def test_the_kit_dispatch_runs_only_after_parity_and_never_from_a_push(self):
        jobs = top_level_block(self.text, "jobs")
        dispatch = nested_block(jobs, "dispatch", 2)
        self.assertIn("    needs: rendered-parity", dispatch)
        condition = next(line for line in dispatch if re.match(r"\s{4}if:", line))
        self.assertIn("github.ref == 'refs/heads/main'", condition)
        self.assertIn("github.event_name != 'push'", condition)


class OracleRevisionTest(unittest.TestCase):
    """Every oracle checkout reads one tracked revision, so moving the oracle edits no workflow."""

    def test_the_tracked_file_holds_one_full_commit_sha(self):
        text = (ROOT / ORACLE_REVISION_FILE).read_text(encoding="utf-8")
        self.assertRegex(text, r"\A[0-9a-f]{40}\n\Z")

    def test_every_oracle_checkout_reads_the_tracked_file(self):
        expected = {
            "ci.yml": {"parity-windows", "catalog-smoke", "sweep"},
            "export-kit.yml": {"rendered-parity"},
        }
        for name, oracle_jobs in expected.items():
            with self.subTest(workflow=name):
                text = (ROOT / ".github" / "workflows" / name).read_text(encoding="utf-8")
                block = top_level_block(text, "jobs")
                names = [m.group(1) for line in block if (m := re.fullmatch(r"  ([\w-]+):", line))]
                found = set()
                for job in names:
                    body = "\n".join(nested_block(block, job, 2))
                    checkouts = body.count("repository: noisefactorllc/noisemaker-for-cpu")
                    if not checkouts:
                        continue
                    found.add(job)
                    self.assertEqual(len(re.findall(ORACLE_PIN, body)), checkouts, f"{name} job {job}")
                    self.assertRegex(body, ORACLE_READ, f"{name} job {job} does not read {ORACLE_REVISION_FILE}")
                self.assertEqual(found, oracle_jobs)


class WeeklySweepWorkflowTest(unittest.TestCase):
    """The bounded parameter sweep and stateful sequences gate the weekly CI.

    The test guards against accidental regressions: a deleted job, a widened
    trigger, a soft fail or a dropped argument. It does not pin the job's
    text, so a reviewed edit such as an action pin bump still passes.
    """

    def setUp(self):
        self.ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        jobs = top_level_block(self.ci, "jobs")
        self.assertIn("  sweep:", jobs, "ci.yml has no sweep job")
        self.job = nested_block(jobs, "sweep", 2)
        self.sweep = "\n".join(self.job)
        self.gate = [step for step in steps(self.job) if "scripts/parity.py" in step]

    def test_the_sweep_runs_only_on_schedule_or_dispatch(self):
        conditions = [line.strip() for line in self.job if line.startswith("    if:")]
        self.assertEqual(
            conditions,
            ["if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'"],
            "the sweep runs only on the weekly schedule and on manual dispatch",
        )

    def test_nothing_in_the_sweep_soft_fails(self):
        self.assertNotIn("continue-on-error", self.sweep, "neither the job nor a step may soft-fail")
        self.assertEqual(len(self.gate), 1, "one step runs scripts/parity.py")
        self.assertNotRegex(self.gate[0], r"\|\|\s*true", "the sweep command must not ignore its exit status")

    def test_the_sweep_compares_against_the_same_pinned_oracle_as_the_parity_gate(self):
        export_kit = (ROOT / ".github" / "workflows" / "export-kit.yml").read_text(encoding="utf-8")
        gate = "\n".join(nested_block(top_level_block(export_kit, "jobs"), "rendered-parity", 2))
        for body in (self.sweep, gate):
            self.assertRegex(body, ORACLE_PIN)
            self.assertRegex(body, ORACLE_READ)

    def test_the_sweep_runs_every_case_with_the_sequence_example(self):
        self.assertIn("--example parity_sequence", self.sweep)
        for flag in ("--sweep", "--rust-sequence", "--json"):
            self.assertIn(flag, self.gate[0])
        self.assertNotIn("--only", self.gate[0], "the sweep covers the whole catalog")


if __name__ == "__main__":
    unittest.main()
