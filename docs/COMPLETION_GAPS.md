# noisemaker-for-rust: completion gaps

Current compatibility matrix: [compatibility report](COMPATIBILITY.md).

## 1. Scope and source revisions

Daily review: 2026-09-25. Current inspected source: [`25f1340c6b087d648bb5b4249b2f4d94be5d5c02`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/25f1340c6b087d648bb5b4249b2f4d94be5d5c02).
Full rendered parity at this SHA: measured (GAP-001). Installed developer workflow: measured on 2026-09-26 at `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (GAP-002; Linux x86_64, CPU rendering only). Distribution and platform qualification remain unverified (GAP-003). No release approval follows from this review.
Current upstream discovery: `bbdeb56c4b75cf33379766c3e87b0f5a18bcbba8`. Published Noisemaker authority: `1.0.179`, source `fca611fd8f91424661d4e531d39313d24ea21134`, 210 effect IDs.
The observations below retain their original source and authority identities. They do not qualify later updates.
Current served kit: `0.1.16`, source `9a043c4ea38766d356fcd20c0d838149558cf0fd`. [Retrieved inventory and hashes](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/current-served-inventories.json). Artifact identity does not establish host qualification.

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
| Distribution | Metadata inspection | Run cargo package --locked in an isolated copy. Install the archive privately on Rust 1.85 and stable. Check packaged examples and notices. |
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
noisemaker-rs effects / generate / run / apply           # exit 0 (205 records, gradient, DSL chain, PNG input)
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
- Evidence: 2026-09-25 rendered parity run at `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` against the pinned JavaScript CPU oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` on an identified CPU revision (AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic). Denominator: 205 catalog records. Result: 202 compared, all byte-exact at tolerance 0; 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`); 0 errors. The default 30-second timeout first reported `filter3d/flow3d` as a timeout failure; the retained 120-second retry compares byte-exact (rust 66.841 s). Parameters and per-record metrics: [report](parity/parity-report-20260925.json), [raw output](parity/parity-run-20260925.log), [flow3d retry](parity/parity-flow3d-120s-retry.json), [source hashes and CPU identity](parity/source-identity.json). The five upstream effects excluded from this standalone CPU port (`render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`) remain excluded and reported as excluded, not as passes.
- Next action: None for the rendered parity denominator. Parameter sweeps and stateful sequences remain separate open qualification; the installed CLI (GAP-002, closed 2026-09-26) and platform matrices (GAP-003) are tracked in their own entries.
- Dependencies: Resolved. Immutable authority inputs are pinned by commit and tree hash in `docs/parity/source-identity.json`; historical goldens and provenance are unchanged, and no golden was regenerated.
- Acceptance criteria: Met. Every applicable case, parameter choice, exclusion, error, and tolerance is reported; the denominator stays at 205.
- Required checks: Existing compiler gates unchanged. The rendered parity evidence is retained in-repo with raw output, exact source hashes, and the identified CPU revision; a CI-declared parity gate is not part of this candidate (workflow changes are out of scope for this job).
- Last verification: 2026-09-25 (local 205-record run against the pinned oracle at the tested source revision).

### GAP-002: installed developer workflow qualification

- Status: closed. Priority: P2. Category: usability.
- Affected scope: Public API, examples, supported hosts, errors, recovery, and lifecycle.
- Expected behavior: Developers can install, produce useful output, integrate it, recover from errors, and remove the package.
- Observed behavior: The complete installed workflow was exercised in a private root on 2026-09-26 at git `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (the audited base `31fb9dc` tree) on Rust 1.85.0 (MSRV) and stable 1.98.1: `cargo check --all-targets` and the full release test suite passed on both toolchains; `cargo package --locked` in an isolated copy; `cargo install` of the packaged crate into an isolated private root on both toolchains (binary reports `noisemaker-rs 0.1.0`); the installed CLI produced 205 `effects` records, a gradient PNG, a chained DSL program PNG, and a `filter/blur` render of a PNG input; invalid values (`colorCount=99`, unknown `bogus`) failed with actionable diagnostics (exit 1) and a corrected run recovered (exit 0); a read-only output directory failed with `Permission denied` (exit 1) while preserving the pre-existing destination; a SIGTERM mid-render preserved the pre-existing destination and left no temporary files; uninstall removed the binary cleanly (exit 0, empty private bin). Rendered parity against the pinned upstream oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` was confirmed on this run with the stable installed binary (202/205 compared byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors) and a focused 1.85.0 comparison (2/2 byte-exact, 0 errors).
- Evidence: retained in-repo at [parity/installed-workflow-20260926/](parity/installed-workflow-20260926/) — steps, commands, exit codes, artifact and output sha256s ([hashes.md](parity/installed-workflow-20260926/hashes.md)), raw logs, PNG outputs, oracle parity reports, host identity, licensing and input-requirement identification, unavailable-platform list, and limitations. Also [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md), [official reference](https://doc.rust-lang.org/cargo/reference/publishing.html), and section 3.
- Next action: none for the installed workflow. Noted limitation: host coverage was single-platform (Linux x86_64, identified AMD EPYC 7713; CPU rendering only — no GPU path exists); macOS and Windows were not exercised and remain explicitly unavailable on this run (GAP-003); `animate`/ffmpeg guidance was not separately re-verified.
- Dependencies: Resolved. An isolated consumer was used (private `CARGO_HOME`/install root, no global changes). Host, GPU, licensing, and input requirements are identified in the retained evidence ([parity/installed-workflow-20260926/README.md](parity/installed-workflow-20260926/README.md) section 7).
- Acceptance criteria: Met. Artifact hashes (crate, installed binaries, PNG outputs), steps, meaningful output, error diagnostics, recovery results, and cleanup results are retained in-repo (see evidence above).
- Required checks: Met. Minimum (1.85.0) and current (stable 1.98.1) supported versions were both tested (check + full release test suite) and installed from the same packaged crate; cancellation and file preservation were checked (SIGTERM preservation + read-only destination preservation); unavailable platforms are kept explicit (macOS/Windows untested, no GPU path).
- Last verification: 2026-09-26 (executed run at `81953750ac7268ed6b46497ee3e3ea620ebb41c1`; evidence retained in-repo).

### GAP-003: distribution and release qualification

- Status: open. Priority: P2. Category: release.
- Affected scope: Actual artifact, dependencies, notices, version promises, and release evidence.
- Expected behavior: The delivered artifact supports its documented installation and first useful result.
- Observed behavior: Complete artifact reproduction, installation, upgrade, and removal remain unverified.
- Evidence: [Distribution instructions](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md), section 1, and exact-source CI in section 2.
- Next action: Run cargo package --locked in an isolated copy. Install the archive privately on Rust 1.85 and stable. Check packaged examples and notices.
- Dependencies: The GAP-002 dependency is resolved (installed workflow passed on 2026-09-26 with retained evidence). Distinguish source CI from downstream publication and native rendering.
- Acceptance criteria: Match artifact bytes to their inventory. Check notices and dependencies. Pass installation, examples, upgrade, and removal.
- Required checks: Inspect exact-source CI jobs and actual render legs. Count skips and errors rather than trusting green summaries.
- Last verification: 2026-09-24. This register does not approve a release.

## 5. Ordered next actions

Current first action: Qualify distribution contents and lifecycle for GAP-003: run cargo package --locked in an isolated copy, install the archive privately on Rust 1.85 and stable, check packaged examples and notices, and match artifact bytes to their inventory. The installed developer workflow (GAP-002) passed on 2026-09-26 with retained evidence, so this dependency is resolved.
Subsequent historical actions remain dependent on that evidence. No implementation is authorized by this audit.

1. Resolve authority identities for GAP-001. Retain earlier denominators, goldens, tolerances, and exclusions.
2. Execute the installed workflow for GAP-002. Record meaningful output, failure recovery, versions, and cleanup. (Done 2026-09-26; see section 3 and [parity/installed-workflow-20260926/](parity/installed-workflow-20260926/).)
3. Run compiler and rendered parity for GAP-001. Keep structural, numerical, and platform evidence separate.
4. Qualify distribution contents and lifecycle for GAP-003 after the installed workflow passes.
5. Record measured results. Close entries only when their acceptance criteria pass.

Implementation belongs to the separate job. Do not port additional effects or advance the current parity checkpoint through this register.

## 6. Pass history

2026-09-25 daily review at `9a043c4ea38766d356fcd20c0d838149558cf0fd`: source freshness and bounded evidence reviewed. Open qualification limits retained. [Retained review evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/rust-ci-36082527571.log). No new closure claimed.

| Date | Source SHA | Changes | Tested scope | Remaining limits |
|---|---|---|---|---|
| 2026-09-26 | `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (source tree of audited base `31fb9dc`) | GAP-002 closed: executed the complete installed workflow in a private root on Rust 1.85.0 and stable 1.98.1 — version-matrix check/test, `cargo package --locked` in an isolated copy, private install of the packaged crate, meaningful output (205 effects records, gradient, DSL chain, PNG-input apply), invalid-value diagnostics + recovery, read-only-output preservation, SIGTERM cancellation + preservation, and clean uninstall. Oracle parity with the installed stable binary: 202/205 byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors; focused 1.85.0 comparison 2/2 byte-exact. Raw logs, PNGs, hashes, and reports in `docs/parity/installed-workflow-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1); cancellation and file preservation checked; unavailable platforms explicit (macOS/Windows untested; CPU-only, no GPU path). | Distribution lifecycle and artifact bytes (GAP-003), parameter sweeps beyond harness defaults, and the macOS/Windows platform matrix remain unqualified. |
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | GAP-001 closed: executed the full 205-record rendered parity run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle on an identified CPU (AMD EPYC 7713, x86_64). | 202/205 compared byte-exact at tolerance 0; 3 unsupported with the stable overlay-interface reason; 0 errors. Default 30 s timeout failure on `filter3d/flow3d` retained; the 120 s retry compares byte-exact. Raw report, log, retry, and source hashes in `docs/parity/`. Workflow changes were not made. | Installation (GAP-002), distribution (GAP-003), parameter sweeps beyond harness defaults, and platform matrices remain unqualified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Created six-section register and README link. No closures. | The generated-bundle check exited 0. This is a reproducibility check, not a render or Cargo package installation test. | Full audit, installed workflows, current rendered parity, platforms, and releases remain unqualified. |

Run ID: `20260924-remaining-gap-documents`.
[Operational evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents). Creating this register does not advance successful-audit timestamps or the rotation.
