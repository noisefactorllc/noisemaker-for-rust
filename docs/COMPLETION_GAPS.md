# noisemaker-for-rust: completion gaps

Current compatibility matrix: [compatibility report](COMPATIBILITY.md).

## 1. Scope and source revisions

Daily review: 2026-09-27, second pass. Current inspected source: [`84b8168a2337ef846a6846d09c4cf32e0b481928`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/84b8168a2337ef846a6846d09c4cf32e0b481928). This head is docs-only above `863f64a`. The runtime tree is unchanged.
Full rendered parity at this SHA: measured 2026-09-27 and re-executed by this review (section 3, GAP-001). Installed developer workflow and distribution qualification: measured 2026-09-26 on Linux x86_64 (GAP-002, GAP-003). Both results are carried across the docs-only revisions after `7f55619` because the runtime tree is unchanged. The macOS and Windows platform matrix remains unmeasured. No release approval follows from this review.
Current upstream discovery (review, 2026-09-27T13:45Z): `93229933b102ba82e713402be19db57207698850`, release `1.0.194` (11:35Z). The audit's 11:05Z observation recorded `1.0.192`. Release `1.0.193` was already published at 10:00Z. Release `1.0.193` is the CPU-pinned revision `12b4d74f`. The `1.0.193` and `1.0.194` manifests are byte-identical to the recorded 1.0.176 manifest. The effect catalog is unchanged. Upstream commits above the CPU pin remain unqualified. They belong to the CPU sync queue.
The observations below retain their original source and authority identities. They do not qualify later updates.
Current served kit: `0.1.21`, source `b6c24628f6b8082f07a8a0a1a0110661325d10ef`. Byte-checked against the repository on 2026-09-27 (section 3). Artifact identity does not establish host qualification.

### Earlier source observations

Date: 2026-09-24. Reviewed SHA: [`2ef1cc4…`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/2ef1cc4179f5163023c26e785f533d07cf699fb4).
Local HEAD matched remote main before checks. The operator requested registers for all remaining eligible ports in this run.
This initial register contains bounded evidence. It is not a completed port audit or release approval.
No implementation or parity checkpoint changed. Full audits remain in the rotation.

Rust CPU renderer with 205 effects. Cargo package noisemaker-for-rust exports library noisemaker_cpu and command noisemaker-rs. Rust 1.85 is required. [Contract](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md).
Generated catalog provenance records parameter-contract revision `44bc4ed4ac729bddaa95b083d64bee942ade35da`. Broader generated-source identity needs reconciliation with current authority.
Current upstream at discovery: `c9ee8a049b2b63cd300da67c01ee40baf29dc288`.
Current CPU authority: `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e`.
These authority heads are review targets, not qualification results. No goldens were regenerated.
Served kit `0.1.13` identifies `2ef1cc4179f5163023c26e785f533d07cf699fb4`. [Metadata](https://kits.noisedeck.app/rust/0/deployment-meta.json). Inventory and compatibility metadata were retrieved. Complete artifact bytes were not examined.
These document paths do not match the current publication workflow filters.
The containing commit identifies this register's publication revision. The shared run record retains commits, remote hashes, and downstream results.

## 2. Completion claims

| Claim ID | Claim source | Claimed scope | Finding | Evidence |
|---|---|---|---|---|
| CLAIM-001 | [Historical source](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Exact RGBA8 parity across the catalog with explicit unsupported interfaces. Runtime rendering does not require JavaScript or a GPU. | partial | The generated-bundle gate exited 0. This is a reproducibility examination, not a render or Cargo package installation test. |
| CLAIM-002 | [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Human usability: installation, output, errors, and recovery | supported | The complete installed workflow was exercised on 2026-09-26. Evidence retained ([installed workflow](parity/installed-workflow-20260926/)). GAP-002. |
| CLAIM-003 | [Ecosystem reference](https://doc.rust-lang.org/cargo/reference/publishing.html) | Ecosystem fit and version support. | supported | The packaged crate was installed from an isolated copy into a private root on Rust 1.85.0 and stable 1.98.1. Both hosts rendered and uninstalled cleanly. |
| CLAIM-004 | [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md) | Release readiness | unverified | Metadata and CI do not replace installation of the actual artifact. GAP-003. |
| CLAIM-005 | [Exact-source Actions](https://github.com/noisefactorllc/noisemaker-for-rust/actions?query=head_sha%3A2ef1cc4179f5163023c26e785f533d07cf699fb4) | Workflow status only | supported | [Export kit](https://github.com/noisefactorllc/noisemaker-for-rust/actions/runs/35829708764): `success`. [ci](https://github.com/noisefactorllc/noisemaker-for-rust/actions/runs/35829708776): `success`. |

## 3. Methods and evidence

Review CI boundary: Exact-source runs: Export kit, ci. A passing export dispatch does not qualify rendered parity. Current complete-render enforcement remains an open requirement. [Exact-source responses](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/noisemaker-for-rust-remote-evidence.json).

### Independent review, 2026-09-27

This review checked worker run `audit-20260927-093000`, the follow-up `dedfd07` commit `84b8168`, and the range after the last reviewed run.
Every commit in `b6c2462..84b8168` touches only `docs/**`. The runtime diff over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty.
The committed `dedfd07` report and log match their recorded sha256 values (`9717bb79…`, `b8a63de6…`).
Environment: Linux x86_64 (AMD EPYC 7713, Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 installed user-level. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <fresh clone of noisemaker-for-cpu at dedfd07>/bin/noisemaker-cpu.js --timeout 120 --json <report>   # exit 0
NOISEMAKER_JS_CPU_DIR=<oracle> cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
target/release/noisemaker-rs effects | head -3   # rust exit 101, broken-pipe panic
target/release/noisemaker-rs generate --param colorCount=99 synth/gradient   # exit 1, actionable diagnostic
target/release/noisemaker-rs generate --param colorCount=3 --width 8 --height 8 --output <png> synth/gradient   # exit 0
```

Independent parity result: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 66.916 s.
Report sha256 `d8390a59d7e9ed39271ccd03b9e7eb4fd2f805d0254b89343cec66af1eff422e`. Run log sha256 `7603601aad9ca5d7a9580a1120be62ea5c941cac085f84015fef194512fe4694`. Evidence record: `/series/review-20260927-133500/result.json`.
The two ignored CI tests are `precision_boundaries_are_byte_exact` and `three_effect_js_oracle_parity_is_byte_exact` in `tests/parity_spine.rs`. They are oracle-gated, not broken. Both pass with `NOISEMAKER_JS_CPU_DIR` set (2 passed, 0 failed). CI resolution stays with the implementation job.
Served kit spot-check: `engine/src/lib.rs`, `engine/Cargo.toml`, and `engine/Cargo.lock` match `git show b6c2462:<path>` byte-exactly. `LICENSES/noisemaker-MIT.txt` matches the noisemaker `LICENSE` at `12b4d74f`.
The GAP-002 evidence manifest hash-matches 37 of 37 files. The GAP-003 manifest hash-matches 26 of 27 entries. Its failed entry is the manifest's own self-listing, not an evidence file.

### Daily review, 2026-09-27, source-lock delivery `dedfd07`

Newest noisemaker-for-cpu delivery: `dedfd07c24f80d9b0adddf912a4224ce6c1d795f`, upstream pin `12b4d74fb4f28d5f00bb1dde107fa8673814d8b9`. Compare URL: [7a824744..dedfd07](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/7a824744cb563f2280811f04e5a49f6792ed319d...dedfd07c24f80d9b0adddf912a4224ce6c1d795f).
Audited locally, not assumed: `7a824744cb563f2280811f04e5a49f6792ed319d` is an ancestor of `dedfd07` (`git merge-base --is-ancestor` exits 0). The new leg is one commit, `dedfd07c…` (`sync: update upstream source lock and inventory through noisemaker@12b4d74f`). The pin moved `7443f6e6..12b4d74f`.
Diffstat: README 2, docs/COMPATIBILITY 12+, docs/COMPLETION_GAPS 1+, docs/EFFECTS 2, CPU evidence records (`source-lock-sync-7443f6e6-12b4d74f-audit.json`, parity and range-audit records, two logs), `scripts/upstream/pinned-source-manifest.json` 11+, `scripts/upstream/source-lock.js` 6, snapshot 2, `test/upstream-inventory.test.js` 2. Total 13 files, 462 insertions(+), 340 deletions(-). Zero `src/` runtime files other than the generated snapshot.
Snapshot identity: sha256 at `7a824744` `7c2699a5c686e93572e56bb08f0985b22bf48a4a414799f9b15dab0da5a79759`, at `dedfd07` `2b7587bd1aefb5a9c90f0dece320127bcdd12d11c36b809b60d79b6f9d838472`. The entire snapshot diff is the revision line.
Manifest corroboration: the pinned-source-manifest diff changes the pin revision, `shaders/src/runtime/pipeline.js` (size and sha256), and adds `shaders/src/runtime/preflight.js`. No `shaders/effects` entries changed. The effect catalog is unchanged, so the port requires no source change and the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`.
Environment: Linux x86_64 (AMD EPYC 7713, Linux 6.8.0-134-generic), cargo 1.98.1 (rustup, homes under `/state/cache`), node v26.5.1, Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01 (byte-identical to the recorded binary)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <fresh clone of noisemaker-for-cpu at dedfd07>/bin/noisemaker-cpu.js --timeout 120 --json docs/parity/parity-report-20260927-dedfd07.json   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; 19 tests, OK
```

Parity result at size 8, time 0.25, seed 1, tolerance 0: 205 catalog records. 202 compared byte-exact (max delta 0). 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`). 0 errors. `filter3d/flow3d` compared byte-exact in-run at 64.103 s, inside the 120 s timeout.
Port source of the run: `863f64a247fa6255c0e2667b273dae9aedeea073`. The runtime tree is unchanged from the prior 2026-09-27 runs, and the commits after `7f55619` are docs-only. Report sha256 `9717bb7923254d5db7f6a58313812e05333290da0862e8d5511909c58430ae49`. Run log sha256 `b8a63de60572c77a62e488c91b7836a1454abb86dbffc05724076b621ab0acc9`. Raw artifacts committed: [report](parity/parity-report-20260927-dedfd07.json), [raw run log](parity/parity-run-20260927-dedfd07.log).
Exact-source CI: the newest `ci` and `Export kit` runs are green at head `b6c24628f6b8082f07a8a0a1a0110661325d10ef` (2026-09-26T16:45:48Z). The runtime diff `b6c2462..863f64a` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty, so those runs bind the current runtime and kit content.
The 2026-09-26 declared publication checks are carried, not re-executed. The runtime tree is unchanged: the diff `7f55619..863f64a` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock` is empty. The rebuilt binary is byte-identical. Carried items: fmt, clippy, check, release tests on 1.85.0 and stable, package.

### Daily review, 2026-09-27

Newest noisemaker-for-cpu delivery: `7a824744cb563f2280811f04e5a49f6792ed319d`, upstream pin `7443f6e6180300a45c5b97608459e5094504659d`.
The pin range `6a0af04d..7443f6e6` changes upstream shader-language sources and tests. Changed source: `shaders/src/lang/transform.js` (+377 lines, GAP-008 replaceEffect prediction). The effect catalog is unchanged. The CPU snapshot changes only its revision line over `901bbd9..7a824744`.
Environment: Linux x86_64 (AMD EPYC 7713, Linux 6.8.0-134-generic), cargo 1.98.1 (rustup, user-level), node v26.5.1, Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <fresh clone of noisemaker-for-cpu at 7a824744>/bin/noisemaker-cpu.js --timeout 120 --json <report>   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; 19 tests, OK
```

Parity result at size 8, time 0.25, seed 1, tolerance 0: 205 catalog records. 202 compared byte-exact (max delta 0). 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` compared byte-exact in-run at 72.072 s, inside the 120 s timeout.
Report sha256 `4e99dd64d154b557eb5439c27854a7f8b07dedc402ca894b9ca9da244ed8d94a`. Run log sha256 `6f0d746e9c6c6cda2d1c60dccc4795300f0679ce4ea9b38f4bee3099681da786`. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.
Exact-source CI: the newest `ci` and `Export kit` runs are green at head `b6c24628f6b8082f07a8a0a1a0110661325d10ef` (2026-09-26T16:45:48Z). That push carried the last `src/` change `1eb130d`. The runtime diff `b6c2462..347e298` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty, so those runs bind the current runtime and kit content. The docs-only heads after it have 0 check runs and match no workflow path filter.
Served kit `0.1.21` (source `b6c2462`): 33 of 35 kit files byte-checked against the repository through the `export-kit/kit.config.json` mappings. `LICENSES/noisemaker-MIT.txt` byte-matches the noisemaker reference `LICENSE` at `e73a44a`. `compat.json` declares exactly the 205 catalog IDs. Raw bytes were compared, not summaries.
The 2026-09-26 declared publication checks are carried, not re-executed. The runtime tree is unchanged: the diff `7f55619..347e298` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock` is empty. The rebuilt binary is byte-identical. Carried items: fmt, clippy, check, release tests on 1.85.0 and stable, package.

### Daily review, 2026-09-25

The current generated-bundle gate passes. Exact-source Cargo CI succeeds, but retains two ignored documentation tests. This does not establish current full rendered parity or the installed CLI on every declared platform. GAP-001 remains open. The 205-ID served declaration leaves five current IDs absent. [Raw evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/rust-ci-36082527571.log).
The review examined source changes, worker evidence, source-bound CI where present, and current served inventories. Full installed-host and platform qualification remains incomplete.
Environment: macOS 26.5, Darwin arm64.
[Source SHA-256 records](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/noisemaker-for-rust-source-hashes.json) bind these checks to the reviewed revision.
[Raw command evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/rust-tests.json). [Remote evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/noisemaker-for-rust-remote.json).

Executed command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check
```

The generated-bundle gate exited 0. This is a reproducibility examination, not a render or Cargo package installation test. Final exit code: 0.
No image denominator or tolerance follows from a unit-test or generated-file result.
Official reference: [Current Cargo Book, accessed 2026-09-24](https://doc.rust-lang.org/cargo/reference/publishing.html).

| Outcome | Observed scope | Remaining work |
|---|---|---|
| Installation | Instructions and metadata inspected | Install the actual artifact privately. |
| First useful output | Selected examinations only | Install the crate into a private root. Render a gradient and DSL chain, apply PNG input, test invalid output, and remove the installation. |
| Host integration | Not fully exercised | Examine parameters, external inputs, state, resize, and cleanup. |
| Errors and recovery | Only the selected items above | Fail through the installed entry point, correct input, and render again. |
| Distribution | Metadata inspection | Qualify distribution contents and lifecycle. |
| Accessibility | Not observed | Examine keyboard, focus, labels, and diagnostics for provided interfaces. |

Headless libraries do not require an editor accessibility test. Their CLI diagnostics and failure handling still require examination.
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

Raw logs, PNG outputs, hashes, parity reports, host/licensing/input identification, and the unavailable-platform list are retained in-repo at [installed workflow](parity/installed-workflow-20260926/).

## 4. Known gaps

P1 means false completion or major correctness failure. P2 means coverage or integration uncertainty. P3 means documentation inconsistency.
These entries record missing qualification. They do not infer implementation defects from absent tests.

### GAP-001: current authority and parity qualification

- Status: closed. Priority: P2. Category: verification.
- Affected scope: Cargo.toml, src/generated/, scripts/parity.py, scripts/generate_bundle.py, README.md
- Expected behavior: Reproducible evidence binds each supported claim to the port and authority revisions.
- Observed behavior: Five upstream effects remain intentionally excluded. Generated-file consistency does not establish pixel parity. It does not exercise the special semantic adapters.
- Evidence:
  - 2026-09-25 rendered parity run at `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` against the pinned JavaScript CPU oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e`. CPU: identified AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Denominator: 205 catalog records. Result: 202 compared, all byte-exact at tolerance 0. 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`). 0 errors.
  - The default 30-second timeout first reported `filter3d/flow3d` as a timeout failure. The retained 120-second retry compares byte-exact (rust 66.841 s).
  - Parameters and per-record metrics: [report](parity/parity-report-20260925.json), [raw output](parity/parity-run-20260925.log), [flow3d retry](parity/parity-flow3d-120s-retry.json), [source identity](parity/source-identity.json).
  - Re-measured 2026-09-27 at the current runtime tree (source `347e298`) against the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`). Result: 202 compared byte-exact at tolerance 0. 3 unsupported with the same reason. 0 errors. The upstream shader-language changes over `6a0af04d..7443f6e6` did not change any compared render (section 3, 2026-09-27 block).
  - Re-measured again 2026-09-27 at the newest delivery `dedfd07` (upstream pin `12b4d74f`). Result: 202 compared byte-exact at tolerance 0. 3 unsupported. 0 errors. Raw artifacts committed at `84b8168` (section 3, `dedfd07` block).
  - Independent review 2026-09-27: re-executed the same 205-record run with a fresh `dedfd07` oracle clone. Result: 202 byte-exact, worst max delta 0, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact at 66.916 s. The rebuilt binary stayed byte-identical (section 3, review block).
  - The five upstream effects excluded from this standalone CPU port remain excluded and reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Next action: None for the rendered parity denominator. Parameter sweeps and stateful sequences remain separate open qualification. The installed CLI (GAP-002, closed 2026-09-26) and platform matrices (GAP-003) are tracked in their own entries.
- Dependencies: Resolved. Immutable authority inputs are pinned by commit and tree hash in `docs/parity/source-identity.json`. Historical goldens and provenance are unchanged, and no golden was regenerated.
- Acceptance criteria: Met. Every applicable case, parameter choice, exclusion, error, and tolerance is reported. The denominator stays at 205.
- Required checks: Existing compiler gates unchanged. The rendered parity evidence is retained in-repo with raw output, exact source hashes, and the identified CPU revision. A CI-declared parity gate is not part of this candidate (workflow changes are out of scope for this job).
- Last verification: 2026-09-27 (worker run at source `863f64a` and independent review re-execution, both against the noisemaker-for-cpu `dedfd07` oracle). Result: 202 byte-exact, 3 unsupported, 0 errors.

### GAP-002: installed developer workflow qualification

- Status: closed. Priority: P2. Category: usability.
- Affected scope: Public API, examples, supported hosts, errors, recovery, and lifecycle.
- Expected behavior: Developers can install, produce useful output, integrate it, recover from errors, and remove the package.
- Observed behavior: The complete installed workflow was exercised in a private root on 2026-09-26 at git `81953750ac7268ed6b46497ee3e3ea620ebb41c1`. This is the audited base `31fb9dc` tree. Hosts: Rust 1.85.0 (MSRV) and stable 1.98.1.
  - `cargo check --all-targets` and the full release test suite passed on both toolchains.
  - `cargo package --locked` ran in an isolated copy. `cargo install` of the packaged crate into an isolated private root succeeded on both toolchains. The binary reports `noisemaker-rs 0.1.0`.
  - The installed CLI printed 205 `effects` records. A truncated `effects | head -3` reader hits a broken-pipe panic and exit 101. This is recorded as a limitation below.
  - The installed CLI rendered a gradient PNG, a chained DSL program PNG, and a `filter/blur` render of a PNG input.
  - Invalid values (`colorCount=99`, unknown `bogus`) failed with actionable diagnostics (exit 1). A corrected run recovered (exit 0).
  - A read-only output directory failed with `Permission denied` (exit 1) while preserving the pre-existing destination.
  - A SIGTERM mid-render preserved the pre-existing destination and left no temporary files.
  - Uninstall removed the binary cleanly (exit 0, empty private bin).
  - Rendered parity against the pinned upstream oracle noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` was re-measured on this run with the stable installed binary. Result: 202/205 compared byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors. The focused 1.85.0 comparison was 2/2 byte-exact with 0 errors.
- Evidence: retained in-repo at [installed workflow](parity/installed-workflow-20260926/). Contents: steps, commands, exit codes, artifact and output sha256s, raw logs, PNG outputs, and oracle parity reports. Also host identity, licensing and input-requirement identification, unavailable-platform list, and limitations. Hash records: [sha256sums](parity/installed-workflow-20260926/sha256sums.txt) and [external artifact hashes](parity/installed-workflow-20260926/sha256sums-external.txt). Also [README](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md), [official reference](https://doc.rust-lang.org/cargo/reference/publishing.html), and section 3.
- Next action:
  - None for the installed workflow.
  - Noted limitation: host coverage was single-platform (Linux x86_64, identified AMD EPYC 7713). CPU rendering only — no GPU path exists. macOS and Windows were not exercised and remain explicitly unavailable on this run (GAP-003). `animate`/ffmpeg guidance was not exercised again separately.
  - Minor usability wart observed and retained. `noisemaker-rs effects` panics with a broken-pipe error and exits 101 when its stdout is truncated early. Context: `effects | head -3`, library/std/src/io/stdio.rs broken pipe, os error 32. It does not exit cleanly. A full untruncated read (`effects | wc -l`) prints all 205 records. This is recorded as an observed limitation, not corrected in this register. Implementation belongs to the separate job.
- Dependencies: Resolved. An isolated consumer was used (private `CARGO_HOME`/install root, no global changes).
  - Licensing: the package is MIT ([LICENSE](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/LICENSE)). The packaged crate carries the MIT license file. Packaging listing retained at [package.log](parity/installed-workflow-20260926/package.log).
  - Input requirements: the CLI takes catalog effect IDs, DSL programs, and optional PNG inputs via `--input`/`--texture`. External-texture effects require a PNG binding. Both the generator and PNG-input paths were exercised.
  - Host identity: [host.txt](parity/installed-workflow-20260926/host.txt) (AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic). GPU: none — CPU-only rendering by design (README.md:14-19).
- Acceptance criteria: Met. Artifact hashes (crate, installed binaries, PNG outputs), steps, meaningful output, error diagnostics, recovery results, and cleanup results are retained in-repo. See the evidence above.
- Required checks: Met. Minimum (1.85.0) and current (stable 1.98.1) supported versions were both examined and installed from the same packaged crate. Both toolchains ran check and the full release test suite. Cancellation and file preservation were examined (SIGTERM preservation plus read-only destination preservation). Unavailable platforms are kept explicit (macOS/Windows untested, no GPU path).
- Last verification: 2026-09-26 (executed run at `81953750ac7268ed6b46497ee3e3ea620ebb41c1`, evidence retained in-repo). Carried, not re-executed, on 2026-09-27. The runtime diff `7f55619..347e298` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock` is empty. The committed evidence files are intact at the audited source. The measured workflow still binds the current runtime tree.
- Review 2026-09-27: the evidence manifest hash-matches 37 of 37 files. The byte-identical rebuilt binary reproduced the broken-pipe exit 101, the invalid-parameter exit 1 with its diagnostic, and the recovery exit 0 (section 3, review block).

### GAP-003: distribution and release qualification

- Status: closed (2026-09-26). Priority: P2. Category: release.
- Affected scope: Actual artifact, dependencies, notices, version promises, and release evidence.
- Expected behavior: The delivered artifact supports its documented installation and first useful result.
- Observed behavior: `cargo package --locked --no-verify` was executed in a fresh isolated copy (`git archive HEAD`) of the audited source tree `ff250e8e91e7003a918c7943146fc64d0fb123bc` (code identical to CI source `31fb9dc168a2f50aac99b6c379950767b06c4c62`).
  - PACKAGE-EXIT 0, "Packaged 52 files, 34.3MiB" ([package.log](parity/distribution-20260926/package.log)).
  - The crate sha256 (`db74b57f…3171a`) reproduced identically from two independent package runs ([crate-sha256.txt](parity/distribution-20260926/crate-sha256.txt)).
  - The extracted archive's 52-file inventory was byte-compared 52/52 between the two runs ([inventory-check.log](parity/distribution-20260926/inventory-check.log), per-file hashes in [archive hashes](parity/distribution-20260926/archive-files-sha256.txt)).
  - Notices LICENSE, README.md, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, and TRADEMARK.md are all present in the archive. The dependency tree is retained ([dep-tree.txt](parity/distribution-20260926/dep-tree.txt), [cargo-metadata.json](parity/distribution-20260926/cargo-metadata.json)).
  - Private installs from the unpacked, byte-checked archive ran on Rust 1.85.0 and stable 1.98.1. They used a private `CARGO_HOME` and install root with no global changes ([steps-1.85.0.log](parity/distribution-20260926/steps-1.85.0.log) / [steps-stable.log](parity/distribution-20260926/steps-stable.log)). Version printed. 205-record effects catalog read ([effects files](parity/distribution-20260926/sha256sums.txt)). Both packaged examples built and ran to PNGs. A gradient rendered from the installed binary. A force-reinstall upgrade over the existing root re-rendered byte-identically. Uninstall and reinstall both succeeded (all exit codes 0, post-upgrade render identical).
  - Known limitation retained with direct evidence: `cargo install <file>.crate` is not supported by cargo. A real invocation of `cargo install --locked --root <R> <file>.crate` exits 101 on both toolchains with no binary installed. 1.85.0 resolves the argument as a registry package spec. Stable rejects it as an invalid package name ([install-from-crate.log](parity/distribution-20260926/install-from-crate.log)). Installation used the unpacked archive instead (the accepted path in [steps-1.85.0.log](parity/distribution-20260926/steps-1.85.0.log) / [steps-stable.log](parity/distribution-20260926/steps-stable.log)).
- Evidence: retained in-repo at [distribution-20260926](parity/distribution-20260926/). Contents: packaging log, crate sha256, per-file inventory and byte comparison, notices, dependency inventory, install/upgrade/removal logs with exit codes, and example and installed-binary PNG outputs. Also host and toolchain identity ([host.txt](parity/distribution-20260926/host.txt), [toolchains.txt](parity/distribution-20260926/toolchains.txt)) and exact-source CI job records.
  - Exact-source CI: the `quality` and `test-and-package` jobs are green at source revision `31fb9dc168a2f50aac99b6c379950767b06c4c62` with step-level inspection. 0 skipped steps and 0 failed steps ([jobs-36208792712.json](parity/distribution-20260926/jobs-36208792712.json)). The export-kit dispatch job is green ([jobs-36208792819.json](parity/distribution-20260926/jobs-36208792819.json)).
  - Every commit from `31fb9dc` through this candidate touches only `docs/**`. `ci.yml` path filters exclude docs-only revisions, so no check runs are expected at the docs-only candidate ([check-runs.json](parity/distribution-20260926/check-runs.json), 0 runs).
  - Actual render legs: the 205-record rendered-parity denominator binds to the stable binary ([parity-run-20260926.log](parity/parity-run-20260926.log)), with the focused 1.85.0 leg at [parity-1.85.log](parity/installed-workflow-20260926/parity-1.85.log). The 1.85.0 gap is recorded as a remaining limit, not a pass.
- Next action: none for distribution qualification. Remaining limits: the macOS/Windows platform matrix, parameter sweeps beyond harness defaults, and broader 1.85.0 artifact-parity coverage remain unqualified. The broken-pipe exit-101 limitation of `noisemaker-rs effects` under truncated stdout is retained uncorrected ([effects-stable.log](parity/installed-workflow-20260926/effects-stable.log)).
- Dependencies: Resolved. The GAP-002 dependency was resolved 2026-09-26. Source CI is distinguished from downstream publication and native rendering. Source CI was inspected at job and step level. crates.io publication is downstream and out of scope.
- Acceptance criteria: Met. Artifact bytes matched to their per-file inventory (52/52 across two independent package runs with an identical crate sha256). Notices and dependencies examined. Installation, packaged examples, upgrade, and removal all passed on Rust 1.85.0 and stable 1.98.1.
- Required checks: Met. Exact-source CI jobs were inspected at step level (0 skips, 0 failures). The actual render legs were counted from the retained 205-record parity run rather than trusting green summaries.
- Last verification: 2026-09-26 (executed run, evidence retained in-repo). This register does not approve a release. Carried, not re-executed, on 2026-09-27. The runtime and `export-kit/` trees are unchanged from `b6c2462` through `347e298`. The committed evidence files are intact. The served kit `0.1.21` was byte-checked against the repository on 2026-09-27 (section 3).
- Review 2026-09-27: the evidence manifest hash-matches 26 of 27 entries. The failed entry is the manifest's own self-listing, not an evidence file. The served kit spot-check matched 4 of 4 sampled files (section 3, review block).

## 5. Ordered next actions

Current first action: This review re-executed the 205-record rendered parity run at the newest CPU delivery (section 3). No implementation is authorized by this review. Remaining qualification limits: the macOS/Windows platform matrix, parameter sweeps beyond harness defaults, and broader 1.85.0 artifact-parity coverage. Also open: the two ignored oracle-parity tests in CI and the retained broken-pipe limitation.
The parameter-contract provenance pin stays at `6a0af04d` while the CPU authority pins `12b4d74f`. The snapshot effect data is byte-identical across that range, so the provenance bump belongs to the separate sync job.
When noisemaker-for-cpu syncs past `dedfd07`, re-run the 205-record parity run at the new oracle. Require 202 byte-exact comparisons, 3 explained unsupported cases, and 0 errors at tolerance 0.
Subsequent historical actions remain dependent on that evidence.

1. Resolve authority identities for GAP-001. Retain earlier denominators, goldens, tolerances, and exclusions.
2. Execute the installed workflow for GAP-002. Record meaningful output, failure recovery, versions, and cleanup. (Done 2026-09-26. See section 3 and [installed workflow](parity/installed-workflow-20260926/).)
3. Run compiler and rendered parity for GAP-001. Keep structural, numerical, and platform evidence separate.
4. Qualify distribution contents and lifecycle for GAP-003 after the installed workflow passes. (Done 2026-09-26. See section 3 and [distribution-20260926](parity/distribution-20260926/).)
5. Record measured results. Close entries only when their acceptance criteria pass.

Implementation belongs to the separate job. Do not port additional effects or advance the current parity checkpoint through this register.

## 6. Pass history

2026-09-27 daily review at `347e298348fbf61993f537f2a7689e798674d7e4`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`). Served kit `0.1.21` byte-checked. No gap opened, closed, or reopened. GAP-002 and GAP-003 evidence carried across the unchanged runtime tree. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.
This pass also rewrote both registers to the strict style rules. Every fact, hash, identifier, link, hedge, and qualifier is preserved. Two compatibility matrix statuses were moved into the controlled vocabulary.
Independent review 2026-09-27 at `84b8168a2337ef846a6846d09c4cf32e0b481928`. This review covered worker run `audit-20260927-093000`, the `dedfd07` follow-up commit, and the range after `087e23a`. All three closure claims re-verified. Both ignored CI tests identified and passing locally. Upstream discovery refreshed to `9322993` (`1.0.194`). No gap opened, closed, or reopened. Evidence record: `/series/review-20260927-133500/result.json`.

| Date | Source SHA | Changes | Tested scope | Remaining limits |
|---|---|---|---|---|
| 2026-09-27 (review) | `84b8168a2337ef846a6846d09c4cf32e0b481928` (docs-only above `863f64a`) | Independent review. Re-executed the 205-record parity run at a fresh `dedfd07` oracle clone. Result: 202 byte-exact at tolerance 0, 3 unsupported, 0 errors, worst max delta 0. `filter3d/flow3d` byte-exact at 66.916 s. Rebuilt binary byte-identical (`d182360e…`). Both ignored CI tests (`tests/parity_spine.rs`) pass with `NOISEMAKER_JS_CPU_DIR` set. GAP-002 evidence hash-verified 37 of 37 files. GAP-003 evidence hash-verified 26 of 27 entries (self-listing defect only). Served kit spot-check matched 4 of 4 sampled files. Upstream discovery refreshed to `9322993` (`1.0.194`), manifests byte-identical. Workflow (CI) changes were not made. | Review of the worker audit and all three closure claims (section 3, review block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, and broader 1.85.0 artifact-parity coverage remain unqualified. The two ignored CI tests and the broken-pipe exit-101 limitation remain uncorrected. |
| 2026-09-27 | `347e298348fbf61993f537f2a7689e798674d7e4` | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `7a824744` oracle (upstream pin `7443f6e6`). The audited pin range `6a0af04d..7443f6e6` changed language sources, not the effect catalog. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 72.072 s. Rebuilt binary byte-identical to the prior audited binary. Served kit `0.1.21` (source `b6c2462`) byte-checked: 33/35 files direct, upstream MIT notice, compat ID set 205/205. Exact-source CI green at `b6c2462` head. Runtime tree identical to `347e298`. GAP-002/GAP-003 evidence carried with the empty runtime diff cited. Registers rewritten to the strict style rules with no factual change. Workflow (CI) changes were not made. | Parity, bundle gate, 19 script tests, kit bytes, CI mapping, upstream and CPU range classification (section 3, 2026-09-27 block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, ignored doctests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-26 (GAP-003) | `ff250e8e91e7003a918c7943146fc64d0fb123bc` (code identical to CI source `31fb9dc`) | GAP-003 closed. `cargo package --locked --no-verify` ran in a fresh isolated copy of the audited tree. Crate sha256 `db74b57f…3171a` reproduced identically across two independent package runs. 52-file archive inventory byte-compared 52/52 between runs. Notices (LICENSE, README.md, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, TRADEMARK.md) and the dependency tree retained. Private installs from the byte-checked archive ran on Rust 1.85.0 and stable 1.98.1. Observed: version output, the 205-record effects catalog, and both packaged examples rendered to PNGs. Also an installed-binary gradient render, a byte-identical force-reinstall upgrade, and uninstall/reinstall (all exit codes 0). Exact-source CI inspected at step level: 0 skipped steps, 0 failed steps at `31fb9dc`. Raw artifacts in `docs/parity/distribution-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1). Artifact bytes matched to inventory. Notices and dependencies examined. Packaged examples, upgrade, and removal exercised. | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, and the broken-pipe exit-101 limitation under truncated stdout remain unqualified or uncorrected. |
| 2026-09-26 | `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (source tree of audited base `31fb9dc`) | GAP-002 closed. Executed the complete installed workflow in a private root on Rust 1.85.0 and stable 1.98.1. Steps: version-matrix check/test, `cargo package --locked` in an isolated copy, and private install of the packaged crate. Observed: meaningful output (205 effects records, gradient, DSL chain, PNG-input apply), invalid-value diagnostics plus recovery, read-only-output preservation, SIGTERM cancellation plus preservation, and clean uninstall. Oracle parity with the installed stable binary: 202/205 byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors. Focused 1.85.0 comparison 2/2 byte-exact. Raw logs, PNGs, hashes, and reports in `docs/parity/installed-workflow-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1). Cancellation and file preservation examined. Unavailable platforms explicit (macOS/Windows untested. CPU-only, no GPU path). | Distribution lifecycle and artifact bytes (GAP-003), parameter sweeps beyond harness defaults, and the macOS/Windows platform matrix remain unqualified. |
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | GAP-001 closed. Executed the full 205-record rendered parity run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle on an identified CPU (AMD EPYC 7713, x86_64). | 202/205 compared byte-exact at tolerance 0. 3 unsupported with the stable overlay-interface reason. 0 errors. Default 30 s timeout failure on `filter3d/flow3d` retained. The 120 s retry compares byte-exact. Raw report, log, retry, and source hashes in `docs/parity/`. Workflow changes were not made. | Installation (GAP-002), distribution (GAP-003), parameter sweeps beyond harness defaults, and platform matrices remain unqualified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Created six-section register and README link. No closures. | The generated-bundle gate exited 0. This is a reproducibility examination, not a render or Cargo package installation test. | Full audit, installed workflows, current rendered parity, platforms, and releases remain unqualified. |

Run ID: `20260924-remaining-gap-documents`.
[Operational evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents). Creating this register does not advance successful-audit timestamps or the rotation.
