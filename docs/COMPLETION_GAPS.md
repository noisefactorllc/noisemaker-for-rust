# noisemaker-for-rust: completion gaps

Current compatibility matrix: [compatibility report](COMPATIBILITY.md).

## 1. Scope and source revisions

Daily review: 2026-09-27. Current inspected source: [`347e298348fbf61993f537f2a7689e798674d7e4`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/347e298348fbf61993f537f2a7689e798674d7e4).
Full rendered parity at this SHA: measured 2026-09-27 (section 3; GAP-001). Installed developer workflow and distribution qualification: measured 2026-09-26 on Linux x86_64 (GAP-002, GAP-003). Both are carried across the docs-only revisions after `7f55619` because the runtime tree is unchanged. The macOS and Windows platform matrix remains unmeasured. No release approval follows from this review.
Current upstream discovery: `e73a44a37f0c99bd3779c5fb26c7bba65a46a379`. Published Noisemaker authority: `1.0.192`, source `e73a44a37f0c99bd3779c5fb26c7bba65a46a379`, 210 effect IDs. The published manifest is byte-identical to the recorded 1.0.176 manifest, so the effect catalog is unchanged.
The observations below retain their original source and authority identities. They do not qualify later updates.
Current served kit: `0.1.21`, source `b6c24628f6b8082f07a8a0a1a0110661325d10ef`. Byte-verified against the repository on 2026-09-27 (section 3). Artifact identity does not establish host qualification.

### Earlier source observations

Date: 2026-09-24. Reviewed SHA: [`2ef1cc4179f5163023c26e785f533d07cf699fb4`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/2ef1cc4179f5163023c26e785f533d07cf699fb4).
Local HEAD matched remote main before checks. The operator requested registers for all remaining eligible ports in this run.
This initial register contains bounded evidence. It is not a completed port audit or release approval.
No implementation or parity checkpoint changed. Full audits remain in the rotation.

Rust CPU renderer with 205 effects. Cargo package noisemaker-for-rust exports library noisemaker_cpu and command noisemaker-rs. Rust 1.85 is required. [Contract](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md).

Generated catalog provenance records parameter-contract revision `44bc4ed4ac729bddaa95b083d64bee942ade35da`. Broader generated-source identity needs reconciliation with current authority.
Current upstream at discovery: `c9ee8a049b2b63cd300da67c01ee40baf29dc288`.
Current CPU authority: `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e`.
These authority heads are review targets, not qualification results. No goldens were regenerated.

Served kit `0.1.13` identifies `2ef1cc4179f5163023c26e785f533d07cf699fb4`. [Metadata](https://kits.noisedeck.app/rust/0/deployment-meta.json). Inventory and compatibility metadata were retrieved. Complete artifact bytes were not checked.

These document paths do not match the current publication workflow filters.
The containing commit identifies this register's publication revision. The shared run record retains commits, remote hashes, and downstream results.

## 2. Completion claims

| Claim ID | Claim source | Claimed scope | Finding | Evidence |
|---|---|---|---|---|
| CLAIM-001 | [Historical source](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Exact RGBA8 parity across the catalog with explicit unsupported interfaces. Runtime rendering does not require JavaScript or a GPU. | partial | The generated-bundle check exited 0. This is a reproducibility check, not a render or Cargo package installation test. |
| CLAIM-002 | [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Human usability: installation, output, errors, and recovery | supported | The complete installed workflow was exercised on 2026-09-26 with retained evidence ([parity/installed-workflow-20260926/](parity/installed-workflow-20260926/)). GAP-002. |
| CLAIM-003 | [Ecosystem reference](https://doc.rust-lang.org/cargo/reference/publishing.html) | Ecosystem fit and version support | supported | The packaged crate was installed from an isolated copy into a private root on Rust 1.85.0 and stable 1.98.1, both of which produced working renders and clean uninstalls. |
| CLAIM-004 | [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Release readiness | unverified | Metadata and CI do not replace installation of the actual artifact. GAP-003. |
| CLAIM-005 | [Exact-source Actions](https://github.com/noisefactorllc/noisemaker-for-rust/actions?query=head_sha%3A2ef1cc4179f5163023c26e785f533d07cf699fb4) | Workflow status only | supported | [Export kit](https://github.com/noisefactorllc/noisemaker-for-rust/actions/runs/35829708764): `success`. [ci](https://github.com/noisefactorllc/noisemaker-for-rust/actions/runs/35829708776): `success`. |

## 3. Methods and evidence

Review CI boundary: Exact-source runs: Export kit, ci. A passing export dispatch does not qualify rendered parity. Current complete-render enforcement remains an open verification requirement. [Exact-source responses and workflows](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/noisemaker-for-rust-remote-evidence.json).

### Daily review, 2026-09-27

Newest noisemaker-for-cpu delivery: `7a824744cb563f2280811f04e5a49f6792ed319d`, upstream pin `7443f6e6180300a45c5b97608459e5094504659d`. The pin range `6a0af04d..7443f6e6` changes upstream shader-language sources (`shaders/src/lang/transform.js` +377 lines, GAP-008 replaceEffect prediction) and tests. The effect catalog is unchanged. The CPU snapshot changes only its revision line over `901bbd9..7a824744`.
Environment: Linux x86_64 (AMD EPYC 7713, Linux 6.8.0-134-generic), cargo 1.98.1 (rustup, user-level), node v26.5.1, Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <fresh clone of noisemaker-for-cpu at 7a824744>/bin/noisemaker-cpu.js --timeout 120 --json <report>   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; 19 tests, OK
```

Parity result at size 8, time 0.25, seed 1, tolerance 0: 205 catalog records, 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason, 0 errors. `filter3d/flow3d` compared byte-exact in-run at 72.072 s, inside the 120 s timeout. Report sha256 `4e99dd64d154b557eb5439c27854a7f8b07dedc402ca894b9ca9da244ed8d94a`; run log sha256 `6f0d746e9c6c6cda2d1c60dccc4795300f0679ce4ea9b38f4bee3099681da786`. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.
Exact-source CI: the newest `ci` and `Export kit` runs are green at head `b6c24628f6b8082f07a8a0a1a0110661325d10ef` (2026-09-26T16:45:48Z), the push that carried the last `src/` change `1eb130d`. `git diff b6c2462..347e298` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty, so those runs bind the current runtime and kit content. The docs-only heads after it have 0 check runs and match no workflow path filter.
Served kit `0.1.21` (source `b6c2462`): 33 of 35 kit files byte-verified against the repository at `b6c2462` through the `export-kit/kit.config.json` mappings. `LICENSES/noisemaker-MIT.txt` byte-matches the noisemaker reference `LICENSE` at `e73a44a`. `compat.json` declares exactly the 205 catalog IDs. Raw bytes were compared, not summaries.
The 2026-09-26 declared publication checks (fmt, clippy, check, release tests on 1.85.0 and stable, package) are carried, not re-executed: the runtime tree is unchanged (`git diff 7f55619..347e298 -- src/ scripts/ examples/ tests/ Cargo.toml Cargo.lock` is empty) and the rebuilt binary is byte-identical.

### Daily review, 2026-09-25

The current generated-bundle check passes. Exact-source Cargo CI succeeds, but retains two ignored documentation tests. This does not establish current full rendered parity or the installed CLI on every declared platform. GAP-001 remains open. The 205-ID served declaration leaves five current IDs absent. [Raw evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/rust-ci-36082527571.log).
The review checked source changes, worker evidence, source-bound CI where present, and current served inventories. Full installed-host and platform qualification remains incomplete.

Environment: macOS 26.5, Darwin arm64.
[Source SHA-256 records](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/noisemaker-for-rust-source-hashes.json) bind these checks to the reviewed revision.
[Raw command evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/rust-tests.json). [Remote evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/noisemaker-for-rust-remote.json).

Executed command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check
```

The generated-bundle check exited 0. This is a reproducibility check, not a render or Cargo package installation test. Final exit code: 0.
No image denominator or tolerance follows from a unit-test or generated-file result.
Official reference: [Current Cargo Book, accessed 2026-09-24](https://doc.rust-lang.org/cargo/reference/publishing.html).

| Outcome | Observed scope | Remaining work |
|---|---|---|
| Installation | Instructions and metadata inspected | Install the actual artifact privately. |
| First useful output | Selected checks only | Install the crate into a private root. Render a gradient and DSL chain, apply PNG input, test invalid output, and remove the installation. |
| Host integration | Not fully exercised | Check parameters, external inputs, state, resize, and cleanup. |
| Errors and recovery | Only the selected checks above | Fail through the installed entry point, correct input, and render again. |
| Distribution | Metadata inspection | Qualify distribution contents and lifecycle. |
| Accessibility | Not observed | Check keyboard, focus, labels, and diagnostics for provided interfaces. |

Headless libraries do not require an editor accessibility test. Their CLI diagnostics and failure handling still require checks.
Host presence does not prove host qualification. This pass made no global installation or user-project changes.

### Installed workflow, 2026-09-26 (GAP-002)

Executed at git `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (source tree of audited base `31fb9dc`) on Linux x86_64 (AMD EPYC 7713) with an isolated consumer (private `CARGO_HOME`/install root, no global changes):

```sh
cargo +1.85.0 check --workspace --all-targets --locked   # exit 0
cargo +stable check --workspace --all-targets --locked   # exit 0
cargo +1.85.0 test --locked --release                    # exit 0
cargo +stable test --locked --release                    # exit 0
cargo +stable package --locked                           # exit 0 (isolated git-archive copy)
cargo +<T> install --locked --root <private> --path <unpacked crate>   # exit 0 for 1.85.0 and stable
noisemaker-rs effects | wc -l                            # 205; but `effects | head -3` exits 101 (broken-pipe panic)
noisemaker-rs generate / run / apply                     # exit 0 (gradient, DSL chain, PNG input)
noisemaker-rs generate --param colorCount=99             # exit 1, "Parameter \"colorCount\" must be at most 4"
noisemaker-rs generate --param colorCount=3              # exit 0 (recovery)
# read-only directory: exit 1 "Permission denied", pre-existing destination preserved
# SIGTERM mid-render: pre-existing destination preserved, no temporary files left
cargo +<T> uninstall --root <private> noisemaker-for-rust              # exit 0, private bin empty
python3 scripts/parity.py --rust <installed binary> --js <pinned oracle f2eb495d...> --timeout 120
# stable: 202/205 byte-exact, 3 unsupported, 0 errors; 1.85.0 focused: 2/2 byte-exact, 0 errors
```

Raw logs, PNG outputs, hashes, parity reports, host/licensing/input identification, and the unavailable-platform list are retained in-repo at [parity/installed-workflow-20260926/](parity/installed-workflow-20260926/).

## 4. Known gaps

P1 means false completion or major correctness failure. P2 means coverage or integration uncertainty. P3 means documentation inconsistency.
These entries record missing qualification. They do not infer implementation defects from absent tests.

### GAP-001: current authority and parity qualification

- Status: closed. Priority: P2. Category: verification.
- Affected scope: Cargo.toml, src/generated/, scripts/parity.py, scripts/generate_bundle.py, README.md
- Expected behavior: Reproducible evidence binds each supported claim to the port and authority revisions.
- Observed behavior: Five upstream effects remain intentionally excluded. Generated-file consistency does not establish pixel parity or validate special semantic adapters.
- Evidence: 2026-09-25 rendered parity run at `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` against the pinned JavaScript CPU oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` on an identified CPU revision (AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic). Denominator: 205 catalog records. Result: 202 compared, all byte-exact at tolerance 0; 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`); 0 errors. The default 30-second timeout first reported `filter3d/flow3d` as a timeout failure; the retained 120-second retry compares byte-exact (rust 66.841 s). Parameters and per-record metrics: [report](parity/parity-report-20260925.json), [raw output](parity/parity-run-20260925.log), [flow3d retry](parity/parity-flow3d-120s-retry.json), [source hashes and CPU identity](parity/source-identity.json). The five upstream effects excluded from this standalone CPU port (`render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`) remain excluded and reported as excluded, not as passes. Re-verified 2026-09-27 at the current runtime tree (source `347e298`) against the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`): 202 compared byte-exact at tolerance 0, 3 unsupported with the same reason, 0 errors; the upstream shader-language changes over `6a0af04d..7443f6e6` did not change any compared render (section 3, 2026-09-27 block).
- Next action: None for the rendered parity denominator. Parameter sweeps and stateful sequences remain separate open qualification; the installed CLI (GAP-002, closed 2026-09-26) and platform matrices (GAP-003) are tracked in their own entries.
- Dependencies: Resolved. Immutable authority inputs are pinned by commit and tree hash in `docs/parity/source-identity.json`; historical goldens and provenance are unchanged, and no golden was regenerated.
- Acceptance criteria: Met. Every applicable case, parameter choice, exclusion, error, and tolerance is reported; the denominator stays at 205.
- Required checks: Existing compiler gates unchanged. The rendered parity evidence is retained in-repo with raw output, exact source hashes, and the identified CPU revision; a CI-declared parity gate is not part of this candidate (workflow changes are out of scope for this job).
- Last verification: 2026-09-27 (205-record run at the current runtime tree, source `347e298`, against the noisemaker-for-cpu `7a824744` oracle; 202 byte-exact, 3 unsupported, 0 errors).

### GAP-002: installed developer workflow qualification

- Status: closed. Priority: P2. Category: usability.
- Affected scope: Public API, examples, supported hosts, errors, recovery, and lifecycle.
- Expected behavior: Developers can install, produce useful output, integrate it, recover from errors, and remove the package.
- Observed behavior: The complete installed workflow was exercised in a private root on 2026-09-26 at git `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (the audited base `31fb9dc` tree) on Rust 1.85.0 (MSRV) and stable 1.98.1: `cargo check --all-targets` and the full release test suite passed on both toolchains; `cargo package --locked` in an isolated copy; `cargo install` of the packaged crate into an isolated private root on both toolchains (binary reports `noisemaker-rs 0.1.0`); the installed CLI printed 205 `effects` records (a truncated `effects | head -3` reader hits a broken-pipe panic and exit 101 — recorded as a limitation below), rendered a gradient PNG, a chained DSL program PNG, and a `filter/blur` render of a PNG input; invalid values (`colorCount=99`, unknown `bogus`) failed with actionable diagnostics (exit 1) and a corrected run recovered (exit 0); a read-only output directory failed with `Permission denied` (exit 1) while preserving the pre-existing destination; a SIGTERM mid-render preserved the pre-existing destination and left no temporary files; uninstall removed the binary cleanly (exit 0, empty private bin). Rendered parity against the pinned upstream oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` was confirmed on this run with the stable installed binary (202/205 compared byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors) and a focused 1.85.0 comparison (2/2 byte-exact, 0 errors).
- Evidence: retained in-repo at [parity/installed-workflow-20260926/](parity/installed-workflow-20260926/) — steps, commands, exit codes, artifact and output sha256s ([sha256sums](parity/installed-workflow-20260926/sha256sums.txt) and [external artifact hashes](parity/installed-workflow-20260926/sha256sums-external.txt)), raw logs, PNG outputs, oracle parity reports, host identity, licensing and input-requirement identification, unavailable-platform list, and limitations. Also [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md), [official reference](https://doc.rust-lang.org/cargo/reference/publishing.html), and section 3.
- Next action: none for the installed workflow. Noted limitation: host coverage was single-platform (Linux x86_64, identified AMD EPYC 7713; CPU rendering only — no GPU path exists); macOS and Windows were not exercised and remain explicitly unavailable on this run (GAP-003); `animate`/ffmpeg guidance was not separately re-verified. Minor usability wart observed and retained: `noisemaker-rs effects` panics with a broken-pipe error and exits 101 when its stdout is truncated early (`effects | head -3`, library/std/src/io/stdio.rs broken pipe, os error 32) instead of exiting cleanly; a full untruncated read (`effects | wc -l`) prints all 205 records. This is recorded as an observed limitation, not fixed in this register (implementation belongs to the separate job).
- Dependencies: Resolved. An isolated consumer was used (private `CARGO_HOME`/install root, no global changes). Licensing: the package is MIT ([LICENSE](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/LICENSE)); the packaged crate carries the MIT license file (packaging listing retained at [package.log](parity/installed-workflow-20260926/package.log)). Input requirements: the CLI takes catalog effect IDs, DSL programs, and optional PNG inputs via `--input`/`--texture`; external-texture effects require a PNG binding, and both the generator and PNG-input paths were exercised. Host identity: [host.txt](parity/installed-workflow-20260926/host.txt) (AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic); GPU: none — CPU-only rendering by design (README.md:14-19).
- Acceptance criteria: Met. Artifact hashes (crate, installed binaries, PNG outputs), steps, meaningful output, error diagnostics, recovery results, and cleanup results are retained in-repo (see evidence above).
- Required checks: Met. Minimum (1.85.0) and current (stable 1.98.1) supported versions were both tested (check + full release test suite) and installed from the same packaged crate; cancellation and file preservation were checked (SIGTERM preservation + read-only destination preservation); unavailable platforms are kept explicit (macOS/Windows untested, no GPU path).
- Last verification: 2026-09-26 (executed run at `81953750ac7268ed6b46497ee3e3ea620ebb41c1`; evidence retained in-repo). Carried, not re-executed, on 2026-09-27: `git diff 7f55619..347e298 -- src/ scripts/ examples/ tests/ Cargo.toml Cargo.lock` is empty and the committed evidence files are intact at the audited source, so the measured workflow still binds the current runtime tree.

### GAP-003: distribution and release qualification

- Status: closed (2026-09-26). Priority: P2. Category: release.
- Affected scope: Actual artifact, dependencies, notices, version promises, and release evidence.
- Expected behavior: The delivered artifact supports its documented installation and first useful result.
- Observed behavior: `cargo package --locked --no-verify` was executed in a fresh isolated copy (`git archive HEAD`) of the audited source tree (`ff250e8e91e7003a918c7943146fc64d0fb123bc`; code identical to CI source `31fb9dc168a2f50aac99b6c379950767b06c4c62`): PACKAGE-EXIT 0, "Packaged 52 files, 34.3MiB" ([package.log](parity/distribution-20260926/package.log)). The crate sha256 (`db74b57f…3171a`) reproduced identically from two independent package runs ([crate-sha256.txt](parity/distribution-20260926/crate-sha256.txt)). The extracted archive's 52-file inventory was byte-compared 52/52 between the two runs ([inventory-check.log](parity/distribution-20260926/inventory-check.log), per-file hashes in [archive-files-sha256.txt](parity/distribution-20260926/archive-files-sha256.txt)); notices LICENSE, README.md, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, and TRADEMARK.md are all present in the archive; the dependency tree is retained ([dep-tree.txt](parity/distribution-20260926/dep-tree.txt), [cargo-metadata.json](parity/distribution-20260926/cargo-metadata.json)). Private installs from the unpacked, byte-verified archive on Rust 1.85.0 and stable 1.98.1 (private `CARGO_HOME` and install root, no global changes; [steps-1.85.0.log](parity/distribution-20260926/steps-1.85.0.log) / [steps-stable.log](parity/distribution-20260926/steps-stable.log)): version printed, 205-record effects catalog read ([effects-*.txt](parity/distribution-20260926/sha256sums.txt)), both packaged examples built and run to PNGs, a gradient rendered from the installed binary, a force-reinstall upgrade over the existing root re-rendered byte-identically, and uninstall/reinstall both succeeded (all exit codes 0, post-upgrade render identical). Known limitation retained with direct evidence: `cargo install <file>.crate` is not supported by cargo — a real invocation of `cargo install --locked --root <R> <file>.crate` exits 101 on both toolchains with no binary installed (1.85.0 resolves the argument as a registry package spec; stable rejects it as an invalid package name; [install-from-crate.log](parity/distribution-20260926/install-from-crate.log)) — so installation used the unpacked archive instead (the accepted path in [steps-1.85.0.log](parity/distribution-20260926/steps-1.85.0.log) / [steps-stable.log](parity/distribution-20260926/steps-stable.log)).
- Evidence: retained in-repo at [parity/distribution-20260926/](parity/distribution-20260926/) — packaging log, crate sha256, per-file inventory and byte comparison, notices, dependency inventory, install/upgrade/removal logs with exit codes, example and installed-binary PNG outputs, host and toolchain identity ([host.txt](parity/distribution-20260926/host.txt), [toolchains.txt](parity/distribution-20260926/toolchains.txt)), and exact-source CI job records. Exact-source CI: the `quality` and `test-and-package` jobs are green at source revision `31fb9dc168a2f50aac99b6c379950767b06c4c62` with step-level inspection — 0 skipped steps and 0 failed steps ([jobs-36208792712.json](parity/distribution-20260926/jobs-36208792712.json)); the export-kit dispatch job is green ([jobs-36208792819.json](parity/distribution-20260926/jobs-36208792819.json)). Every commit from `31fb9dc` through this candidate touches only `docs/**`, and `ci.yml` path filters exclude docs-only revisions, so no check runs are expected at the docs-only candidate ([check-runs.json](parity/distribution-20260926/check-runs.json), 0 runs). Actual render legs: the 205-record rendered-parity denominator binds to the stable binary ([parity-run-20260926.log](parity/parity-run-20260926.log)), with the focused 1.85.0 leg at [parity-1.85.log](parity/installed-workflow-20260926/parity-1.85.log); the 1.85.0 gap is recorded as a remaining limit, not a pass.
- Next action: none for distribution qualification. Remaining limits: macOS/Windows platform matrix, parameter sweeps beyond harness defaults, and broader 1.85.0 artifact-parity coverage remain unqualified. The broken-pipe exit-101 limitation of `noisemaker-rs effects` under truncated stdout is retained unfixed ([effects-stable.log](parity/installed-workflow-20260926/effects-stable.log)).
- Dependencies: Resolved. The GAP-002 dependency was resolved 2026-09-26. Source CI is distinguished from downstream publication and native rendering: source CI was inspected at job and step level; crates.io publication is downstream and out of scope.
- Acceptance criteria: Met. Artifact bytes matched to their per-file inventory (52/52 across two independent package runs with an identical crate sha256); notices and dependencies checked; installation, packaged examples, upgrade, and removal all passed on Rust 1.85.0 and stable 1.98.1.
- Required checks: Met. Exact-source CI jobs were inspected at step level (0 skips, 0 failures) and the actual render legs were counted from the retained 205-record parity run rather than trusting green summaries.
- Last verification: 2026-09-26 (executed run; evidence retained in-repo). This register does not approve a release. Carried, not re-executed, on 2026-09-27: the runtime and `export-kit/` trees are unchanged from `b6c2462` through `347e298` and the committed evidence files are intact. The served kit `0.1.21` was byte-verified against the repository on 2026-09-27 (section 3).

## 5. Ordered next actions

Current first action: Rendered parity re-measured 2026-09-27 at the newest CPU delivery (section 3); no implementation is authorized by this audit. Remaining qualification limits: the macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, the unresolved ignored doctests, and the retained broken-pipe limitation. The port's parameter-contract provenance pin stays at `6a0af04d` while the CPU authority pins `7443f6e6`; the snapshot effect data is byte-identical across that range, so the provenance bump belongs to the separate sync job.
Subsequent historical actions remain dependent on that evidence.

1. Resolve authority identities for GAP-001. Retain earlier denominators, goldens, tolerances, and exclusions.
2. Execute the installed workflow for GAP-002. Record meaningful output, failure recovery, versions, and cleanup. (Done 2026-09-26; see section 3 and [parity/installed-workflow-20260926/](parity/installed-workflow-20260926/).)
3. Run compiler and rendered parity for GAP-001. Keep structural, numerical, and platform evidence separate.
4. Qualify distribution contents and lifecycle for GAP-003 after the installed workflow passes. (Done 2026-09-26; see section 3 and [parity/distribution-20260926/](parity/distribution-20260926/).)
5. Record measured results. Close entries only when their acceptance criteria pass.

Implementation belongs to the separate job. Do not port additional effects or advance the current parity checkpoint through this register.

## 6. Pass history

2026-09-27 daily review at `347e298348fbf61993f537f2a7689e798674d7e4`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`); served kit `0.1.21` byte-verified; no gap opened, closed, or reopened. GAP-002 and GAP-003 evidence carried across the unchanged runtime tree. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.

| Date | Source SHA | Changes | Tested scope | Remaining limits |
|---|---|---|---|---|
| 2026-09-27 | `347e298348fbf61993f537f2a7689e798674d7e4` | Rendered parity re-measured at the newest CPU delivery end: 205-record run against the noisemaker-for-cpu `7a824744` oracle (upstream pin `7443f6e6`, pin range `6a0af04d..7443f6e6` audited upstream: language sources changed, effect catalog unchanged); 202 byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors; `filter3d/flow3d` byte-exact in-run at 72.072 s. Rebuilt binary byte-identical to the prior audited binary. Served kit `0.1.21` (source `b6c2462`) byte-verified: 33/35 files direct, upstream MIT notice, compat ID set 205/205. Exact-source CI green at `b6c2462` head; runtime tree identical to `347e298`. GAP-002/GAP-003 evidence carried with the empty runtime diff cited. Workflow (CI) changes were not made. | Parity, bundle check, 19 script tests, kit bytes, CI mapping, upstream and CPU range classification (section 3, 2026-09-27 block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, ignored doctests, and the broken-pipe exit-101 limitation remain unqualified/unfixed. |
| 2026-09-26 (GAP-003) | `ff250e8e91e7003a918c7943146fc64d0fb123bc` (code identical to CI source `31fb9dc`) | GAP-003 closed: `cargo package --locked --no-verify` in a fresh isolated copy of the audited tree; crate sha256 `db74b57f…3171a` reproduced identically across two independent package runs; 52-file archive inventory byte-compared 52/52 between runs; notices (LICENSE, README.md, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, TRADEMARK.md) and the dependency tree retained; private installs from the byte-verified archive on Rust 1.85.0 and stable 1.98.1 with version output, the 205-record effects catalog, both packaged examples rendered to PNGs, an installed-binary gradient render, a force-reinstall upgrade with byte-identical post-upgrade render, and uninstall/reinstall (all exit codes 0). Exact-source CI inspected at step level: 0 skipped steps, 0 failed steps at `31fb9dc` (docs-only revisions after it intentionally trigger no CI). Raw artifacts in `docs/parity/distribution-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1); artifact bytes matched to inventory; notices and dependencies checked; packaged examples, upgrade, and removal exercised. | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, and the broken-pipe exit-101 limitation under truncated stdout remain unqualified/unfixed. |
| 2026-09-26 | `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (source tree of audited base `31fb9dc`) | GAP-002 closed: executed the complete installed workflow in a private root on Rust 1.85.0 and stable 1.98.1 — version-matrix check/test, `cargo package --locked` in an isolated copy, private install of the packaged crate, meaningful output (205 effects records, gradient, DSL chain, PNG-input apply), invalid-value diagnostics + recovery, read-only-output preservation, SIGTERM cancellation + preservation, and clean uninstall. Oracle parity with the installed stable binary: 202/205 byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors; focused 1.85.0 comparison 2/2 byte-exact. Raw logs, PNGs, hashes, and reports in `docs/parity/installed-workflow-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1); cancellation and file preservation checked; unavailable platforms explicit (macOS/Windows untested; CPU-only, no GPU path). | Distribution lifecycle and artifact bytes (GAP-003), parameter sweeps beyond harness defaults, and the macOS/Windows platform matrix remain unqualified. |
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | GAP-001 closed: executed the full 205-record rendered parity run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle on an identified CPU (AMD EPYC 7713, x86_64). | 202/205 compared byte-exact at tolerance 0; 3 unsupported with the stable overlay-interface reason; 0 errors. Default 30 s timeout failure on `filter3d/flow3d` retained; the 120 s retry compares byte-exact. Raw report, log, retry, and source hashes in `docs/parity/`. Workflow changes were not made. | Installation (GAP-002), distribution (GAP-003), parameter sweeps beyond harness defaults, and platform matrices remain unqualified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Created six-section register and README link. No closures. | The generated-bundle check exited 0. This is a reproducibility check, not a render or Cargo package installation test. | Full audit, installed workflows, current rendered parity, platforms, and releases remain unqualified. |

Run ID: `20260924-remaining-gap-documents`.
[Operational evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents). Creating this register does not advance successful-audit timestamps or the rotation.
