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



def trimmed(lines):
    """The lines without trailing blank or comment-only lines."""
    lines = list(lines)
    while lines and (not lines[-1].strip() or lines[-1].lstrip().startswith("#")):
        lines.pop()
    return lines

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


# The reviewed weekly sweep job in .github/workflows/ci.yml.
SWEEP_JOB = """    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    timeout-minutes: 90
    steps:
      - name: Harden runner
        uses: step-security/harden-runner@b09bb98e06d4d774595224525879c09bc6e98c40 # v2.20.1
        with:
          egress-policy: audit

      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false

      - name: Check out pinned JavaScript CPU oracle
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          repository: noisefactorllc/noisemaker-for-cpu
          ref: 0670ceace1c0c52791ef7d57493ded6e0b35a4b9
          path: oracle
          persist-credentials: false

      - uses: actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 # v7.0.0
        with:
          python-version: "3.14"

      - uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7.0.0
        with:
          node-version: "22"

      - name: Install stable Rust
        run: rustup toolchain install stable --profile minimal

      - name: Build the CLI and the sequence example
        run: cargo +stable build --locked --release --bin noisemaker-rs --example parity_sequence

      - name: Sweep non-default parameters and stateful sequences against the pinned oracle
        run: python3 scripts/parity.py --rust "$GITHUB_WORKSPACE/target/release/noisemaker-rs" --rust-sequence "$GITHUB_WORKSPACE/target/release/examples/parity_sequence" --js "$GITHUB_WORKSPACE/oracle/bin/noisemaker-cpu.js" --timeout 120 --sweep --json sweep-report.json

      - name: Retain sweep report
        if: always()
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v4.6.2
        with:
          name: rust-sweep-${{ github.sha }}
          path: sweep-report.json
          if-no-files-found: error"""

# Everything in ci.yml before `jobs:`: its name, triggers, permissions and
# concurrency.
PREAMBLE = "\n".join([
    'name: ci',
    '',
    'on:',
    '  push:',
    '    paths:',
    "      - '**'",
    "      - '!*.md'",
    "      - '!.github/ISSUE_TEMPLATE/**'",
    "      - '!.github/PULL_REQUEST_TEMPLATE*'",
    "      - '!.github/FUNDING.yml'",
    "      - '!.github/CODEOWNERS'",
    "      - '!.editorconfig'",
    "      - '!docs/**'",
    "      - 'docs/EFFECTS.md'",
    '    branches: [main]',
    '  workflow_dispatch:',
    '  schedule:',
    "    - cron: '23 8 * * 1'",
    '',
    'permissions:',
    '  contents: read',
    '',
    '# A push runs the quality checks and the test suite. Everything that renders',
    '# the catalog on the CPU (the catalog smoke tests, the cross-port spine and the',
    '# Windows rendered parity) runs weekly and on manual dispatch.',
    'concurrency:',
    "  group: ci-${{ github.ref }}-${{ github.event_name == 'push' && 'gate' || 'full' }}",
    "  cancel-in-progress: ${{ github.event_name == 'push' }}",
    '',
])


class WeeklySweepWorkflowTest(unittest.TestCase):
    """The bounded parameter sweep and stateful sequences gate the weekly CI."""

    def setUp(self):
        self.ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        self.sweep = "\n".join(nested_block(top_level_block(self.ci, "jobs"), "sweep", 2))

    def test_the_sweep_runs_only_on_schedule_or_dispatch_and_never_soft_fails(self):
        conditions = [line.strip() for line in self.sweep.splitlines() if line.startswith("    if:")]
        self.assertEqual(
            conditions,
            ["if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'"],
            "the sweep runs only on the weekly schedule and on manual dispatch",
        )
        self.assertNotIn("continue-on-error", self.sweep)

    def test_the_sweep_compares_against_the_same_pinned_oracle_as_the_parity_gate(self):
        pin = r"repository: noisefactorllc/noisemaker-for-cpu\n\s+ref: ([0-9a-f]{40})\n"
        export_kit = (ROOT / ".github" / "workflows" / "export-kit.yml").read_text(encoding="utf-8")
        gate = "\n".join(nested_block(top_level_block(export_kit, "jobs"), "rendered-parity", 2))
        self.assertEqual(re.search(pin, self.sweep).group(1), re.search(pin, gate).group(1))

    def test_the_sweep_job_is_exactly_the_reviewed_job(self):
        # The whole job is pinned, and so is everything before `jobs:`, and no
        # top-level key may follow `jobs:`: any change to the job or to a
        # workflow-level key that reaches it needs a reviewed change here.
        self.assertEqual("\n".join(trimmed(nested_block(top_level_block(self.ci, "jobs"), "sweep", 2))), SWEEP_JOB)

    def test_the_workflow_runs_under_the_reviewed_preamble(self):
        self.assertIn("\njobs:\n", self.ci)
        self.assertEqual(self.ci.split("\njobs:\n", 1)[0], PREAMBLE)

    def test_jobs_is_the_last_top_level_key_and_every_job_id_is_plain_and_unique(self):
        lines = self.ci.splitlines()
        after = lines[lines.index("jobs:") + 1:]
        self.assertEqual([line for line in after if line and not line.startswith((" ", "#"))], [],
                         "no top-level key follows jobs:")
        ids = [line for line in after if re.match(r"^ {2}\S", line) and not line.lstrip().startswith("#")]
        for line in ids:
            self.assertRegex(line, r"^  [a-z][a-z0-9_-]*:$")
        names = [line.strip()[:-1].lower() for line in ids]
        self.assertEqual(len(names), len(set(names)), "job ids are unique")
        self.assertIn("  sweep:", ids)

    def test_ci_runs_every_contract_test(self):
        self.assertEqual(self.ci.count("discover -s scripts/tests"), 1)
        self.assertIn("\n        run: python -m unittest discover -s scripts/tests -p 'test_*.py'\n", self.ci)

    def test_the_sweep_runs_every_case_with_the_sequence_example(self):
        self.assertIn("--example parity_sequence", self.sweep)
        command = next(line for line in self.sweep.splitlines() if "scripts/parity.py" in line)
        for flag in ("--sweep", "--rust-sequence", "--json"):
            self.assertIn(flag, command)
        self.assertNotIn("--only", command, "the sweep covers the whole catalog")


if __name__ == "__main__":
    unittest.main()
