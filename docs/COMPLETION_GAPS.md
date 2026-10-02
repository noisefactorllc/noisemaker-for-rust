# noisemaker-for-rust: completion gaps

Current compatibility matrix: [compatibility report](COMPATIBILITY.md).

## 1. Scope and source revisions

Daily review: 2026-10-02, ninth pass, external-input import delivery `b0e6c41`. Current inspected source: the record-carrying candidate commit of this sync (its runtime tree carries the b93980b..b0e6c41 import; see [source identity](parity/source-identity.json) (`b0e6c41_range_run`)).
Full rendered parity at this SHA: measured 2026-10-02 (section 3, 2026-10-02 block): 210 compared byte-exact at tolerance 0, 0 unsupported, 0 errors, at the newest CPU delivery end `b0e6c41` with the five external-input effects imported (denominator moves 205 to 210). Installed developer workflow and distribution qualification: measured 2026-09-26 on Linux x86_64 (GAP-002, GAP-003). The earlier 205-record results (2026-09-27 through 2026-10-01) remain bound to their dated blocks. The macOS and Windows platform matrix remains unmeasured. No release approval follows from this review.
Daily review: 2026-10-01, eighth pass. Current inspected source: [`0f8d01cb7532b35e4cdf9b5cc189c30887e9f355`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/0f8d01cb7532b35e4cdf9b5cc189c30887e9f355) — the published head this review executed. Every commit after `7f55619` is docs-only; the runtime tree is unchanged.
Full rendered parity at this SHA: measured 2026-09-27/28 and re-executed by this review (section 3), including the 205-record run at the newest CPU delivery end `d6664f8`. Installed developer workflow and distribution qualification: measured 2026-09-26 on Linux x86_64 (GAP-002, GAP-003). Both results are carried across the docs-only revisions after `7f55619` because the runtime tree is unchanged. The macOS and Windows platform matrix remains unmeasured. No release approval follows from this review.
Current upstream discovery (review, 2026-09-27T13:45Z): `93229933b102ba82e713402be19db57207698850`, release `1.0.194` (11:35Z). The audit's 11:05Z observation recorded `1.0.192`. Release `1.0.193` was already published at 10:00Z. Release `1.0.193` is the CPU-pinned revision `12b4d74f`. The `1.0.193` and `1.0.194` manifests are byte-identical to the recorded 1.0.176 manifest. The effect catalog is unchanged. Upstream commits above the CPU pin remain unqualified. They belong to the CPU sync queue. Bounded refresh 2026-09-27T21:23Z: `git ls-remote` records upstream main at `04e8582c1db495f5a140d59a3be78e266775b702`, beyond the CPU pin `296e0138c4744ed485b2e95de3eeb466c17629ee`; those upstream commits remain unqualified and belong to the CPU sync queue.
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

Review CI boundary: Exact-source runs: Export kit, ci. A passing export dispatch does not qualify rendered parity. Current complete-render enforcement remains an open requirement. Exact-source responses (audit evidence `review-20260925-053200/noisemaker-for-rust-remote-evidence.json`).

### Daily review, 2026-10-02, GAP-003 fourth-leg delivery `b0e6c41` (external-input import)

This review audited the noisemaker-for-cpu delivery `b93980b512a24ef71ab61a959ed2ce35c1f09666..b0e6c4130ac2815145695114a475132282b266a9` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/b93980b512a2...b0e6c4130ac2)) at the record-carrying candidate of this sync.
Newest noisemaker-for-cpu delivery: `b0e6c4130ac2815145695114a475132282b266a9` (GAP-003 fourth leg: the five reactive/mesh effects are imported into the CPU catalog with CPU external-input support; no upstream pin change). No force-push flag; the range start is the previously published end (`git merge-base --is-ancestor` exits 0). Reviewable source evidence: the CPU-repository git bundle `noisemaker-for-cpu-b0e6c41.bundle` (sha256 `0f401c72bbbf33951f712f2c1a10b4a5585a68ff7c36c538a1fbdabed9275d7d`) is archived with the job in `/workspace/evidence`; `git clone -b main <bundle>` reproduces the complete noisemaker-for-cpu main history through `b0e6c41` (`git bundle verify` reports a complete history).
The leg is two commits (subjects emitted from `git log --format='%H %s'`): `796ec92f7ab82e5f8f289f63f52345ef71fcdc19` (`GAP-003: import the reactive and mesh effect trees; CPU external-input support`) and `b0e6c4130ac2815145695114a475132282b266a9` (`GAP-003: regenerate provenance introducedIn for the 5 reactive/mesh goldens against the published introducing commit 796ec92`). Diffstat: 32 files, 1921 insertions(+), 170 deletions(-).
Output-neutrality audit: the pinned-source manifest is byte-identical across the range (sha256 `9950d16a…` at both ends, recomputed locally); the generated snapshot adds exactly the five reactive/mesh effect trees (450 lines, sha256 `0c7119e9…` at `b93980b` -> `64500fb2…` at `b0e6c41`) with the `UPSTREAM_REVISION` line unchanged. The executable change is the external-input import (`src/runtime/external-input.js`, `src/runtime/external-textures.js`, `src/effects/cpu/mesh-render.js`, `renderer.js` externalInputs binding, `bin/noisemaker-cpu.js` random-pool exclusion), which the Rust port mirrors. Per-leg details: [source identity](parity/source-identity.json) (`b0e6c41_leg_audit`).
Port change (this sync, one candidate commit): the five formerly excluded effects are imported. Generated catalog/shaders/bundle-lock and the parameter contract (`6a0af04d…` -> `e24c844f…`) are regenerated through `scripts/generate_bundle.py` emission (`--check` exit 0); the runtime re-implements the rendering-relevant external-input state (`src/external_input.rs`: MidiState/AudioState/parseOBJ/packMeshDataForTextures ports), binds reactive uniforms and mesh/note-grid data textures (`src/renderer.rs`), runs `render/meshRender` through a native CPU triangle-mesh draw adapter (`src/draw_ops.rs`), exposes `--external-input {midi,audio,mesh}` CLI fixtures bound to the same deterministic constants as the CPU port's `scripts/parity/reactive-fixtures.js`, and the DSL allows external-input effects with no surface parameters to begin a chain. Tests: the catalog/CLI/package/scatter/dsl/smoke inventories move 205 -> 210 (296 programs, 8 draw-op keys), `scripts/parity.py` compares the five cases through host-fed fixtures (Rust CLI flags and an inline node CPU harness), and `scripts/tests/test_export_kit.py` pins the 210-record compat set.
GAP-001's measured state improves to 210/210 byte-exact at tolerance 0 (0 unsupported, 0 errors) at the new oracle end — re-measured below. Its status row in section 4 is updated to this measured state by its own entry; this Tearoff sync opens, closes, and reopens no gap. GAP-002/GAP-003 evidence is carried.
Checks re-executed at this tree: all CONTRIBUTING.md publication checks exit 0 (build, fmt, clippy -D warnings, `+1.85.0 check`, full release test suite with every result line `ok` including the 210-effect catalog smoke, unittest, `generate_bundle.py --check`, package) and the 210-record parity run at the `b0e6c41` oracle returns 210 compared byte-exact, 0 unsupported, 0 errors. Per-leg record: [source identity](parity/source-identity.json) (`b0e6c41_range_run`); verbatim command output and raw run artifacts are archived with the job in `/workspace/evidence` (checks-20261002-b0e6c41, parity-report-20261002-b0e6c41, parity-run-20261002-b0e6c41.log, cpu-range-review-20261002.txt) per the no-committed-logs policy.

### Daily review, 2026-10-01, GAP-003 first-leg delivery `d6664f8`

This review audited the noisemaker-for-cpu delivery `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e..d6664f8a494aec3bd10bce200d63d42ec833d61a` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/5f12866e919f...d6664f8a494a)) at the port head `0f8d01c`.
Newest noisemaker-for-cpu delivery: `d6664f8a494aec3bd10bce200d63d42ec833d61a` (GAP-003 first leg; no upstream pin change).
The delivery trigger flags a force-push/unknown-diff and cites the observed ranges `5f12866e919f..50c1cbe33191` and `50c1cbe33191..d6664f8a494a`; `fd9d56c74ce7`, `5f12866e919f`, and `50c1cbe33191` were verified ancestors of `d6664f8a494a` locally (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery — audited, not assumed. Reviewable source evidence: the CPU-repository git bundle `noisemaker-for-cpu-d6664f8.bundle` (sha256 `e0c5fd58ba2f6ea6b205661a674bc16d71ead7b83bdf8d66facac17703eaaa84`) is archived with the job in `/workspace/evidence`; `git clone -b main <bundle>` reproduces the complete noisemaker-for-cpu main history through `d6664f8a494a` (`git bundle verify` reports a complete history), and all three merge-base ancestry checks exit 0 from a fresh clone of that bundle.
The leg is two commits (subjects emitted from `git log --format='%H %s'`): `50c1cbe3319158be9d2965ebd4934b2852d96789` (`GAP-003 first leg: add the executable scripts/parity-summary whole-port parity summary entrypoint`) and `d6664f8a494aec3bd10bce200d63d42ec833d61a` (`GAP-003 first leg: publish the parity-summary record update on the verified entrypoint commit`). Diffstat: 6 files, 440 insertions(+), 50 deletions(-). Zero `src/` runtime files, zero `shaders/` or pinned-source-manifest entries, zero generated upstream-snapshot changes.
Output-neutrality audit: the generated upstream snapshot is byte-identical at both range ends (sha256 `0c7119e9…` at `5f12866`, recomputed locally at `d6664f8`, same sha256) — this leg changes no snapshot, no manifest, and no parameter-contract input; the parameter contract stays at `6a0af04d` (`generate_bundle.py --check` exit 0 at the unchanged contract). The range touches only CPU-repo parity-harness files (`scripts/parity-summary` entrypoint, `scripts/parity/lib.js` extracted from `run.js`, `scripts/parity/summary.js`, `test/parity-summary.test.js`) plus the CPU repo's own `docs/COMPLETION_GAPS.md` record update — files the Rust port neither consumes nor mirrors: the port drives the CPU oracle only through `bin/noisemaker-cpu.js` via `scripts/parity.py`, which is untouched. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`d6664f8_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Checks re-executed at the `0f8d01c` runtime tree (re-verified verbatim at the published head `9227d47`, docs-only over it): all CONTRIBUTING.md publication checks exit 0, and the 205-record parity run at the `d6664f8` oracle returns 202 compared byte-exact, 3 unsupported with the stable overlay-interface reason, 0 errors. Per-leg record: [source identity](parity/source-identity.json) (`d6664f8_range_run`); verbatim command output and raw run artifacts are archived with the job in `/workspace/evidence` (checks-20261001-final, checks-20261001, parity-report-20261001-d6664f8) per the no-committed-logs policy.

### Daily review, 2026-09-30, source-lock delivery `5f12866`

This review audited the noisemaker-for-cpu delivery `d2965d0b7880cee678de11ec797155c8a65c7b66..5f12866e919f76fc19e4d3e6f0670d61d10dc672` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/d2965d0b7880...5f12866e919f)) at the port head `b4a8402`.
Newest noisemaker-for-cpu delivery: `5f12866e919f76fc19e4d3e6f0670d61d10dc672` (upstream pin now `e24c844f8dada85551ab084f41db8944fbc176c8`).
The delivery also cites the observed range `ce3926d7a224..5f12866e919f` and flags a force-push/unknown-diff; all three range starts (`fd9d56c74ce7`, `d2965d0b7880`, `ce3926d7a224`) were verified ancestors of `5f12866e919f` locally (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery — audited, not assumed. Reviewable source evidence: the CPU-repository git bundle `noisemaker-for-cpu-5f12866.bundle` (sha256 `03492fa5277d92cc1bacb684802a68d25f5c67d740528aba81be4f93a9400b86`) is archived with the job in `/workspace/evidence`; `git clone -b main <bundle>` reproduces the complete noisemaker-for-cpu main history through `5f12866e919f` (`git bundle verify` reports a complete history), and all three merge-base ancestry checks exit 0 from a fresh clone of that bundle.
The leg is seven commits (subjects emitted from `git log --format='%H %s'`): `36872dd4cd9d` (CRT approximation/compatibility constraints record), `c58badadb8b0` (supervisor-publication blocker note), `52219d90f7e9` (portable-path rewording), `65a52cb04134`, `7e50900231df`, `ce3926d7a224` (GAP-002 external-input qualification records and required-checks tie), and `5f12866e919f76fc19e4d3e6f0670d61d10dc672` (`sync: advance the pinned upstream source-lock revision to e24c844f8dada85551ab084f41db8944fbc176c8`). Diffstat: 9 files, 67 insertions(+), 41 deletions(-). Zero `src/` runtime files other than the generated snapshot.
Output-neutrality audit: the snapshot diff is the `UPSTREAM_REVISION` line only (sha256 `50cb6c50…` at `d2965d0`, `0c7119e9…` at `5f12866`); the pinned-source manifest changes only the revision field plus four upstream GPU-runtime `shaders/src` entries (`shaders/src/runtime/backends/diagnostics.js` 7128->8059 bytes sha256 `2d3e07a8…`, `backends/webgl2.js` 78150->81111 sha256 `5f8c655c…`, `backends/webgpu.js` 164220->164648 sha256 `def2f469…`, `runtime/pipeline.js` 123038->124928 sha256 `4696cbea…`; noisemaker@e24c844f finalizes GAP-007 backend diagnostics and accepts the `input`/`resolution` dimension keywords in the validator) that the CPU-port engine does not execute; no `shaders/effects` entry changed; the parameter contract stays at `6a0af04d` (`generate_bundle.py --check` exit 0 at the unchanged contract). The six docs records are CPU-repo documents the Rust port neither consumes nor mirrors. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`5f12866_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Environment: Linux x86_64 (Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 (797e8a9bc 2026-08-05) / rustc 1.98.1 (48a229cea) via rustup 1.29.1 with relocated rustup/cargo homes under `/state/cache/rustup` and `/state/cache/cargo`, plus toolchain 1.85.0. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final, all run at the `b4a8402` tree):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01 (reproduced from a from-scratch build after `rm -rf target`, 38.99 s, the previously recorded d182360e… hash)
cargo fmt --all -- --check   # exit 0
cargo clippy --all-targets --all-features --locked -- -D warnings   # exit 0
cargo +1.85.0 check --all-targets --all-features --locked   # exit 0
cargo test --release --locked --all-features   # exit 0; every result line ok, 0 failed, 2 oracle-gated ignored
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; OK
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
cargo package --locked   # exit 0; packaged and verified
NOISEMAKER_JS_CPU_DIR=/state/cache/noisemaker-for-cpu cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js /state/cache/noisemaker-for-cpu/bin/noisemaker-cpu.js --timeout 120 --json /workspace/evidence/parity-report-20260930-5f12866.json   # exit 0
```

Parity result at the `5f12866` oracle: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 107.507 s; `filter/crt` byte-exact at 0.598 s.
Report sha256 `425b52ddd873371bcf126e436ee6ada6f82242adaaba562236574058e9563bdf`. Run log sha256 `8aaa36794356fdb1aa6c7f0ad3fab9be9a81323450fe0001cec22d63c37cdf7e`. Both raw artifacts and the CPU-repository git bundle are archived with the job in `/workspace/evidence` per the no-committed-logs policy; no new logs are committed under `docs/parity/`.
Exact-source CI: the audited head `b4a8402` is docs-only, so the audited tree triggers no `ci` path-filter run; the commit carrying this record touches docs only, so it likewise triggers no exact-source run. The audited tree itself changes no crate source. The crate declares no `[features]` section, so `--all-features` is a no-op flag.

### Daily review, 2026-09-29, source-lock delivery `d2965d0`

This review audited the noisemaker-for-cpu delivery `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e..d2965d0b7880cee678de11ec797155c8a65c7b66` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/fd9d56c74ce7...d2965d0b7880)) at the port head `c266f4b`.
Newest noisemaker-for-cpu delivery: `d2965d0b7880cee678de11ec797155c8a65c7b66` (upstream pin now `f24b52540af6a88d12daa05feba1a04ad61b22a2`).
The delivery also cites the observed range `7863f096061b..d2965d0b7880` and flags a force-push/unknown-diff; both range starts were verified ancestors of `d2965d0` locally (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery — audited, not assumed. Reviewable source evidence: the CPU-repository git bundle `noisemaker-for-cpu-d2965d0.bundle` (sha256 `565e08375f0d93c08af2520522661877529157340f9f682eec466248bd1ed310`) is archived with the job in `/workspace/evidence`; `git clone -b main <bundle>` reproduces the complete noisemaker-for-cpu main history through `d2965d0` (`git bundle verify` reports a complete history), and both merge-base ancestry checks exit 0 from a fresh clone of that bundle.
The leg is ten commits (subjects emitted from `git log --format='%H %s'`): `64539bcf`, `f9a36f2d`, `c87d5895`, `5e15ca86` (GAP-003/GAP-007 audit-review records), `6296476a`, `e0ff79cf`, `3c30781c`, `83fea7cb`, `7863f096` (GAP-001 docs records), and `d2965d0b7880cee678de11ec797155c8a65c7b66` (`sync: update upstream source lock and inventory through noisemaker@f24b5254`). Diffstat: 9 files, 134 insertions(+), 52 deletions(-). Zero `src/` runtime files other than the generated snapshot.
Output-neutrality audit: the snapshot diff is the `UPSTREAM_REVISION` line only (sha256 `f54a9e00…` at `fd9d56c`, `50cb6c50…` at `d2965d0`); the pinned-source manifest changes only the revision field plus one upstream WebAudio-runtime `shaders/src` entry (`shaders/src/runtime/external-input.js` 54120->67725 bytes, sha256 `c29887aa…`, the GAP-032 multi-capture `AudioInputManager` rework) that the CPU-port engine does not execute; no `shaders/effects` entry changed; the parameter contract stays at `6a0af04d` (`generate_bundle.py --check` exit 0 at the unchanged contract). The nine audit-review/doc records are CPU-repo documents the Rust port neither consumes nor mirrors. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`d2965d0_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Environment: Linux x86_64 (Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 (797e8a9bc 2026-08-05) / rustc 1.98.1 (48a229cea) via rustup 1.29.1 with relocated rustup/cargo homes under `/state/cache/rustup` and `/state/cache/cargo`, plus toolchain 1.85.0. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final, all run at the `c266f4b` tree):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01 (reproduced the previously recorded d182360e… hash)
cargo fmt --all -- --check   # exit 0
cargo clippy --all-targets --all-features -- -D warnings   # exit 0
cargo +1.85.0 check --all-targets --all-features   # exit 0
cargo test --release --locked --all-features   # exit 0; every result line ok, 0 failed, 2 oracle-gated ignored
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; OK
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
cargo package --locked   # exit 0; packaged and verified
NOISEMAKER_JS_CPU_DIR=/state/cache/checkouts/noisemaker-for-cpu cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js /state/cache/checkouts/noisemaker-for-cpu/bin/noisemaker-cpu.js --timeout 120 --json /workspace/evidence/parity-report-20260929-d2965d0.json   # exit 0
```

Parity result at the `d2965d0` oracle: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 66.376 s; `filter/crt` byte-exact at 0.413 s.
Report sha256 `bd49a22a9d93b0bba3fdaff05da02ee01bf482e80ee86aad8d5c133d1542dc7d`. Run log sha256 `8f789c834b551f57a1d800d3922a7fd131a5a35e8dbd0453390de2477a14b36a`. Both raw artifacts, the CPU-repository git bundle, the verbatim suite-check logs at revision `2454cf7`, and the inspectable bundle-ancestry/range-diff artifacts are archived, exactly these files, in `/workspace/evidence/evidence-1790709750021.tar.gz` (sha256 `98122317e549c0f27ad559393a0441481649e80e06dd8fdab668435560801197`; contents listed in `MANIFEST.txt` inside the archive) per the no-committed-logs policy; no new logs are committed under `docs/parity/`.
Exact-source CI: the audited head `c266f4b` is docs-only, so the audited tree triggers no `ci` path-filter run; the commit carrying this record touches docs only, so it likewise triggers no exact-source run. The audited tree itself changes no crate source. The crate declares no `[features]` section, so `--all-features` is a no-op flag.

### Daily review, 2026-09-28, source-lock delivery `fd9d56c`

This review audited the noisemaker-for-cpu delivery `f0ccebef830bf6b8d99eafadf73f260a6c224873..fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/f0ccebef830b...fd9d56c74ce7)) at the port head `b7fbfff`.
Newest noisemaker-for-cpu delivery: `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` (upstream pin now `73c15be00d6888f4b5d2835d8e242ee9e840df45`).
`f0ccebe` is an ancestor of `fd9d56c` (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery.
The leg is three commits (subjects emitted from `git log --format='%H %s'`): `21d211e0f3dcdf409b197fb5d212aac706fc75e0` (`test: qualify landscape on Metal and reopen native CRT tracing`), `f290040b497e0aa138a9cc1e3b8aa9cea47bd196` (`GAP-008: report golden reference provenance separately from the kernel pin`), and `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` (`sync: update upstream source lock and inventory through noisemaker@73c15be0`). Diffstat: 14 files, 1516 insertions(+), 27 deletions(-). Zero `src/` runtime files other than the generated snapshot.
Output-neutrality audit: the snapshot diff is the `UPSTREAM_REVISION` line only (sha256 `f78c661c…` at `f0ccebe`, `f54a9e00…` at `fd9d56c`); the pinned-source manifest changes only the revision field plus three upstream GPU-runtime `shaders/src` entries (webgl2.js 78025->78150, compiler.js 8595->8795, pipeline.js 116862->123038) that the CPU-port engine does not execute; no `shaders/effects` entry changed; the parameter contract stays at `6a0af04d` (`generate_bundle.py --check` exit 0 at the unchanged contract). GAP-008's golden-provenance tooling (`scripts/parity/run.js`, `write-provenance.js`, `parity/goldens/provenance.json`, its test) and the Metal landscape audit record are CPU-repo artifacts the Rust port neither consumes nor mirrors. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`fd9d56c_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Environment: Linux x86_64 (6 cores, Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 (797e8a9bc 2026-08-05) / rustc 1.98.1 (48a229cea) via rustup 1.29.1 with relocated rustup/cargo homes under `/state/cache/rustup` and `/state/cache/cargo`, plus toolchain 1.85.0. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final, all run at the `b7fbfff` tree):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01 (reproduced the previously recorded d182360e… hash)
cargo fmt --all -- --check   # exit 0
cargo clippy --all-targets --all-features -- -D warnings   # exit 0
cargo +1.85.0 check --all-targets --all-features   # exit 0
cargo test --release --locked --all-features   # exit 0; every result line ok, 0 failed, 2 oracle-gated ignored
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; OK
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
cargo package --locked   # exit 0; packaged and verified
NOISEMAKER_JS_CPU_DIR=/state/cache/platform/noisemaker-for-cpu cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js /state/cache/platform/noisemaker-for-cpu/bin/noisemaker-cpu.js --timeout 120 --json /state/cache/scratch/parity-report-fd9d56c.json   # exit 0
```

Parity result at the `fd9d56c` oracle: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 72.633 s; `filter/crt` byte-exact at 0.704 s.
Report sha256 `11d51a067ca40919dc736ed7d7a44dcc2bb9eb118b445d835876535cf0b1665e`. Run log sha256 `a4887fd389427088d2d20886413a4aaa4aec2db7b6b4527856a7d48d48a13b2f`. Raw artifacts are archived with the job in `/workspace/evidence` per the no-committed-logs policy; no new logs are committed under `docs/parity/`.
Exact-source CI: the audited head `b7fbfff` is docs-only, so the audited tree triggers no `ci` path-filter run; the commit carrying this record may additionally touch the repository's test entrypoint, which the workflow's non-docs path filter runs exact-source. The audited tree itself changes no crate source. The crate declares no `[features]` section, so `--all-features` is a no-op flag.

### Daily review, 2026-09-27, GAP-007 delivery `f0ccebe`

This review audited the noisemaker-for-cpu delivery `b61b658399f18b5a93abd0020c02fff3be9630f5..f0ccebef830bf6b8d99eafadf73f260a6c224873` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/b61b658399f1...f0ccebef830b)) at the port head `3ae887b`.
Newest noisemaker-for-cpu delivery: `f0ccebef830bf6b8d99eafadf73f260a6c224873` (upstream pin unchanged at `296e0138c4744ed485b2e95de3eeb466c17629ee`).
`b61b658` is an ancestor of `f0ccebe` (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery.
The leg is one commit (subject emitted from `git log --format='%H %s'`): `f0ccebef830bf6b8d99eafadf73f260a6c224873` (`GAP-007: give browser demo controls distinct accessible names`). Diffstat: docs/COMPLETION_GAPS 6, examples/browser/control-factory 37+, examples/browser/demo 12+, examples/browser/effect-select 11+, examples/browser/index.html 14, examples/browser/toggle-switch 5+, test/demo-pipeline.test 38+. Total 7 files, 112 insertions(+), 11 deletions(-). Zero `src/` and zero `scripts/` files (`git diff b61b658..f0ccebe -- src/ scripts/` is empty).
Output-neutrality audit: the snapshot (`f78c661c…` sha256 at both ends), the pinned-source manifest, and the source-lock constants are byte-identical across the range; the parameter contract stays at `6a0af04d`. The leg is a browser-demo accessibility change (control aria-labels plus unit tests); the Rust port mirrors no browser demo (its `examples/` contains only `render_dsl.rs` and `render_effect.rs`), so the port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`f0ccebe_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Environment: Linux x86_64 (6 cores, Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 (797e8a9bc 2026-08-05) via rustup 1.29.1 with relocated rustup/cargo homes under `/state/cache/rustup` and `/state/cache/cargo`, plus toolchain 1.85.0. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final, all run at the `3ae887b` tree):

```sh
cargo build --release --locked   # exit 0; binary sha256 d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01 (reproduced the previously recorded d182360e… hash)
cargo fmt --all -- --check   # exit 0
cargo clippy --all-targets --all-features -- -D warnings   # exit 0
cargo +1.85.0 check --all-targets --all-features   # exit 0
cargo test --release --locked --all-features   # exit 0; 206 passed, 0 failed, 2 oracle-gated ignored
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; OK
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
cargo package --locked   # exit 0; packaged and verified
NOISEMAKER_JS_CPU_DIR=/state/cache/noisemaker-for-cpu cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js /state/cache/noisemaker-for-cpu/bin/noisemaker-cpu.js --timeout 120 --json /state/cache/scratch/parity-report-f0ccebe.json   # exit 0
```

The rebuilt binary sha256 `d182360e…` reproduces the previously recorded hash from the pre-eviction build; the runtime tree is unchanged since `7f55619` (`git diff 7f55619..3ae887b` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty). The executed parity run is the authoritative contract evidence.

Parity result at the `f0ccebe` oracle: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 66.448 s; `filter/crt` byte-exact at 0.424 s.
Report sha256 `94487e8c5f594fe1768432e207f4ed4f453bda852e30045184a141eee9730489`. Run log sha256 `7bc0543303c15a4ffb5ddd0d4c358d52e3cc3b5f3f1c0d480acc5a79a6f07826`. Raw artifacts: `docs/parity/parity-report-20260927-f0ccebe.json`, `docs/parity/parity-run-20260927-f0ccebe.log`.
Exact-source CI: this audit head is docs-only, and the `ci` workflow path filter excludes `docs/**`, so no exact-source run is expected for it. The crate declares no `[features]` section, so `--all-features` is a no-op flag.

### Daily review, 2026-09-27, two-sync delivery `b61b658`

This review audited the noisemaker-for-cpu delivery `dedfd07c24f80d9b0adddf912a4224ce6c1d795f..b61b658399f18b5a93abd0020c02fff3be9630f5` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/dedfd07c24f8...b61b658399f1)) at the port head `4092859`.
Newest noisemaker-for-cpu delivery: `b61b658399f18b5a93abd0020c02fff3be9630f5` (upstream pin `296e0138c4744ed485b2e95de3eeb466c17629ee`, the upstream main at the sync commits).
`dedfd07` is an ancestor of `b61b658` (`git merge-base --is-ancestor` exits 0), so the previously audited legs plus this one cover the full declared delivery.
The leg is two commits, both CPU source-lock sync commits (subjects emitted from `git log --format='%H %s'`): `34a0325c5392e20c2520f39740042675dbcd5cc4` (`noisemaker@93229933`) and `b61b658399f18b5a93abd0020c02fff3be9630f5` (`noisemaker@296e0138`). Diffstat: README 2, docs/COMPATIBILITY 23+, docs/COMPLETION_GAPS 2+, docs/EFFECTS 2, two new CPU audit JSONs, three refreshed CPU evidence records and logs, pinned-source-manifest 2, source-lock.js 2, snapshot 2, upstream-inventory.test.js 2. Total 14 files, 630 insertions(+), 322 deletions(-). Zero `src/` runtime files other than the generated snapshot.
Snapshot identity: sha256 `2b7587bd…` at `dedfd07`, `f78c661c…` at `b61b658`. The entire snapshot diff is the revision line.
Output-neutrality audit: the pinned-source-manifest diff is the revision line only; the `PINNED_SOURCE_DIGEST`/`PINNED_SOURCE_MANIFEST_DIGEST` constants are byte-unchanged across both pin moves; and a fresh blobless clone of the upstream noisemaker repo records identical git tree objects for `shaders/effects` (`7ba7a60f…`) and `shaders/src` (`6b791b53…`) at both `12b4d74f` and `296e0138`. The upstream range's only `shaders/` commits (93229933, a912749f, 296e0138) touch `shaders/tests/` plus the upstream harness and test registrations — code the CPU port does not consume and no Rust equivalent mirrors. The port requires no source change, and the parameter contract stays at `6a0af04d`. Per-leg details and CPU-side audit artifact hashes: [source identity](parity/source-identity.json) (`b61b658_leg_audit`).
No gap opened, closed, or reopened. GAP-001's 202/205-byte-exact-with-3-unsupported state is unchanged and re-measured below at the new oracle end. GAP-002/GAP-003 evidence is carried (the runtime tree is unchanged).

Environment: Linux x86_64 (AMD EPYC 7713, Linux 6.8.0-134-generic). Toolchain cargo 1.98.1 via rustup 1.29.1 with relocated rustup/cargo homes under `/state/cache/scratch` (the earlier user-level install was evicted with the container home), plus toolchain 1.85.0. Node v26.5.1. Python 3.11.2. No GPU, macOS, or Windows in this container.

Executed commands (exit codes final, all run at the `4092859` tree):

```sh
cargo build --release --locked   # exit 0; binary sha256 4eb6288abc0d687ed3bd9b279ab1977ffb77ee37881c7b75b037ee4cf35e5586
cargo fmt --all -- --check   # exit 0
cargo clippy --all-targets --all-features -- -D warnings   # exit 0
cargo +1.85.0 check --all-targets --all-features   # exit 0 (cargo 1.85.0, d73d2caf9 2024-12-31; crate target re-verified after touching src/lib.rs)
cargo test --release --locked --all-features   # exit 0; 27 result lines all 'ok', 206 passed, 0 failed, 2 oracle-gated ignored; longest binary 569.19 s
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'   # exit 0; OK
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check   # exit 0
cargo package --locked   # exit 0; packaged and verified
NOISEMAKER_JS_CPU_DIR=<oracle at b61b658> cargo test --release --locked --test parity_spine -- --ignored   # exit 0; 2 passed, 0 failed
PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <full clone of noisemaker-for-cpu at b61b658>/bin/noisemaker-cpu.js --timeout 120 --json docs/parity/parity-report-20260927-b61b658.json   # exit 0
```

The rebuilt binary sha256 `4eb6288…` differs from the previously recorded `d182360e…` because the relocated rustup/cargo homes embed different absolute paths in the build; the runtime tree is unchanged since `7f55619` (`git diff 7f55619..4092859` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty). The executed parity run is the authoritative contract evidence.

Independent parity result at the `b61b658` oracle: 205 catalog records. 202 compared byte-exact, worst max delta 0. 3 unsupported with the stable overlay-interface reason. 0 errors. `filter3d/flow3d` byte-exact in-run at 71.159 s; `filter/crt` byte-exact at 0.382 s.
Report sha256 `b79e3ac2db3536c6e7af264b449a12090ca3e2e41352eb17ded15b75ea30da89`. Run log sha256 `63b21eb7c60d9c640a41245fca651f45d548ab47b20d162c57be26095e375b67`. Raw artifacts: `docs/parity/parity-report-20260927-b61b658.json`, `docs/parity/parity-run-20260927-b61b658.log`.
The 2026-09-26 declared publication checks are superseded by the re-execution above at this tree, which covers every declared check with exit 0. Exact-source CI: no workflow in the repository declares a path filter or an exact-source run that binds this docs-only head, so none was executed. The crate declares no `[features]` section, so `--all-features` is a no-op flag.

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

The current generated-bundle gate passes. Exact-source Cargo CI succeeds, but retains two ignored documentation tests. This does not establish current full rendered parity or the installed CLI on every declared platform. GAP-001 remains open. The 205-ID served declaration leaves five current IDs absent. Raw evidence (audit evidence `review-20260925-053200/rust-ci-36082527571.log`).
The review examined source changes, worker evidence, source-bound CI where present, and current served inventories. Full installed-host and platform qualification remains incomplete.
Environment: macOS 26.5, Darwin arm64.
Source SHA-256 records (audit evidence `evidence-20260924-remaining-gap-documents/noisemaker-for-rust-source-hashes.json`) bind these checks to the reviewed revision.
Raw command evidence (audit evidence `evidence-20260924-remaining-gap-documents/rust-tests.json`). Remote evidence (audit evidence `evidence-20260924-remaining-gap-documents/noisemaker-for-rust-remote.json`).

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
- Observed behavior: Five upstream effects were intentionally excluded until 2026-10-02, when the external-input import brought them into the catalog with host-fed external-input fixtures (denominator 205 to 210, section 3 2026-10-02 block). Generated-file consistency does not establish pixel parity on its own; the executed parity runs do.
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
- Acceptance criteria: Met at the recorded measurements. Every applicable case, parameter choice, exclusion, error, and tolerance is reported. The denominator tracks the imported catalog: 205 through the 2026-10-01 run, 210 from the 2026-10-02 external-input import onward.
- Required checks: Existing compiler gates unchanged. The rendered parity evidence is retained in-repo with raw output, exact source hashes, and the identified CPU revision. A CI-declared parity gate is not part of this candidate (workflow changes are out of scope for this job).
- Last verification: 2026-09-27 (worker run at source `863f64a` and independent review re-execution, both against the noisemaker-for-cpu `dedfd07` oracle). Result: 202 byte-exact, 3 unsupported, 0 errors.
- Re-measured 2026-10-02 at the newest delivery `b0e6c41` after the five external-input effects were imported (section 3, 2026-10-02 block). Result: 210 compared byte-exact at tolerance 0, 0 unsupported, 0 errors. The denominator moves 205 to 210; the three overlay cases were already compared in Ready mode at the 2026-10-01 `b93980b` run (205/205 byte-exact), so no unsupported classification remains.

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
The parameter-contract provenance pin stays at `6a0af04d` while the CPU authority pins `f24b5254`. The snapshot effect data is byte-identical across that range and the pinned `shaders/src` changes are upstream WebAudio-runtime entries the port does not execute, so the provenance bump belongs to the separate sync job.
When noisemaker-for-cpu syncs past `d2965d0`, re-run the 205-record parity run at the new oracle. Require 202 byte-exact comparisons, 3 explained unsupported cases, and 0 errors at tolerance 0.
Subsequent historical actions remain dependent on that evidence.

1. Resolve authority identities for GAP-001. Retain earlier denominators, goldens, tolerances, and exclusions.
2. Execute the installed workflow for GAP-002. Record meaningful output, failure recovery, versions, and cleanup. (Done 2026-09-26. See section 3 and [installed workflow](parity/installed-workflow-20260926/).)
3. Run compiler and rendered parity for GAP-001. Keep structural, numerical, and platform evidence separate.
4. Qualify distribution contents and lifecycle for GAP-003 after the installed workflow passes. (Done 2026-09-26. See section 3 and [distribution-20260926](parity/distribution-20260926/).)
5. Record measured results. Close entries only when their acceptance criteria pass.

Implementation belongs to the separate job. Do not port additional effects or advance the current parity checkpoint through this register.

## 6. Pass history

2026-09-30 sync review (seventh pass) at `b4a8402873a693664d8e8e33f2e73d00e8efc4a3`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `5f12866` (upstream pin now `e24c844f`; four pinned upstream GPU-runtime `shaders/src` entries changed, effect catalog unchanged) with the full publication-check set re-executed at this tree, including a from-scratch binary rebuild reproducing sha256 `d182360e…`. No gap opened, closed, or reopened. GAP-002/GAP-003 evidence carried across the unchanged runtime tree.
2026-09-29 sync review (sixth pass) at `c266f4b620bb3102476423ffb7c36750d175c860`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `d2965d0` (upstream pin now `f24b5254`; one pinned upstream WebAudio-runtime `shaders/src` entry changed, effect catalog unchanged) with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. GAP-002/GAP-003 evidence carried across the unchanged runtime tree.
2026-09-28 sync review (fifth pass) at `b7fbfff5417de70191261758a9a37a51cad382d5`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `fd9d56c` (upstream pin now `73c15be0`; pinned upstream GPU-runtime `shaders/src` entries changed, effect catalog unchanged) with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. GAP-002/GAP-003 evidence carried across the unchanged runtime tree.
2026-09-27 sync review (fourth pass) at `3ae887b774f00dd2a56c647a6459c942c19b8f62`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `f0ccebe` (upstream pin `296e0138` unchanged) with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. GAP-002/GAP-003 evidence carried across the unchanged runtime tree.
2026-09-27 daily review at `347e298348fbf61993f537f2a7689e798674d7e4`: rendered parity re-measured against the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`). Served kit `0.1.21` byte-checked. No gap opened, closed, or reopened. GAP-002 and GAP-003 evidence carried across the unchanged runtime tree. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.
This pass also rewrote both registers to the strict style rules. Every fact, hash, identifier, link, hedge, and qualifier is preserved. Two compatibility matrix statuses were moved into the controlled vocabulary.
Independent review 2026-09-27 at `84b8168a2337ef846a6846d09c4cf32e0b481928`. This review covered worker run `audit-20260927-093000`, the `dedfd07` follow-up commit, and the range after `087e23a`. All three closure claims re-verified. Both ignored CI tests identified and passing locally. Upstream discovery refreshed to `9322993` (`1.0.194`). No gap opened, closed, or reopened. Evidence record: `/series/review-20260927-133500/result.json`.

| Date | Source SHA | Changes | Tested scope | Remaining limits |
|---|---|---|---|---|
| 2026-09-30 (seventh pass, sync) | `b4a8402873a693664d8e8e33f2e73d00e8efc4a3` (docs-only above `c266f4b`) | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `5f12866` oracle (upstream pin now `e24c844f`). The audited range `d2965d0..5f12866` is seven CPU commits — six docs records (`36872dd4`, `c58badad`, `52219d90`, `65a52cb0`, `7e509002`, `ce3926d7`) and `5f12866` (`sync: advance the pinned upstream source-lock revision to e24c844f8dada85551ab084f41db8944fbc176c8`) — 9 files, 67+/41-; all three cited range starts (`fd9d56c`, `d2965d0`, `ce3926d7`) verified ancestors locally (force-push-flagged delivery, audited not assumed, re-verified from a fresh clone of the archived git bundle); zero `src/` runtime files other than the generated snapshot, whose diff is the revision line only (`50cb6c50…` → `0c7119e9…`). The pinned-source manifest changes only the revision field plus four upstream GPU-runtime `shaders/src` entries (`backends/diagnostics.js`, `backends/webgl2.js`, `backends/webgpu.js`, `runtime/pipeline.js`; GAP-007 backend-diagnostics final leg, validator `input`/`resolution` dimension keywords) the port does not execute; no `shaders/effects` entry changed and the parameter contract stays at `6a0af04d` — the port requires no source change. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 107.507 s. From-scratch binary rebuild (after `rm -rf target`) reproduced the previously recorded sha256 `d182360e…`. Every CONTRIBUTING.md publication check re-executed, all exit 0, including the oracle-gated `parity_spine` tests (2 passed). Raw run artifacts archived with the job in `/workspace/evidence` (report `425b52dd…`, log `8aaa3679…`); no logs committed. Workflow (CI) changes were not made. | Parity, bundle gate, script tests, publication checks, CPU range classification (section 3, `5f12866` block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, the two ignored CI tests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-29 (sixth pass, sync) | `c266f4b620bb3102476423ffb7c36750d175c860` (docs-only above `b7fbfff`) | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `d2965d0` oracle (upstream pin now `f24b5254`). The audited range `fd9d56c..d2965d0` is ten CPU commits — nine GAP-001/GAP-003/GAP-007 audit-review and docs records (`64539bcf`, `f9a36f2d`, `c87d5895`, `5e15ca86`, `6296476a`, `e0ff79cf`, `3c30781c`, `83fea7cb`, `7863f096`) and `d2965d0` (`sync: update upstream source lock and inventory through noisemaker@f24b5254`) — 9 files, 134+/52-; both cited range starts verified ancestors locally (force-push-flagged delivery, audited not assumed); zero `src/` runtime files other than the generated snapshot, whose diff is the revision line only (`f54a9e00…` → `50cb6c50…`). The pinned-source manifest changes only the revision field plus one upstream WebAudio-runtime `shaders/src` entry (`shaders/src/runtime/external-input.js`, GAP-032 `AudioInputManager` rework) the port does not execute; no `shaders/effects` entry changed and the parameter contract stays at `6a0af04d` — the port requires no source change. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 66.376 s. Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Every CONTRIBUTING.md publication check re-executed, all exit 0, including the oracle-gated `parity_spine` tests (2 passed). Raw run artifacts archived with the job in `/workspace/evidence` (report `bd49a22a…`, log `8f789c83…`); no logs committed. Workflow (CI) changes were not made. | Parity, bundle gate, script tests, publication checks, CPU range classification (section 3, `d2965d0` block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, the two ignored CI tests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-28 (fifth pass, sync) | `b7fbfff5417de70191261758a9a37a51cad382d5` (docs-only above `3ae887b`) | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `fd9d56c` oracle (upstream pin now `73c15be0`). The audited range `f0ccebe..fd9d56c` is three CPU commits: `21d211e0` (`test: qualify landscape on Metal and reopen native CRT tracing`), `f290040b` (`GAP-008: report golden reference provenance separately from the kernel pin`), and `fd9d56c` (`sync: update upstream source lock and inventory through noisemaker@73c15be0`), 14 files, 1516+/27-; zero `src/` runtime files other than the generated snapshot, whose diff is the revision line only (`f78c661c…` → `f54a9e00…`). The pinned-source manifest changes only the revision field plus three upstream GPU-runtime `shaders/src` entries (webgl2.js, compiler.js, pipeline.js) the port does not execute; no `shaders/effects` entry changed and the parameter contract stays at `6a0af04d` — the port requires no source change. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 72.633 s. Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Every CONTRIBUTING.md publication check re-executed, all exit 0, including the oracle-gated `parity_spine` tests (2 passed). Raw run artifacts archived with the job in `/workspace/evidence` (report `11d51a06…`, log `a4887fd3…`); no logs committed. Workflow (CI) changes were not made. | Parity, bundle gate, script tests, publication checks, CPU range classification (section 3, `fd9d56c` block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, the two ignored CI tests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-27 (fourth pass, sync) | `3ae887b774f00dd2a56c647a6459c942c19b8f62` (docs-only above `4092859`) | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `f0ccebe` oracle (upstream pin `296e0138` unchanged). The audited range `b61b658..f0ccebe` is one browser-demo accessibility commit (`f0ccebef830b`, GAP-007 aria-labels plus unit tests, 7 files, 112+/11-); `git diff b61b658..f0ccebe -- src/ scripts/` is empty, the snapshot/manifest/source-lock are byte-identical, and the parameter contract stays at `6a0af04d` — the port requires no source change. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 66.448 s. Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Every CONTRIBUTING.md publication check re-executed, all exit 0, including the oracle-gated `parity_spine` tests (2 passed) and the full release suite (206 passed, 0 failed, 2 oracle-gated ignored). Raw artifacts: `docs/parity/parity-report-20260927-f0ccebe.json` / `parity-run-20260927-f0ccebe.log`. Workflow (CI) changes were not made. | Parity, bundle gate, script tests, publication checks, CPU range classification (section 3, `f0ccebe` block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, the two ignored CI tests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-27 (review) | `84b8168a2337ef846a6846d09c4cf32e0b481928` (docs-only above `863f64a`) | Independent review. Re-executed the 205-record parity run at a fresh `dedfd07` oracle clone. Result: 202 byte-exact at tolerance 0, 3 unsupported, 0 errors, worst max delta 0. `filter3d/flow3d` byte-exact at 66.916 s. Rebuilt binary byte-identical (`d182360e…`). Both ignored CI tests (`tests/parity_spine.rs`) pass with `NOISEMAKER_JS_CPU_DIR` set. GAP-002 evidence hash-verified 37 of 37 files. GAP-003 evidence hash-verified 26 of 27 entries (self-listing defect only). Served kit spot-check matched 4 of 4 sampled files. Upstream discovery refreshed to `9322993` (`1.0.194`), manifests byte-identical. Workflow (CI) changes were not made. | Review of the worker audit and all three closure claims (section 3, review block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, and broader 1.85.0 artifact-parity coverage remain unqualified. The two ignored CI tests and the broken-pipe exit-101 limitation remain uncorrected. |
| 2026-09-27 | `347e298348fbf61993f537f2a7689e798674d7e4` | Rendered parity re-measured at the newest CPU delivery end. 205-record run against the noisemaker-for-cpu `7a824744` oracle (upstream pin `7443f6e6`). The audited pin range `6a0af04d..7443f6e6` changed language sources, not the effect catalog. 202 byte-exact at tolerance 0. 3 unsupported with reasons. 0 errors. `filter3d/flow3d` byte-exact in-run at 72.072 s. Rebuilt binary byte-identical to the prior audited binary. Served kit `0.1.21` (source `b6c2462`) byte-checked: 33/35 files direct, upstream MIT notice, compat ID set 205/205. Exact-source CI green at `b6c2462` head. Runtime tree identical to `347e298`. GAP-002/GAP-003 evidence carried with the empty runtime diff cited. Registers rewritten to the strict style rules with no factual change. Workflow (CI) changes were not made. | Parity, bundle gate, 19 script tests, kit bytes, CI mapping, upstream and CPU range classification (section 3, 2026-09-27 block). | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, ignored doctests, and the broken-pipe exit-101 limitation remain unqualified or uncorrected. |
| 2026-09-26 (GAP-003) | `ff250e8e91e7003a918c7943146fc64d0fb123bc` (code identical to CI source `31fb9dc`) | GAP-003 closed. `cargo package --locked --no-verify` ran in a fresh isolated copy of the audited tree. Crate sha256 `db74b57f…3171a` reproduced identically across two independent package runs. 52-file archive inventory byte-compared 52/52 between runs. Notices (LICENSE, README.md, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, TRADEMARK.md) and the dependency tree retained. Private installs from the byte-checked archive ran on Rust 1.85.0 and stable 1.98.1. Observed: version output, the 205-record effects catalog, and both packaged examples rendered to PNGs. Also an installed-binary gradient render, a byte-identical force-reinstall upgrade, and uninstall/reinstall (all exit codes 0). Exact-source CI inspected at step level: 0 skipped steps, 0 failed steps at `31fb9dc`. Raw artifacts in `docs/parity/distribution-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1). Artifact bytes matched to inventory. Notices and dependencies examined. Packaged examples, upgrade, and removal exercised. | macOS/Windows platform matrix, parameter sweeps beyond harness defaults, broader 1.85.0 artifact-parity coverage, and the broken-pipe exit-101 limitation under truncated stdout remain unqualified or uncorrected. |
| 2026-09-26 | `81953750ac7268ed6b46497ee3e3ea620ebb41c1` (source tree of audited base `31fb9dc`) | GAP-002 closed. Executed the complete installed workflow in a private root on Rust 1.85.0 and stable 1.98.1. Steps: version-matrix check/test, `cargo package --locked` in an isolated copy, and private install of the packaged crate. Observed: meaningful output (205 effects records, gradient, DSL chain, PNG-input apply), invalid-value diagnostics plus recovery, read-only-output preservation, SIGTERM cancellation plus preservation, and clean uninstall. Oracle parity with the installed stable binary: 202/205 byte-exact at tolerance 0, 3 unsupported overlay interfaces, 0 errors. Focused 1.85.0 comparison 2/2 byte-exact. Raw logs, PNGs, hashes, and reports in `docs/parity/installed-workflow-20260926/`. Workflow (CI) changes were not made. | Version matrix met (1.85.0 + stable 1.98.1). Cancellation and file preservation examined. Unavailable platforms explicit (macOS/Windows untested. CPU-only, no GPU path). | Distribution lifecycle and artifact bytes (GAP-003), parameter sweeps beyond harness defaults, and the macOS/Windows platform matrix remain unqualified. |
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | GAP-001 closed. Executed the full 205-record rendered parity run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle on an identified CPU (AMD EPYC 7713, x86_64). | 202/205 compared byte-exact at tolerance 0. 3 unsupported with the stable overlay-interface reason. 0 errors. Default 30 s timeout failure on `filter3d/flow3d` retained. The 120 s retry compares byte-exact. Raw report, log, retry, and source hashes in `docs/parity/`. Workflow changes were not made. | Installation (GAP-002), distribution (GAP-003), parameter sweeps beyond harness defaults, and platform matrices remain unqualified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Created six-section register and README link. No closures. | The generated-bundle gate exited 0. This is a reproducibility examination, not a render or Cargo package installation test. | Full audit, installed workflows, current rendered parity, platforms, and releases remain unqualified. |

Run ID: `20260924-remaining-gap-documents`.
Operational evidence (audit evidence `evidence-20260924-remaining-gap-documents`). Creating this register does not advance successful-audit timestamps or the rotation.
