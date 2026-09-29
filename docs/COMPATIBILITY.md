# noisemaker-for-rust: compatibility report

## 1. Source and authority revisions

Daily review: 2026-09-29, sixth pass. Current inspected source: [`c266f4b620bb3102476423ffb7c36750d175c860`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/c266f4b620bb3102476423ffb7c36750d175c860) — the published head this review executed. Every commit after `7f55619` is docs-only; the runtime tree is unchanged.
Full rendered parity at this SHA: measured 2026-09-27/28 and re-executed by this review (section 3, 2026-09-29 run), including the 205-record run at the newest CPU delivery end `d2965d0`. Installation, host, distribution packaging, and platform qualification are measured on Linux x86_64 (sections 2-3). The macOS/Windows platform matrix remains unmeasured. No release approval follows from this review.
Generated catalog provenance records parameter-contract revision `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`. The CPU authority (noisemaker-for-cpu `d2965d0`) now pins upstream `f24b52540af6a88d12daa05feba1a04ad61b22a2`; the snapshot effect data is byte-identical apart from the revision line, and the pinned-source manifest changes only one upstream WebAudio-runtime (`shaders/src/runtime/external-input.js`) entry the port does not execute. The port requires no source change.
Current upstream discovery (review, 2026-09-27T13:45Z): `93229933b102ba82e713402be19db57207698850`, release `1.0.194` (11:35Z). The audit's 11:05Z observation recorded `1.0.192`. Release `1.0.193` was already published at 10:00Z. Release `1.0.193` is the CPU-pinned revision `12b4d74f`. The `1.0.193` and `1.0.194` manifests are byte-identical to the recorded 1.0.176 manifest. The effect catalog is unchanged. Upstream commits above the CPU pin remain unqualified. They belong to the CPU sync queue. Bounded refresh 2026-09-27T21:23Z: `git ls-remote` records upstream main at `04e8582c1db495f5a140d59a3be78e266775b702`, beyond the CPU pin `296e0138c4744ed485b2e95de3eeb466c17629ee`; those upstream commits remain unqualified and belong to the CPU sync queue.
The observations below retain their original source and authority identities. They do not qualify later updates.
Current served kit: `0.1.21`, source `b6c24628f6b8082f07a8a0a1a0110661325d10ef`. Byte-verified against the repository on 2026-09-27 (section 4). Artifact identity does not establish host qualification.

### Earlier source observations

Report date: 2026-09-24. Source inspected: [`2ef1cc4179f5163023c26e785f533d07cf699fb4`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/2ef1cc4179f5163023c26e785f533d07cf699fb4).
Full rendered parity at this SHA: **unverified**. This is not a release approval.
A later documentation-only commit does not change this tested source identity.
Any runtime, package, or authority update requires fresh evidence before this report can qualify it.

Rust CPU renderer with 205 effects. Cargo package noisemaker-for-rust exports library noisemaker_cpu and command noisemaker-rs. Rust 1.85 is required. [Source contract](https://github.com/noisefactorllc/noisemaker-for-rust/blob/2ef1cc4179f5163023c26e785f533d07cf699fb4/README.md).

Generated catalog provenance records parameter-contract revision `44bc4ed4ac729bddaa95b083d64bee942ade35da`. Broader generated-source identity needs reconciliation with current authority.
Current upstream discovery SHA: `c9ee8a049b2b63cd300da67c01ee40baf29dc288`.
Published authority: `1.0.176`, source `c9ee8a049b2b63cd300da67c01ee40baf29dc288`.
[Immutable published manifest](https://shaders.noisedeck.app/1.0.176/effects/manifest.json) contains 210 effect IDs.
Its SHA-256 is `05c4d7b7744837ae90a3bb4c89e5403ff09448a74d9d7e824abb3d719ad3314e`.
These IDs do not define complete parameter, state, input, or platform coverage.

Served kit `0.1.13` records `2ef1cc4179f5163023c26e785f533d07cf699fb4`. [Source metadata](https://kits.noisedeck.app/rust/0/deployment-meta.json).
Historical measurements remain bound to their original revisions in [completion gaps](COMPLETION_GAPS.md).

## 2. Host and distribution matrix

Current tests and qualification limits are in [section 3](#3-parity-coverage).
The matrix below retains the earlier measured scope. A historical verified row is not a current-source or full-platform certification.

| Dimension | Status | Measured scope or limit |
|---|---|---|
| Source-level verification | verified | The generated-bundle gate exited 0. This is a reproducibility examination, not a render or Cargo package installation test. |
| Actual host rendering | verified (Linux x86_64) | 2026-09-25/26: complete native CPU rendering measured against the pinned JS oracle. Re-measured 2026-09-29 at the newest CPU delivery `d2965d0` (section 3). No browser workflow qualified by this report. |
| Minimum and current host versions | verified (Rust) | 2026-09-26: `cargo check --all-targets` and the full release test suite passed on Rust 1.85.0 (MSRV) and stable 1.98.1. The packaged crate was installed on both. [Evidence](parity/installed-workflow-20260926/sha256sums.txt). |
| Supported operating systems and backends | verified (Linux x86_64) | Linux x86_64 measured (see the installed-workflow and parity evidence). Windows and macOS: unmeasured (no host in this container). |
| Installed package and first useful result | verified (Linux x86_64) | 2026-09-26: packaged-crate install into a private root on Rust 1.85.0 and stable 1.98.1. Gradient, DSL chain, and PNG-input renders succeeded. macOS/Windows untested. [Evidence](parity/installed-workflow-20260926/sha256sums.txt). |
| Parameters, external inputs, state, and chains | verified (installed CLI, selected cases) | Installed-chain gradient, DSL chain, and `apply` PNG-input renders were exercised. Full current-authority combinations remain unmeasured. |
| Invalid input and recovery | verified (installed CLI, selected cases) | Executed 2026-09-26. Out-of-range and unknown parameters failed with actionable diagnostics (exit 1) through the installed binary. A corrected run recovered (exit 0). [Evidence](parity/installed-workflow-20260926/recovery-stable.log). |
| Upgrade, removal, and resource cleanup | verified (isolated install) | Executed 2026-09-26. Force-reinstall upgrade over an existing private root re-rendered byte-identically (Rust 1.85.0 and stable 1.98.1). Clean uninstall from the private root (exit 0, empty bin). Read-only destination preserved on failure. SIGTERM mid-render preserved the destination and left no temporary files. [Evidence](parity/distribution-20260926/steps-stable.log), [installed-workflow](parity/installed-workflow-20260926/sha256sums.txt). |
| Distribution packaging and artifact bytes | verified (isolated repackage, served kit byte-verified) | 2026-09-26: `cargo package --locked` in a fresh isolated copy of the audited tree. Crate sha256 `db74b57f…3171a` reproduced identically across two independent package runs. 52-file archive inventory byte-compared 52/52 between runs. Six notice files present. Dependency tree retained. Packaged examples built and rendered from the unpacked archive on Rust 1.85.0 and stable 1.98.1. Force-reinstall upgrade byte-identical. Uninstall/reinstall clean. `cargo install <file>.crate` is unsupported (real invocation exits 101 on both toolchains with no binary installed, [install-from-crate.log](parity/distribution-20260926/install-from-crate.log)). The unpacked byte-verified archive was used. 2026-09-27: served kit `0.1.21` (source `b6c2462`) byte-verified. Coverage: 33 of 35 files against the repository. Also the upstream MIT notice against the noisemaker reference. Also the 205-ID compat set against the catalog (section 4). [Evidence](parity/distribution-20260926/sha256sums.txt). |
| Exact-source CI | verified (step-level, `31fb9dc` head `b6c2462`) | `quality` and `test-and-package` green at source revision `31fb9dc168a2f50aac99b6c379950767b06c4c62` with 0 skipped and 0 failed steps. Export-kit dispatch green. The newest `ci` and `Export kit` runs are green at head `b6c24628f6b8082f07a8a0a1a0110661325d10ef` (2026-09-26T16:45:48Z). The runtime and `export-kit/` trees are unchanged from `b6c2462` through `863f64a`, so those runs bind the current runtime. Docs-only revisions after each head intentionally trigger no CI (path filters quoted in section 4). [Evidence](parity/distribution-20260926/jobs-36208792712.json). |
| Accessibility of provided controls | unverified | Keyboard, focus, labels, and diagnostics need host observations where applicable. |
| Release readiness | blocked | Linux x86_64 parity, installation, host, and artifact evidence are complete (sections 2-3). A release decision and crates.io publication remain out of scope for this report. The macOS/Windows platform matrix is unmeasured. |

## 3. Parity coverage

### Full render suite at the source-lock delivery end `d2965d0`, 2026-09-29

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `c266f4b620bb3102476423ffb7c36750d175c860` (tree `a3716d5cec976fada44b0fcf6efb1303c545fa86`). The runtime tree equals the tree the prior 2026-09-28 run executed; every commit after `7f55619` is docs-only (`git diff c266f4b` over `src/`, `examples/`, `tests/`, `scripts/generate_bundle.py`, `scripts/transpiler/`, `scripts/parity.py`, `scripts/test`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty).
- JavaScript CPU oracle: noisemaker-for-cpu `d2965d0b7880cee678de11ec797155c8a65c7b66` (tree `69d2ffc4731b23fddd472b830d3458915ede2bc4`, immutable authority input, full clone checked out at that commit). It now pins upstream `f24b52540af6a88d12daa05feba1a04ad61b22a2`.
- Audited range `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e..d2965d0b7880cee678de11ec797155c8a65c7b66` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/fd9d56c74ce7...d2965d0b7880)); the delivery also cites the observed range `7863f096..d2965d0b7880` and flags a force-push/unknown-diff, so both range starts were verified ancestors of `d2965d0` locally (`git merge-base --is-ancestor` exits 0) instead of assuming the range. Reviewable source evidence: the CPU-repository git bundle `noisemaker-for-cpu-d2965d0.bundle` (sha256 `565e08375f0d93c08af2520522661877529157340f9f682eec466248bd1ed310`) is archived with the job in `/workspace/evidence`; `git clone -b main <bundle>` reproduces the complete noisemaker-for-cpu main history through `d2965d0` (`git bundle verify` reports a complete history), and both merge-base ancestry checks exit 0 from a fresh clone of that bundle. The leg is ten commits: nine CPU-repo GAP-001/GAP-003/GAP-007 audit-review and docs records (`64539bcf`, `f9a36f2d`, `c87d5895`, `5e15ca86`, `6296476a`, `e0ff79cf`, `3c30781c`, `83fea7cb`, `7863f096`) and `d2965d0` (`sync: update upstream source lock and inventory through noisemaker@f24b5254`). Diffstat: 9 files, 134 insertions(+), 52 deletions(-); zero `src/` runtime files other than the generated snapshot.
- Output-neutrality audit: the snapshot diff is the `UPSTREAM_REVISION` line only (sha256 `f54a9e00…` at `fd9d56c`, `50cb6c50…` at `d2965d0`), and the pinned-source manifest changes only the revision field plus one upstream WebAudio-runtime `shaders/src` entry (`shaders/src/runtime/external-input.js`, 54120→67725 bytes, GAP-032 multi-capture `AudioInputManager` rework) that the CPU-port engine does not execute. No `shaders/effects` entry changed, the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa` (`generate_bundle.py --check` exit 0 at the unchanged contract), and the nine audit-review/doc records are CPU-repo documents the port neither consumes nor mirrors. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`d2965d0_leg_audit`).
- CPU revision: x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 66.376 s and `filter/crt` at 0.413 s, inside the 120 s timeout.
- The rebuilt Rust binary (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`, cargo 1.98.1 with relocated rustup homes under `/state/cache/rustup` and `/state/cache/cargo`) reproduced the previously recorded `d182360e…` binary hash; the runtime tree is unchanged since `7f55619`.
- Every CONTRIBUTING.md publication check was re-executed at this tree, all exit 0 (fmt, clippy -D warnings, `+1.85.0 check`, full release test suite with every result line `ok`, 0 failed, 2 oracle-gated ignored, unittest, `generate_bundle.py --check`, build, package, plus the oracle-gated `parity_spine` tests: 2 passed with `NOISEMAKER_JS_CPU_DIR` set at the `d2965d0` oracle). Details in [source identity](parity/source-identity.json) (`d2965d0_range_run`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `bd49a22a9d93b0bba3fdaff05da02ee01bf482e80ee86aad8d5c133d1542dc7d`. Run log sha256 `8f789c834b551f57a1d800d3922a7fd131a5a35e8dbd0453390de2477a14b36a`. Both raw artifacts, the CPU-repository git bundle, the verbatim suite-check logs at revision `2454cf7`, and the inspectable bundle-ancestry/range-diff artifacts are archived, exactly these files, in `/workspace/evidence/evidence-1790705589411.tar.gz` (sha256 `98122317e549c0f27ad559393a0441481649e80e06dd8fdab668435560801197`; contents listed in `MANIFEST.txt` inside the archive) per the no-committed-logs policy; no new logs are committed under `docs/parity/`.

### Full render suite at the source-lock delivery end `fd9d56c`, 2026-09-28

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `b7fbfff5417de70191261758a9a37a51cad382d5` (tree `3d10dbef7ecd41c32e193f3fa56f54d1cb1c5a51`). The runtime tree equals the tree the prior 2026-09-27 runs executed; every commit after `7f55619` is docs-only (`git diff 7f55619..b7fbfff` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty).
- JavaScript CPU oracle: noisemaker-for-cpu `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` (tree `406bea7cdf50f6080784bfd58ce78902697ccba1`, immutable authority input, full clone checked out at that commit). It now pins upstream `73c15be00d6888f4b5d2835d8e242ee9e840df45`.
- Audited range `f0ccebef830bf6b8d99eafadf73f260a6c224873..fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/f0ccebef830b...fd9d56c74ce7)). `f0ccebe` is an ancestor of `fd9d56c` (`git merge-base --is-ancestor` exits 0). The leg is three commits: `21d211e0` (`test: qualify landscape on Metal and reopen native CRT tracing`), `f290040b` (`GAP-008: report golden reference provenance separately from the kernel pin`), and `fd9d56c` (`sync: update upstream source lock and inventory through noisemaker@73c15be0`). Diffstat: 14 files, 1516 insertions(+), 27 deletions(-); zero `src/` runtime files other than the generated snapshot.
- Output-neutrality audit: the snapshot diff is the `UPSTREAM_REVISION` line only (sha256 `f78c661c…` at `f0ccebe`, `f54a9e00…` at `fd9d56c`), and the pinned-source manifest changes only the revision field plus three upstream GPU-runtime `shaders/src` entries (webgl2.js, compiler.js, pipeline.js) that the CPU-port engine does not execute. No `shaders/effects` entry changed, the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa` (`generate_bundle.py --check` exit 0 at the unchanged contract), and GAP-008's provenance tooling plus the Metal landscape audit are CPU-repo records the port neither consumes nor mirrors. The port requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`fd9d56c_leg_audit`).
- CPU revision: x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 72.633 s and `filter/crt` at 0.704 s, inside the 120 s timeout.
- The rebuilt Rust binary (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`, cargo 1.98.1 with relocated rustup homes under `/state/cache/rustup` and `/state/cache/cargo`) reproduced the previously recorded `d182360e…` binary hash; the runtime tree is unchanged since `7f55619`.
- Every CONTRIBUTING.md publication check was re-executed at this tree, all exit 0 (fmt, clippy -D warnings, `+1.85.0 check`, full release test suite with every result line `ok`, 0 failed, 2 oracle-gated ignored, unittest, `generate_bundle.py --check`, build, package, plus the oracle-gated `parity_spine` tests: 2 passed with `NOISEMAKER_JS_CPU_DIR` set at the `fd9d56c` oracle). Details in [source identity](parity/source-identity.json) (`fd9d56c_range_run`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `11d51a067ca40919dc736ed7d7a44dcc2bb9eb118b445d835876535cf0b1665e`. Run log sha256 `a4887fd389427088d2d20886413a4aaa4aec2db7b6b4527856a7d48d48a13b2f`. Raw artifacts are archived with the job in `/workspace/evidence` per the no-committed-logs policy; no new logs are committed under `docs/parity/`.

### Full render suite at the GAP-007 delivery end `f0ccebe`, 2026-09-27

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `3ae887b774f00dd2a56c647a6459c942c19b8f62` (tree `dd9dd8e573c3e58cf902ad19899d3d03e021bb9a`). The runtime tree equals the tree the prior 2026-09-27 runs executed; every commit after `7f55619` is docs-only (`git diff 7f55619..3ae887b` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty).
- JavaScript CPU oracle: noisemaker-for-cpu `f0ccebef830bf6b8d99eafadf73f260a6c224873` (immutable authority input, full clone checked out at that commit). It still pins upstream `296e0138c4744ed485b2e95de3eeb466c17629ee`.
- Audited range `b61b658399f18b5a93abd0020c02fff3be9630f5..f0ccebef830bf6b8d99eafadf73f260a6c224873` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/b61b658399f1...f0ccebef830b)). `b61b658` is an ancestor of `f0ccebe` (`git merge-base --is-ancestor` exits 0). The leg is one commit: `f0ccebef830b` (`GAP-007: give browser demo controls distinct accessible names`). Diffstat: docs/COMPLETION_GAPS 6, examples/browser/control-factory 37+, examples/browser/demo 12+, examples/browser/effect-select 11+, examples/browser/index.html 14, examples/browser/toggle-switch 5+, test/demo-pipeline.test 38+. Total 7 files, 112 insertions(+), 11 deletions(-). Zero `src/` and zero `scripts/` files.
- Output-neutrality audit: `git diff b61b658..f0ccebe -- src/ scripts/` is empty, so the snapshot (sha256 `f78c661c…` at both ends), the pinned-source manifest, and the source-lock constants are byte-identical across the range, and the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`. The leg is a browser-demo accessibility change (control aria-labels plus `test/demo-pipeline.test.js` unit tests); the Rust port mirrors no browser demo, so it requires no source change. Per-leg details: [source identity](parity/source-identity.json) (`f0ccebe_leg_audit`).
- CPU revision: x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 66.448 s and `filter/crt` at 0.424 s, inside the 120 s timeout.
- The rebuilt Rust binary (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`, cargo 1.98.1 with relocated rustup homes under `/state/cache/rustup` and `/state/cache/cargo`) reproduced the previously recorded `d182360e…` binary hash; the runtime tree is unchanged since `7f55619`.
- Every CONTRIBUTING.md publication check was re-executed at this tree, all exit 0 (fmt, clippy -D warnings, `+1.85.0 check`, full release test suite: 206 passed, 0 failed, 2 oracle-gated ignored, unittest, `generate_bundle.py --check`, build, package, plus the oracle-gated `parity_spine` tests: 2 passed with `NOISEMAKER_JS_CPU_DIR` set at the `f0ccebe` oracle). Details in [source identity](parity/source-identity.json) (`f0ccebe_range_run`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `94487e8c5f594fe1768432e207f4ed4f453bda852e30045184a141eee9730489`. Run log sha256 `7bc0543303c15a4ffb5ddd0d4c358d52e3cc3b5f3f1c0d480acc5a79a6f07826`. Raw artifacts: [report](parity/parity-report-20260927-f0ccebe.json), [raw run log](parity/parity-run-20260927-f0ccebe.log).

### Full render suite at the two-sync delivery end `b61b658`, 2026-09-27

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `4092859cec814704bbd8d195e4b53492d9067325` (tree `85ca7c66be992cafb19e0d38b555aec97c71840b`). The runtime tree equals the tree the prior 2026-09-27 runs executed; every commit after `7f55619` is docs-only (`git diff 7f55619..4092859` over `src/`, `scripts/`, `examples/`, `tests/`, `Cargo.toml`, `Cargo.lock`, `export-kit/` is empty).
- JavaScript CPU oracle: noisemaker-for-cpu `b61b658399f18b5a93abd0020c02fff3be9630f5` (immutable authority input, full clone checked out at that commit). It pins upstream `296e0138c4744ed485b2e95de3eeb466c17629ee`.
- Audited range `dedfd07c24f80d9b0adddf912a4224ce6c1d795f..b61b658399f18b5a93abd0020c02fff3be9630f5` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/dedfd07c24f8...b61b658399f1)). `dedfd07` is an ancestor of `b61b658` (`git merge-base --is-ancestor` exits 0). The leg is two commits: `34a0325c5392` (`sync: update upstream source lock and inventory through noisemaker@93229933`) and `b61b658399f1` (`sync: update upstream source lock and inventory through noisemaker@296e0138`). Diffstat: README 2, docs/COMPATIBILITY 23+, docs/COMPLETION_GAPS 2+, docs/EFFECTS 2, two new CPU audit JSONs, three refreshed CPU evidence records and logs, pinned-source-manifest 2, source-lock.js 2, snapshot 2, upstream-inventory.test.js 2. Total 14 files, 630 insertions(+), 322 deletions(-). Zero `src/` runtime files other than the generated snapshot.
- Snapshot identity: sha256 at `dedfd07` `2b7587bd1aefb5a9c90f0dece320127bcdd12d11c36b809b60d79b6f9d838472`, at `b61b658` `f78c661c4b35146106f52ca3546f2444b533a8817966b87a018dafdcda4c40ec`. The entire snapshot diff is the revision line.
- Output-neutrality audit: the pinned-source-manifest diff is the revision line only; the `PINNED_SOURCE_DIGEST` and `PINNED_SOURCE_MANIFEST_DIGEST` constants are byte-unchanged across both pin moves; and an independent fresh upstream clone records identical git tree objects for `shaders/effects` (`7ba7a60f3c3afaced19555f3078ca7f4f46af968`) and `shaders/src` (`6b791b539462bb6165f620541f6acec1573c6981`) at both `12b4d74f` and `296e0138`, so the pinned directories are byte-identical across the range. The upstream range's only `shaders/` commits (93229933, a912749f, 296e0138) touch `shaders/tests/` plus the upstream harness and test registrations — code the CPU port does not consume and no Rust equivalent mirrors. The port requires no source change, and the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`. Per-leg details and CPU-side audit artifact hashes: [source identity](parity/source-identity.json) (`b61b658_leg_audit`).
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 71.159 s and `filter/crt` at 0.382 s, inside the 120 s timeout.
- The rebuilt Rust binary (sha256 `4eb6288abc0d687ed3bd9b279ab1977ffb77ee37881c7b75b037ee4cf35e5586`, cargo 1.98.1 with relocated rustup homes under `/state/cache/scratch`) executed this run. The runtime tree is unchanged since `7f55619`; the hash differs from the previously recorded `d182360e…` binary because this container's relocated cargo/rustup homes embed different absolute paths in the build, not because of any source change.
- Every CONTRIBUTING.md publication check was re-executed at this tree, all exit 0 (fmt, clippy -D warnings, `+1.85.0 check`, full release test suite: 27 result lines all `ok`, 206 passed, 0 failed, 2 oracle-gated ignored, unittest OK, `generate_bundle.py --check`, build, package, plus the oracle-gated `parity_spine` tests: 2 passed with `NOISEMAKER_JS_CPU_DIR` set at the `b61b658` oracle). Details in [source identity](parity/source-identity.json) (`b61b658_range_run`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `b79e3ac2db3536c6e7af264b449a12090ca3e2e41352eb17ded15b75ea30da89`. Run log sha256 `63b21eb7c60d9c640a41245fca651f45d548ab47b20d162c57be26095e375b67`. Raw artifacts: [report](parity/parity-report-20260927-b61b658.json), [raw run log](parity/parity-run-20260927-b61b658.log).

### Full render suite, independent review, 2026-09-27

One 205-record parity run executed the complete catalog through both CLIs against identified revisions. This review re-executed the worker's run independently.
- Port source: `84b8168a2337ef846a6846d09c4cf32e0b481928`. The runtime tree equals the audited tree. The commits after `7f55619` are docs-only.
- JavaScript CPU oracle: noisemaker-for-cpu `dedfd07c24f80d9b0adddf912a4224ce6c1d795f` (immutable authority input, fresh clone at that commit). It pins upstream `12b4d74fb4f28d5f00bb1dde107fa8673814d8b9`.
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (worst max delta 0), 3 unsupported with the stable overlay-interface reason, 0 errors. `filter3d/flow3d` byte-exact in-run at 66.916 s.
- The rebuilt Rust binary is byte-identical to the audited binary (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`).
- Report sha256 `d8390a59d7e9ed39271ccd03b9e7eb4fd2f805d0254b89343cec66af1eff422e`. Run log sha256 `7603601aad9ca5d7a9580a1120be62ea5c941cac085f84015fef194512fe4694`. Evidence record `/series/review-20260927-133500/result.json`.
- The two ignored CI tests (`precision_boundaries_are_byte_exact`, `three_effect_js_oracle_parity_is_byte_exact` in `tests/parity_spine.rs`) pass with `NOISEMAKER_JS_CPU_DIR` set. Result: 2 passed, 0 failed.

### Full render suite at the 2026-09-27 source-lock delivery end, 2026-09-27

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `863f64a247fa6255c0e2667b273dae9aedeea073`. The runtime tree equals the tree the prior 2026-09-27 runs executed. The commits after `7f55619` are docs-only.
- JavaScript CPU oracle: noisemaker-for-cpu `dedfd07c24f80d9b0adddf912a4224ce6c1d795f` (immutable authority input, fresh clone at that commit). It pins upstream `12b4d74fb4f28d5f00bb1dde107fa8673814d8b9`.
- Audited range `7a824744cb563f2280811f04e5a49f6792ed319d..dedfd07c24f80d9b0adddf912a4224ce6c1d795f` ([compare](https://github.com/noisefactorllc/noisemaker-for-cpu/compare/7a824744cb563f2280811f04e5a49f6792ed319d...dedfd07c24f80d9b0adddf912a4224ce6c1d795f)). `7a824744` is an ancestor of `dedfd07` (`git merge-base --is-ancestor` exits 0). The leg is one commit: `dedfd07c24f80d9b0adddf912a4224ce6c1d795f` (`sync: update upstream source lock and inventory through noisemaker@12b4d74f`). Diffstat: README 2, docs/COMPATIBILITY 12+, docs/COMPLETION_GAPS 1+, docs/EFFECTS 2, CPU evidence records, pinned-source-manifest 11+, source-lock.js 6, snapshot 2, upstream-inventory.test.js 2. Total 13 files, 462 insertions(+), 340 deletions(-). Zero `src/` runtime files other than the generated snapshot.
- Snapshot identity: sha256 at `7a824744` `7c2699a5c686e93572e56bb08f0985b22bf48a4a414799f9b15dab0da5a79759`, at `dedfd07` `2b7587bd1aefb5a9c90f0dece320127bcdd12d11c36b809b60d79b6f9d838472`. The entire snapshot diff is the revision line.
- Manifest corroboration: the pinned-source-manifest diff changes the pin revision, `shaders/src/runtime/pipeline.js` (size and sha256), and adds `shaders/src/runtime/preflight.js`. No `shaders/effects` entries changed. The pin moved `7443f6e6..12b4d74f` with the effect catalog unchanged, so the port requires no source change and the parameter contract stays at `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`.
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 64.103 s, inside the 120 s timeout.
- The rebuilt Rust binary is byte-identical to the binary the prior runs executed (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `9717bb7923254d5db7f6a58313812e05333290da0862e8d5511909c58430ae49`. Run log sha256 `b8a63de60572c77a62e488c91b7836a1454abb86dbffc05724076b621ab0acc9`. Raw artifacts: [report](parity/parity-report-20260927-dedfd07.json), [raw run log](parity/parity-run-20260927-dedfd07.log).

### Full render suite at the 2026-09-27 new-pin delivery end, 2026-09-27

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `347e298348fbf61993f537f2a7689e798674d7e4`. The runtime tree equals the `7f55619`/`986b5f8` tree the prior runs executed. The commits after `7f55619` are docs-only.
- JavaScript CPU oracle: noisemaker-for-cpu `7a824744cb563f2280811f04e5a49f6792ed319d` (immutable authority input, fresh clone at that commit). It pins upstream `7443f6e6180300a45c5b97608459e5094504659d`.
- Pin range `6a0af04d..7443f6e6`: changes upstream shader-language sources and tests. Changed source: `shaders/src/lang/transform.js` (+377 lines, GAP-008 replaceEffect prediction). The effect catalog is unchanged, and the CPU snapshot changes only its revision line over `901bbd9..7a824744`.
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. `filter3d/flow3d` compared byte-exact in-run at 72.072 s, inside the 120 s timeout.
- The rebuilt Rust binary is byte-identical to the binary the prior runs executed (sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`).
- The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. Excluded IDs: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
- Report sha256 `4e99dd64d154b557eb5439c27854a7f8b07dedc402ca894b9ca9da244ed8d94a`. Run log sha256 `6f0d746e9c6c6cda2d1c60dccc4795300f0679ce4ea9b38f4bee3099681da786`. Evidence record `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.

### Full render suite at the 2026-09-26 GAP-002/003 audit range end, 2026-09-26

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `7f5561908b343b1fce11c2e7e1949d4446a1e6ff` (tree `986b5f8ee4602ffb2349d2fa20245f14d5d890ff`). The committed record following this run is docs-only with no `src/` or `scripts/` changes.
- JavaScript CPU oracle: noisemaker-for-cpu `12db707f3f49e93a9d204263415a2a2ea55cb828` (immutable authority input, fresh clone at its published default branch).
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors.
- This run follows the noisemaker-for-cpu range audit `4b590d2f7f60..12db707f3f49` recorded in [source identity](parity/source-identity.json). The delivered range start `bfbe54764eee` is an ancestor of `4b590d2f7f60` (merge-base verified locally, not assumed). The previously audited legs (`upstream_range_audit`, `forced_range_leg_audit`, `followup_range_audit`, `crt_range_leg_audit`) plus the new leg `4b590d2f7f60..12db707f3f49` cover the whole declared delivery `bfbe54764eee..12db707f3f49`. The delivery also cites the observed sub-span `7145223b522f..12db707f3f49`, which is contained in it.
- The new leg's two commits (`7145223`, `12db707`) are CPU-repo audit records for GAP-002 (bounded external-input parity comparison) and GAP-003 (bounded landscape authority comparison). The runtime diff `4b590d2..12db707` over `src/`, `scripts/` is empty. The generated snapshot is byte-identical at both ends (sha256 `df429875fc9af9a24f6e9026b72f4d5b6dd75faf94e6c27dcbdd50f8e1344d7a`). The parameter contract stays at revision `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`, so the port requires no source change.
- `filter/crt` and `filter3d/flow3d` both compare byte-exact (max delta 0) in this run.
- Raw output, the machine-readable report, and exact source hashes: [parity report](parity/parity-report-20260926-12db707.json), [raw run log](parity/parity-run-20260926-12db707.log), [source identity](parity/source-identity.json).

### Full render suite at the 2026-09-26 CRT-range end, 2026-09-26

One 205-record parity run executed the complete catalog through both CLIs against identified revisions for the newest noisemaker-for-cpu delivery.
- Port source: `b6c24628f6b8082f07a8a0a1a0110661325d10ef` (tree `f27ce1b7eb6a6f51344e70bbfbe2f0fbd2cc7587`). The committed records following this run are docs-only with no `src/` or `scripts/` changes.
- JavaScript CPU oracle: noisemaker-for-cpu `4b590d2f7f607a2788a5bc7675288ec166d0b633` (immutable authority input, fresh clone at its published default branch).
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors.
- This run follows the noisemaker-for-cpu range audit `bfbe54764eee..4b590d2f7f60` recorded in [source identity](parity/source-identity.json). `41b9268` is `bfbe547`'s parent, and `bfbe547` is an ancestor of `4b590d2f7f60`. The observed sub-span `ea198510ae27..4b590d2f7f60` is contained in it. The previously audited legs plus the new `d03aed7b3038..4b590d2f7f60` leg cover the whole declared delivery.
- The new leg's only `src/` commit (`8debec5`, a filter/crt adapter cos-wrap) was reverted in its next commit (`c0d53d0`). The runtime diff `d03aed7..4b590d2` over `src/`, `scripts/upstream/` is empty. The generated snapshot is byte-identical from `d03aed7` through `4b590d2`. The remaining changes are CPU-repo docs, committed parity artifacts, and CRT characterization tests the port does not mirror. The port requires no source change, and the generated parameter contract stays at revision `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa`.
- `filter/crt` and `filter3d/flow3d` both compare byte-exact (max delta 0) in this run.
- Raw output, the machine-readable report, and exact source hashes: [parity report](parity/parity-report-20260926-ea1985.json), [raw run log](parity/parity-run-20260926-ea1985.log), [source identity](parity/source-identity.json).

### Full render suite at the 2026-09-26 follow-up range end, 2026-09-26

One 205-record parity run executed the complete catalog through both CLIs against identified revisions, binding the previously unported upstream range.
- Port source: `1eb130d9df3d70da8f9a3f0b157a1de0d2bbc875` (tree `9a42d8e0c48eca550e7e30109755473ad6f16901`).
- JavaScript CPU oracle: noisemaker-for-cpu `d03aed7b30384bcebc04bc7f73a7f750ce3b2227` (immutable authority input, resolved by pinned checkout).
- CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s.
- Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors.
- This run follows the noisemaker-for-cpu range audits recorded in [source identity](parity/source-identity.json): the forced-delivery leg `41b9268..bfbe54764eee` and the observed range `79c3ad626f82..d03aed7b3038`. Together they cover both cited delivery ranges because `41b9268` is `bfbe547`'s parent and `bfbe547` is an ancestor of `d03aed7`.
- The forced leg bumps the pinned upstream revision from `9d3474dfdc6cb737ebb7b2f3598b16d940af1544` to `8eeb7b5ac14eb37a8d16037f607a88ce63924cd3`. Snapshot effect data is byte-identical apart from the UPSTREAM_REVISION line (per-leg snapshot diff and per-entry manifest comparison recorded). It changes CPU-internal manifests and contains the only src/ runtime change: CPU renderer.js texture-dimension handling that no shipped catalog effect exercises.
- The observed leg bumps the pin from `8eeb7b5ac14eb37a8d16037f607a88ce63924cd3` to `6a0af04d3c4f345ffab5e9f8e54e532216b4cdaa` with identical snapshot effect data and two CPU-repo docs-only audit commits. The port reduces to a parameter-contract provenance bump.
- Raw output, the machine-readable report, and exact source hashes: [parity report](parity/parity-report-20260926-ranged.json), [raw run log](parity/parity-run-20260926-ranged.log), [source identity](parity/source-identity.json).

### Full render suite, 2026-09-26 (prior range end 41b9268)

One 205-record parity run executed the complete catalog through both CLIs against identified revisions. Port source: `0638cacece3e24f6922f32aab8c032a275ae05c0`. JavaScript CPU oracle: noisemaker-for-cpu `41b92689c23fb1ccf9b513ecc8a30fbe11365473` (immutable authority input, resolved by pinned checkout). CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s. Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. This run follows the noisemaker-for-cpu range `aaa6df50421d..41b9268` audit recorded in [source identity](parity/source-identity.json). The range bumped the pinned upstream revision to `9d3474dfdc6cb737ebb7b2f3598b16d940af1544` with unchanged snapshot effect data and adds CPU-internal anti-staleness tooling. The port reduces to a parameter-contract provenance bump. Raw output, the machine-readable report, and exact source hashes: [parity report](parity/parity-report-20260926.json), [raw run log](parity/parity-run-20260926.log), [source identity](parity/source-identity.json).

### Previous full render suite, 2026-09-25

One 205-record parity run executed the complete catalog through both CLIs against identified revisions. Port source: `25f1340c6b087d648bb5b4249b2f4d94be5d5c02`. JavaScript CPU oracle: noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` (immutable authority input, resolved by pinned checkout). CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s. Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. The harness default 30-second timeout first reported `filter3d/flow3d` as a timeout failure. The retained 120-second retry compares byte-exact (rust 66.841 s). The default-timeout failure is retained in this record, not discarded. Raw output, the machine-readable report, the retry record, and exact source hashes: [parity report](parity/parity-report-20260925.json), [raw run log](parity/parity-run-20260925.log), [flow3d retry](parity/parity-flow3d-120s-retry.json), [source identity](parity/source-identity.json).

Remaining coverage limits after the 2026-09-26 installed-workflow and distribution runs are listed here. Unmeasured: parameter sweeps beyond the documented defaults, stateful sequences beyond the harness chain shape, and broader 1.85.0 artifact-parity coverage beyond the focused leg. The macOS/Windows platform matrix also remains unmeasured. The installed CLI qualification (GAP-002) and the distribution packaging/lifecycle qualification (GAP-003) are measured on Linux x86_64. See [installed-workflow-20260926](parity/installed-workflow-20260926/sha256sums.txt) and [distribution-20260926](parity/distribution-20260926/sha256sums.txt). No skip or tolerated difference counts as exact parity.

### Earlier measurements

Full parity requires complete applicable coverage with no skips or missing cases.
Historical NEAR, CHAOS, and tolerated differences do not count as strict equality.
The existing numerical contracts remain separate from exact comparison. This report does not change tolerances or goldens.
Unknown values mean `not measured`, never zero.

| Gate | Expected cases | Executed | Strict passes | Failures | Skips | Status |
|---|---|---|---|---|---|---|
| Current full render suite | 205 | 205 (202 compared, 3 unsupported with reasons) | 202 (byte-exact) | 0 | 0 | 2026-09-29 run at the newest delivery end. Port `c266f4b` vs noisemaker-for-cpu `d2965d0` (upstream pin `f24b5254`), tolerance 0. See section 3 |

Earlier served compatibility inventory declares 205 effect IDs. Declaration does not establish execution or parity.
IDs absent from the served declaration: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
Missing effects remain visible toward the full-parity goal. Contract exclusions do not become successful tests.

Current served declaration: 205 effect IDs. This inventory is not evidence of execution. The declaration column below reflects kit `0.1.21`, byte-verified against the repository on 2026-09-27.

### Effect inventory

| Effect ID | Declared in served kit | Current full parity |
|---|---|---|
| `classicNoisedeck/bitEffects` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/caustic` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/cellNoise` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/cellRefract` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/coalesce` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/colorLab` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/composite` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/effects` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/fractal` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/glitch` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/kaleido` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/lensDistortion` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/moodscape` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/noise` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/noise3d` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/refract` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/shapeMixer` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/shapes` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/shapes3d` | yes | pass (byte-exact, tolerance 0) |
| `classicNoisedeck/splat` | yes | pass (byte-exact, tolerance 0) |
| `filter/adjust` | yes | pass (byte-exact, tolerance 0) |
| `filter/bloom` | yes | pass (byte-exact, tolerance 0) |
| `filter/blur` | yes | pass (byte-exact, tolerance 0) |
| `filter/bulge` | yes | pass (byte-exact, tolerance 0) |
| `filter/celShading` | yes | pass (byte-exact, tolerance 0) |
| `filter/channel` | yes | pass (byte-exact, tolerance 0) |
| `filter/chroma` | yes | pass (byte-exact, tolerance 0) |
| `filter/chromaticAberration` | yes | pass (byte-exact, tolerance 0) |
| `filter/chrome` | yes | pass (byte-exact, tolerance 0) |
| `filter/clouds` | yes | pass (byte-exact, tolerance 0) |
| `filter/colorReplace` | yes | pass (byte-exact, tolerance 0) |
| `filter/convolutionFeedback` | yes | pass (byte-exact, tolerance 0) |
| `filter/corrupt` | yes | pass (byte-exact, tolerance 0) |
| `filter/craquelure` | yes | pass (byte-exact, tolerance 0) |
| `filter/crt` | yes | pass (byte-exact, tolerance 0) |
| `filter/degauss` | yes | pass (byte-exact, tolerance 0) |
| `filter/deriv` | yes | pass (byte-exact, tolerance 0) |
| `filter/directionalBlur` | yes | pass (byte-exact, tolerance 0) |
| `filter/dither` | yes | pass (byte-exact, tolerance 0) |
| `filter/edge` | yes | pass (byte-exact, tolerance 0) |
| `filter/emboss` | yes | pass (byte-exact, tolerance 0) |
| `filter/extrude` | yes | pass (byte-exact, tolerance 0) |
| `filter/feedback` | yes | pass (byte-exact, tolerance 0) |
| `filter/fibers` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag. The Rust parity CLI path intentionally uses Initial semantics) |
| `filter/flipMirror` | yes | pass (byte-exact, tolerance 0) |
| `filter/fxaa` | yes | pass (byte-exact, tolerance 0) |
| `filter/glowingEdge` | yes | pass (byte-exact, tolerance 0) |
| `filter/glyphMap` | yes | pass (byte-exact, tolerance 0) |
| `filter/grade` | yes | pass (byte-exact, tolerance 0) |
| `filter/grain` | yes | pass (byte-exact, tolerance 0) |
| `filter/grime` | yes | pass (byte-exact, tolerance 0) |
| `filter/halftone` | yes | pass (byte-exact, tolerance 0) |
| `filter/hatch` | yes | pass (byte-exact, tolerance 0) |
| `filter/highPass` | yes | pass (byte-exact, tolerance 0) |
| `filter/historicPalette` | yes | pass (byte-exact, tolerance 0) |
| `filter/invert` | yes | pass (byte-exact, tolerance 0) |
| `filter/lens` | yes | pass (byte-exact, tolerance 0) |
| `filter/lensFlare` | yes | pass (byte-exact, tolerance 0) |
| `filter/lensWarp` | yes | pass (byte-exact, tolerance 0) |
| `filter/lightLeak` | yes | pass (byte-exact, tolerance 0) |
| `filter/lighting` | yes | pass (byte-exact, tolerance 0) |
| `filter/lowPoly` | yes | pass (byte-exact, tolerance 0) |
| `filter/median` | yes | pass (byte-exact, tolerance 0) |
| `filter/morphology` | yes | pass (byte-exact, tolerance 0) |
| `filter/mosaicTiles` | yes | pass (byte-exact, tolerance 0) |
| `filter/motionBlur` | yes | pass (byte-exact, tolerance 0) |
| `filter/normalMap` | yes | pass (byte-exact, tolerance 0) |
| `filter/normalize` | yes | pass (byte-exact, tolerance 0) |
| `filter/octaveWarp` | yes | pass (byte-exact, tolerance 0) |
| `filter/oilPaint` | yes | pass (byte-exact, tolerance 0) |
| `filter/osd` | yes | pass (byte-exact, tolerance 0) |
| `filter/outline` | yes | pass (byte-exact, tolerance 0) |
| `filter/palette` | yes | pass (byte-exact, tolerance 0) |
| `filter/parallax` | yes | pass (byte-exact, tolerance 0) |
| `filter/patchwork` | yes | pass (byte-exact, tolerance 0) |
| `filter/photocopy` | yes | pass (byte-exact, tolerance 0) |
| `filter/pinch` | yes | pass (byte-exact, tolerance 0) |
| `filter/pixelSort` | yes | pass (byte-exact, tolerance 0) |
| `filter/pixels` | yes | pass (byte-exact, tolerance 0) |
| `filter/plasticWrap` | yes | pass (byte-exact, tolerance 0) |
| `filter/polar` | yes | pass (byte-exact, tolerance 0) |
| `filter/pondRipples` | yes | pass (byte-exact, tolerance 0) |
| `filter/posterize` | yes | pass (byte-exact, tolerance 0) |
| `filter/prismaticAberration` | yes | pass (byte-exact, tolerance 0) |
| `filter/reindex` | yes | pass (byte-exact, tolerance 0) |
| `filter/relief` | yes | pass (byte-exact, tolerance 0) |
| `filter/repeat` | yes | pass (byte-exact, tolerance 0) |
| `filter/reverb` | yes | pass (byte-exact, tolerance 0) |
| `filter/ridge` | yes | pass (byte-exact, tolerance 0) |
| `filter/rotate` | yes | pass (byte-exact, tolerance 0) |
| `filter/scale` | yes | pass (byte-exact, tolerance 0) |
| `filter/scanlineError` | yes | pass (byte-exact, tolerance 0) |
| `filter/scatter` | yes | pass (byte-exact, tolerance 0) |
| `filter/scratches` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag. The Rust parity CLI path intentionally uses Initial semantics) |
| `filter/scroll` | yes | pass (byte-exact, tolerance 0) |
| `filter/seamless` | yes | pass (byte-exact, tolerance 0) |
| `filter/sharpen` | yes | pass (byte-exact, tolerance 0) |
| `filter/simpleAberration` | yes | pass (byte-exact, tolerance 0) |
| `filter/sine` | yes | pass (byte-exact, tolerance 0) |
| `filter/skew` | yes | pass (byte-exact, tolerance 0) |
| `filter/smooth` | yes | pass (byte-exact, tolerance 0) |
| `filter/smoothstep` | yes | pass (byte-exact, tolerance 0) |
| `filter/snow` | yes | pass (byte-exact, tolerance 0) |
| `filter/sobel` | yes | pass (byte-exact, tolerance 0) |
| `filter/spatter` | yes | pass (byte-exact, tolerance 0) |
| `filter/spinBlur` | yes | pass (byte-exact, tolerance 0) |
| `filter/spiral` | yes | pass (byte-exact, tolerance 0) |
| `filter/spookyTicker` | yes | pass (byte-exact, tolerance 0) |
| `filter/stamp` | yes | pass (byte-exact, tolerance 0) |
| `filter/step` | yes | pass (byte-exact, tolerance 0) |
| `filter/stipple` | yes | pass (byte-exact, tolerance 0) |
| `filter/strayHair` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag. The Rust parity CLI path intentionally uses Initial semantics) |
| `filter/strokes` | yes | pass (byte-exact, tolerance 0) |
| `filter/temporalAberration` | yes | pass (byte-exact, tolerance 0) |
| `filter/tetraColorArray` | yes | pass (byte-exact, tolerance 0) |
| `filter/tetraCosine` | yes | pass (byte-exact, tolerance 0) |
| `filter/text` | yes | pass (byte-exact, tolerance 0) |
| `filter/texture` | yes | pass (byte-exact, tolerance 0) |
| `filter/threshold` | yes | pass (byte-exact, tolerance 0) |
| `filter/tile` | yes | pass (byte-exact, tolerance 0) |
| `filter/tint` | yes | pass (byte-exact, tolerance 0) |
| `filter/translate` | yes | pass (byte-exact, tolerance 0) |
| `filter/tunnel` | yes | pass (byte-exact, tolerance 0) |
| `filter/unsharpMask` | yes | pass (byte-exact, tolerance 0) |
| `filter/vaseline` | yes | pass (byte-exact, tolerance 0) |
| `filter/vignette` | yes | pass (byte-exact, tolerance 0) |
| `filter/warp` | yes | pass (byte-exact, tolerance 0) |
| `filter/watercolor` | yes | pass (byte-exact, tolerance 0) |
| `filter/waves` | yes | pass (byte-exact, tolerance 0) |
| `filter/wind` | yes | pass (byte-exact, tolerance 0) |
| `filter/wobble` | yes | pass (byte-exact, tolerance 0) |
| `filter/wormhole` | yes | pass (byte-exact, tolerance 0) |
| `filter/zoomBlur` | yes | pass (byte-exact, tolerance 0) |
| `filter3d/flow3d` | yes | pass (byte-exact, tolerance 0) |
| `filter3d/palette3d` | yes | pass (byte-exact, tolerance 0) |
| `mixer/alphaMask` | yes | pass (byte-exact, tolerance 0) |
| `mixer/applyMode` | yes | pass (byte-exact, tolerance 0) |
| `mixer/blendMode` | yes | pass (byte-exact, tolerance 0) |
| `mixer/cellSplit` | yes | pass (byte-exact, tolerance 0) |
| `mixer/centerMask` | yes | pass (byte-exact, tolerance 0) |
| `mixer/channelCombine` | yes | pass (byte-exact, tolerance 0) |
| `mixer/distortion` | yes | pass (byte-exact, tolerance 0) |
| `mixer/focusBlur` | yes | pass (byte-exact, tolerance 0) |
| `mixer/mashup` | yes | pass (byte-exact, tolerance 0) |
| `mixer/patternMix` | yes | pass (byte-exact, tolerance 0) |
| `mixer/shadow` | yes | pass (byte-exact, tolerance 0) |
| `mixer/shapeMask` | yes | pass (byte-exact, tolerance 0) |
| `mixer/split` | yes | pass (byte-exact, tolerance 0) |
| `mixer/thresholdMix` | yes | pass (byte-exact, tolerance 0) |
| `mixer/uvRemap` | yes | pass (byte-exact, tolerance 0) |
| `points/attractor` | yes | pass (byte-exact, tolerance 0) |
| `points/buddhabrot` | yes | pass (byte-exact, tolerance 0) |
| `points/dla` | yes | pass (byte-exact, tolerance 0) |
| `points/flock` | yes | pass (byte-exact, tolerance 0) |
| `points/flow` | yes | pass (byte-exact, tolerance 0) |
| `points/heightGrid` | yes | pass (byte-exact, tolerance 0) |
| `points/hydraulic` | yes | pass (byte-exact, tolerance 0) |
| `points/lenia` | yes | pass (byte-exact, tolerance 0) |
| `points/life` | yes | pass (byte-exact, tolerance 0) |
| `points/physarum` | yes | pass (byte-exact, tolerance 0) |
| `points/physical` | yes | pass (byte-exact, tolerance 0) |
| `render/loopBegin` | yes | pass (byte-exact, tolerance 0) |
| `render/loopEnd` | yes | pass (byte-exact, tolerance 0) |
| `render/meshLoader` | no | excluded (media/runtime interface absent from this standalone CPU port) |
| `render/meshRender` | no | excluded (media/runtime interface absent from this standalone CPU port) |
| `render/pointsBillboardRender` | yes | pass (byte-exact, tolerance 0) |
| `render/pointsEmit` | yes | pass (byte-exact, tolerance 0) |
| `render/pointsRender` | yes | pass (byte-exact, tolerance 0) |
| `render/render3d` | yes | pass (byte-exact, tolerance 0) |
| `render/renderCubemap3d` | yes | pass (byte-exact, tolerance 0) |
| `render/renderCubemapSurface` | yes | pass (byte-exact, tolerance 0) |
| `render/renderLandscape3d` | yes | pass (byte-exact, tolerance 0) |
| `render/renderLit3d` | yes | pass (byte-exact, tolerance 0) |
| `synth/bitwise` | yes | pass (byte-exact, tolerance 0) |
| `synth/cell` | yes | pass (byte-exact, tolerance 0) |
| `synth/cellularAutomata` | yes | pass (byte-exact, tolerance 0) |
| `synth/curl` | yes | pass (byte-exact, tolerance 0) |
| `synth/gabor` | yes | pass (byte-exact, tolerance 0) |
| `synth/gradient` | yes | pass (byte-exact, tolerance 0) |
| `synth/julia` | yes | pass (byte-exact, tolerance 0) |
| `synth/mandala` | yes | pass (byte-exact, tolerance 0) |
| `synth/mandelbrot` | yes | pass (byte-exact, tolerance 0) |
| `synth/media` | yes | pass (byte-exact, tolerance 0) |
| `synth/mnca` | yes | pass (byte-exact, tolerance 0) |
| `synth/modPattern` | yes | pass (byte-exact, tolerance 0) |
| `synth/navierStokes` | yes | pass (byte-exact, tolerance 0) |
| `synth/newton` | yes | pass (byte-exact, tolerance 0) |
| `synth/noise` | yes | pass (byte-exact, tolerance 0) |
| `synth/osc2d` | yes | pass (byte-exact, tolerance 0) |
| `synth/pattern` | yes | pass (byte-exact, tolerance 0) |
| `synth/perlin` | yes | pass (byte-exact, tolerance 0) |
| `synth/polygon` | yes | pass (byte-exact, tolerance 0) |
| `synth/reactionDiffusion` | yes | pass (byte-exact, tolerance 0) |
| `synth/remap` | yes | pass (byte-exact, tolerance 0) |
| `synth/roll` | no | excluded (media/runtime interface absent from this standalone CPU port) |
| `synth/sacredGeometry` | yes | pass (byte-exact, tolerance 0) |
| `synth/scope` | no | excluded (media/runtime interface absent from this standalone CPU port) |
| `synth/shape` | yes | pass (byte-exact, tolerance 0) |
| `synth/solid` | yes | pass (byte-exact, tolerance 0) |
| `synth/spectrum` | no | excluded (media/runtime interface absent from this standalone CPU port) |
| `synth/subdivide` | yes | pass (byte-exact, tolerance 0) |
| `synth/testPattern` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/cell3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/cellularAutomata3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/flythrough3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/fractal3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/heightmap3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/noise3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/reactionDiffusion3d` | yes | pass (byte-exact, tolerance 0) |
| `synth3d/shape3d` | yes | pass (byte-exact, tolerance 0) |

## 4. Evidence

Review CI boundary: Exact-source runs: Export kit, ci. A passing export dispatch does not qualify rendered parity. Current complete-render enforcement remains an open verification requirement. [Exact-source responses and workflows](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/noisemaker-for-rust-remote-evidence.json).

2026-09-27 executed evidence. Environment: Linux x86_64, AMD EPYC 7713, Linux 6.8.0-134-generic. Toolchain: cargo 1.98.1 installed user-level, node v26.5.1, Python 3.11.2.

- `cargo build --release --locked` exit 0. Binary sha256 `d182360e22cc5fe44c4dd0b6d58fdb810a2a0daaef388ea27831210ec5244b01`.
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/parity.py --rust target/release/noisemaker-rs --js <fresh clone of noisemaker-for-cpu at 7a824744>/bin/noisemaker-cpu.js --timeout 120 --json <report>` exit 0. Result: 202 compared byte-exact, 3 unsupported, 0 errors.
- `PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check` exit 0.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` exit 0. 19 tests OK.
- Served kit `0.1.21` byte-verification: 33 of 35 `kit.json` files against `git show b6c2462:<path>` under the `export-kit/kit.config.json` mappings. `LICENSES/noisemaker-MIT.txt` against the noisemaker reference `LICENSE` at `e73a44a`. `compat.json` ID set equals the 205-entry catalog.
- Workflow path filters quoted from the workflow files. `ci.yml` push paths: `'**'`, `'!*.md'`, `'!.github/ISSUE_TEMPLATE/**'`, `'!.github/PULL_REQUEST_TEMPLATE*'`, `'!.github/FUNDING.yml'`, `'!.github/CODEOWNERS'`, `'!.editorconfig'`, `'!docs/**'`, `'docs/EFFECTS.md'`. `export-kit.yml` push paths: `'export-kit/**'`, `'.github/workflows/export-kit.yml'`, `'LICENSE'`, `'Cargo.toml'`, `'Cargo.lock'`, `'src/**'`. Audit document changes match neither filter set. Publishing them triggers no CI and no kit republish.
- Full evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.

Independent review 2026-09-27. Environment: Linux x86_64, AMD EPYC 7713, Linux 6.8.0-134-generic. Toolchain cargo 1.98.1 installed user-level, node v26.5.1, Python 3.11.2.

- `cargo build --release --locked` exit 0. Binary sha256 `d182360e…`, byte-identical to the audited binary.
- Full 205-record parity run at a fresh `dedfd07` oracle clone: exit 0. Result: 202 compared byte-exact at tolerance 0, 3 unsupported, 0 errors. Report sha256 `d8390a59…`.
- `NOISEMAKER_JS_CPU_DIR=<oracle> cargo test --release --locked --test parity_spine -- --ignored` exit 0. Both previously ignored tests pass (2 passed, 0 failed).
- CLI probes through the rebuilt binary: `effects | head -3` exits 101 with the broken-pipe panic. `generate --param colorCount=99` exits 1 with an actionable diagnostic. A corrected run exits 0 and renders a PNG.
- Served kit `0.1.21` sampled files: `engine/src/lib.rs`, `engine/Cargo.toml`, `engine/Cargo.lock` byte-match `git show b6c2462:<path>`. `LICENSES/noisemaker-MIT.txt` byte-matches the noisemaker `LICENSE` at `12b4d74f`.
- GAP-002 evidence manifest hash-verified 37 of 37 files. GAP-003 evidence manifest hash-verified 26 of 27 entries (self-listing defect only).
- Evidence record: `/series/review-20260927-133500/result.json`.

[Bounded test evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/rust-tests.json). [Exact-source Actions](https://github.com/noisefactorllc/noisemaker-for-rust/actions?query=head_sha%3A2ef1cc4179f5163023c26e785f533d07cf699fb4).
[This run evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents) retains commands, exit codes, source identities, and distribution metadata.
Official ecosystem reference: [Current Cargo Book, accessed 2026-09-24](https://doc.rust-lang.org/cargo/reference/publishing.html).
Source CI, export dispatch, artifact delivery, and rendered parity are separate evidence dimensions.
A successful dispatch or unit-test summary does not establish a full rendered gate.

## 5. Open compatibility limits

Next bounded verification: Rendered parity was re-measured 2026-09-27 and independently re-executed by this review (section 3). The served kit `0.1.21` was byte-verified (section 4). The two ignored CI tests are oracle-gated tests in `tests/parity_spine.rs`, not doctests. They pass with `NOISEMAKER_JS_CPU_DIR` set. CI resolution stays with the implementation job. Next executable verifications need unavailable hosts or broader sweeps: macOS and Windows platform legs, and parameter sweeps beyond the harness defaults.
See the stable entries in [completion gaps](COMPLETION_GAPS.md).

See [GAP-001 and the complete gap register](COMPLETION_GAPS.md#4-known-gaps) for evidence, dependencies, and acceptance criteria.

1. Reconcile the current authority and complete case inventory, including parameters, inputs, stateful frames, and host versions. Done for the documented harness denominator (section 3). Broader sweeps remain open.
2. Run the existing actual-renderer suite without skip options. Record every missing, failed, refused, or timed-out case. Executed 2026-09-25 (section 3). Parameter sweeps and stateful sequences beyond the harness shape remain open.
3. Verify installation, useful output, errors, recovery, upgrades, and removal with the actual distribution.
4. Inspect exact-source CI and retain artifact hashes. Keep unresolved qualification failed or unverified.

All eligible ports have equal priority. Full parity and zero skipped cases remain the goal.
Implementation corrections remain with the separate job. This report does not advance the parity checkpoint.

## 6. History

2026-09-29 sixth-pass sync review at `c266f4b620bb3102476423ffb7c36750d175c860`: rendered parity re-measured at the newest CPU delivery end `d2965d0` (upstream pin now `f24b5254`; one pinned upstream WebAudio-runtime `shaders/src` entry changed, effect catalog unchanged), with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. See [completion gaps](COMPLETION_GAPS.md).
2026-09-28 fifth-pass sync review at `b7fbfff5417de70191261758a9a37a51cad382d5`: rendered parity re-measured at the newest CPU delivery end `fd9d56c` (upstream pin now `73c15be0`; pinned upstream GPU-runtime `shaders/src` entries changed, effect catalog unchanged), with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. See [completion gaps](COMPLETION_GAPS.md).
2026-09-27 fourth-pass sync review at `3ae887b774f00dd2a56c647a6459c942c19b8f62`: rendered parity re-measured at the newest CPU delivery end `f0ccebe` (upstream pin `296e0138` unchanged), with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. See [completion gaps](COMPLETION_GAPS.md).
2026-09-27 third-pass daily review at `4092859cec814704bbd8d195e4b53492d9067325`: rendered parity re-measured at the newest CPU delivery end `b61b658` (upstream pin `296e0138`), with the full publication-check set re-executed at this tree. No gap opened, closed, or reopened. See [completion gaps](COMPLETION_GAPS.md).
Independent review 2026-09-27 at `84b8168a2337ef846a6846d09c4cf32e0b481928`: this review re-executed the 205-record parity run at the `dedfd07` oracle. Result: 202 byte-exact, 3 unsupported, 0 errors. It re-verified all three closure claims. It identified both ignored CI tests and passed them locally. It refreshed the upstream discovery to `9322993` (`1.0.194`). See [completion gaps](COMPLETION_GAPS.md).
2026-09-27 daily review at `863f64a247fa6255c0e2667b273dae9aedeea073`: rendered parity re-measured at the newest CPU delivery `dedfd07` (upstream pin `12b4d74f`), with GAP-002/GAP-003 evidence carried across the unchanged runtime tree. No gap opened, closed, or reopened. See [completion gaps](COMPLETION_GAPS.md).
2026-09-27 daily review at `347e298348fbf61993f537f2a7689e798674d7e4`: rendered parity re-measured at the newest CPU delivery, served kit `0.1.21` byte-verified, and GAP-002/GAP-003 evidence carried across the unchanged runtime tree. No gap opened, closed, or reopened. Evidence record: `/series/evidence-audit-20260927-093000/result-noisemaker-for-rust.json`.

| Date | Source | Result | Change |
|---|---|---|---|
| 2026-09-29 (sixth pass, sync) | `c266f4b620bb3102476423ffb7c36750d175c860` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `fd9d56c74ce7500b7eaeea90a93d3bf49375d28e..d2965d0b7880cee678de11ec797155c8a65c7b66` (upstream pin now `f24b5254…`); the delivery also cites the observed range `7863f096…d2965d0` and flags a force-push, so both range starts were verified ancestors of `d2965d0` locally (merge-base verified, not assumed). The leg is ten CPU commits: nine GAP-001/GAP-003/GAP-007 audit-review and docs records (`64539bcf`, `f9a36f2d`, `c87d5895`, `5e15ca86`, `6296476a`, `e0ff79cf`, `3c30781c`, `83fea7cb`, `7863f096`) and `d2965d0` (`sync: update upstream source lock and inventory through noisemaker@f24b5254`); 9 files, 134+/52-, zero `src/` runtime files other than the generated snapshot. The snapshot changes only its revision line (sha256s `f54a9e00…` at `fd9d56c`, `50cb6c50…` at `d2965d0`); the pinned-source manifest changes only the revision field plus one upstream WebAudio-runtime `shaders/src` entry (`shaders/src/runtime/external-input.js`, 54120→67725 bytes, GAP-032 `AudioInputManager` rework) the port does not execute. The parameter contract stays at `6a0af04d…`, so the Rust port requires no source change. Executed the 205-record run against the `d2965d0` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 66.376 s. Re-executed every CONTRIBUTING.md publication check at this tree, all exit 0, including the oracle-gated `parity_spine` tests (2 passed) and the full release test suite (every result line `ok`, 0 failed, 2 oracle-gated ignored). Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Raw run artifacts archived with the job in `/workspace/evidence` (report `bd49a22a…`, log `8f789c83…`); no logs committed. Workflow (CI) changes were not made. |
| 2026-09-28 (fifth pass, sync) | `b7fbfff5417de70191261758a9a37a51cad382d5` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `f0ccebef830bf6b8d99eafadf73f260a6c224873..fd9d56c74ce7500b7eaeea90a93d3bf49375d28e` (upstream pin now `73c15be0…`). `f0ccebe` is an ancestor of `fd9d56c` (merge-base verified locally). The leg is three CPU commits: `21d211e0` (`test: qualify landscape on Metal and reopen native CRT tracing`), `f290040b` (`GAP-008: report golden reference provenance separately from the kernel pin`), and `fd9d56c` (`sync: update upstream source lock and inventory through noisemaker@73c15be0`); 14 files, 1516+/27-, zero `src/` runtime files other than the generated snapshot. The snapshot changes only its revision line (sha256s `f78c661c…` at `f0ccebe`, `f54a9e00…` at `fd9d56c`); the pinned-source manifest changes only the revision field plus three upstream GPU-runtime `shaders/src` entries (webgl2.js, compiler.js, pipeline.js) the port does not execute. The parameter contract stays at `6a0af04d…`, so the Rust port requires no source change. Executed the 205-record run against the `fd9d56c` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 72.633 s. Re-executed every CONTRIBUTING.md publication check at this tree, all exit 0, including the oracle-gated `parity_spine` tests (2 passed) and the full release test suite (every result line `ok`, 0 failed, 2 oracle-gated ignored). Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Raw run artifacts archived with the job in `/workspace/evidence` (report `11d51a06…`, log `a4887fd3…`); no logs committed. Workflow (CI) changes were not made. |
| 2026-09-27 (fourth pass, sync) | `3ae887b774f00dd2a56c647a6459c942c19b8f62` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `b61b658399f18b5a93abd0020c02fff3be9630f5..f0ccebef830bf6b8d99eafadf73f260a6c224873` (upstream pin unchanged at `296e0138…`). `b61b658` is an ancestor of `f0ccebe` (merge-base verified locally). The leg is one CPU commit, `f0ccebef830b` (`GAP-007: give browser demo controls distinct accessible names`): browser-demo aria-label changes plus `test/demo-pipeline.test.js` unit tests, 7 files, 112 insertions(+), 11 deletions(-). `git diff b61b658..f0ccebe -- src/ scripts/` is empty, so the snapshot (`f78c661c…` at both ends), the pinned-source manifest, and the source-lock constants are byte-identical across the range; the parameter contract stays at `6a0af04d…`, and the Rust port (which mirrors no browser demo) requires no source change. Executed the 205-record run against the `f0ccebe` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 66.448 s. Re-executed every CONTRIBUTING.md publication check at this tree, all exit 0, including the oracle-gated `parity_spine` tests (2 passed) and the full release test suite (206 passed, 0 failed, 2 oracle-gated ignored). Rebuilt binary reproduced the previously recorded sha256 `d182360e…`. Raw artifacts: `docs/parity/parity-report-20260927-f0ccebe.json` / `parity-run-20260927-f0ccebe.log`. Workflow (CI) changes were not made. |
| 2026-09-27 (third pass) | `4092859cec814704bbd8d195e4b53492d9067325` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `dedfd07c24f80d9b0adddf912a4224ce6c1d795f..b61b658399f18b5a93abd0020c02fff3be9630f5` (upstream pin now `296e0138…`). `dedfd07` is an ancestor of `b61b658` (merge-base verified locally). The leg is two CPU source-lock sync commits: `34a0325c5392` (`noisemaker@93229933`) and `b61b658399f1` (`noisemaker@296e0138`). The snapshot changes only its revision line (sha256s `2b7587bd…` at `dedfd07`, `f78c661c…` at `b61b658`); the pinned-source manifest changes only the revision field; the pinned `shaders/effects`/`shaders/src` upstream trees are byte-identical across both pin moves (unchanged `PINNED_SOURCE_DIGEST`/`PINNED_SOURCE_MANIFEST_DIGEST` constants plus identical upstream tree objects `7ba7a60f…`/`6b791b53…` at both pins in a fresh upstream clone). The port requires no source change, and the parameter contract stays at `6a0af04d…`. Executed the 205-record run against the `b61b658` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 71.159 s. Re-executed every CONTRIBUTING.md publication check at this tree, all exit 0, including the oracle-gated `parity_spine` tests (2 passed) and the full release test suite (206 passed, 0 failed, 2 oracle-gated ignored). Raw artifacts: `docs/parity/parity-report-20260927-b61b658.json` / `parity-run-20260927-b61b658.log`. Upstream main has since advanced to `04e8582c…` (bounded `git ls-remote` observation); it belongs to the CPU sync queue. Workflow (CI) changes were not made. |
| 2026-09-27 (review) | `84b8168a2337ef846a6846d09c4cf32e0b481928` | Independent re-execution: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Reviewed the worker audit and all three closure claims. Rebuilt the binary byte-identical. Re-ran the full 205-record parity suite at a fresh `dedfd07` oracle clone: 202 byte-exact, worst max delta 0, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact at 66.916 s. Both ignored CI tests (`tests/parity_spine.rs`) pass with `NOISEMAKER_JS_CPU_DIR` set. GAP-002 evidence 37/37 hash-verified. GAP-003 evidence 26/27 hash-verified (self-listing defect only). Served kit sampled files matched 4 of 4 byte-identical. Upstream discovery refreshed to `9322993` (`1.0.194`, 11:35Z). The `1.0.193` and `1.0.194` manifests are byte-identical to the recorded 1.0.176 manifest. Workflow (CI) changes were not made. |
| 2026-09-27 | `863f64a247fa6255c0e2667b273dae9aedeea073` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the newest noisemaker-for-cpu delivery `dedfd07c24f80d9b0adddf912a4224ce6c1d795f` (upstream pin `12b4d74f…`). The leg is one CPU source-lock sync commit. `7a824744` is an ancestor of `dedfd07` (merge-base verified locally). The snapshot changes only its revision line (sha256s `7c2699a5…` at `7a824744`, `2b7587bd…` at `dedfd07`). The pinned-source manifest changes only the pin, `shaders/src/runtime/pipeline.js`, and new `preflight.js`. No `shaders/effects` entries changed. The port requires no source change, and the parameter contract stays at `6a0af04d…`. Executed the 205-record run against the `dedfd07` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 64.103 s. Rebuilt binary byte-identical (sha256 `d182360e…`). Raw artifacts: `docs/parity/parity-report-20260927-dedfd07.json` / `parity-run-20260927-dedfd07.log`. Workflow (CI) changes were not made. |
| 2026-09-27 | `347e298348fbf61993f537f2a7689e798674d7e4` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the newest noisemaker-for-cpu delivery `7a824744` (upstream pin `7443f6e6`). The pin range `6a0af04d..7443f6e6` changes upstream shader-language sources and tests. The effect catalog is unchanged, and the CPU snapshot changes only its revision line. The port requires no source change. Executed the 205-record run against the `7a824744` oracle: 202 byte-exact, 3 unsupported, 0 errors. `filter3d/flow3d` byte-exact in-run at 72.072 s. Rebuilt binary byte-identical. Byte-verified the served kit `0.1.21` (33/35 files direct, upstream MIT notice, 205-ID compat set). Recorded the newest exact-source CI (`b6c2462` head, runtime identical). Published authority advanced to `1.0.192` (`e73a44a`), manifest byte-identical to 1.0.176. Registers rewritten to the strict style rules with no factual change. Workflow (CI) changes were not made. |
| 2026-09-26 | `72c9ac0e9ec959f01cc6c4c195f7bbba5ada5cea` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `12db707f3f49..901bbd98eda0`. This completes the forced-start delivery `bfbe54764eee..901bbd98eda0`. Both range starts verified ancestors of `901bbd98eda0` locally. The single new commit is a CPU-repo docs/summary reconciliation (GAP-004 closure). It touches README, `docs/COMPLETION_GAPS.md`, `docs/CRT-PARITY.md`, `docs/EFFECTS.md`, and the CPU parity-runner comment only. The runtime diff `12db707..901bbd9` over `src/`, `scripts/upstream/` is empty, and the generated snapshot is byte-identical at both ends. The port requires no source change, and the parameter contract stays at `6a0af04d…`. Executed 205-record run against the `901bbd98eda0` oracle (202 byte-exact, 3 unsupported, 0 errors. `filter/crt` and `filter3d/flow3d` byte-exact). Raw artifacts in `docs/parity/parity-report-20260926-901bbd9.json` / `parity-run-20260926-901bbd9.log`. Hashes in [source identity](parity/source-identity.json). |
| 2026-09-26 | `7f5561908b343b1fce11c2e7e1949d4446a1e6ff` | Rendered parity re-measured at the newest delivery end: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3. | Audited the noisemaker-for-cpu range `4b590d2f7f60..12db707f3f49`, completing the forced-start delivery `bfbe54764eee..12db707f3f49`. Both new commits are CPU-repo GAP-002/GAP-003 audit records. The runtime diff `4b590d2..12db707` over `src/`, `scripts/` is empty, and the generated snapshot is byte-identical at both ends. The port requires no source change, and the parameter contract stays at `6a0af04d…`. Executed 205-record run against the `12db707f3f49` oracle (202 byte-exact, 3 unsupported, 0 errors. `filter/crt` and `filter3d/flow3d` byte-exact). Raw artifacts in `docs/parity/parity-report-20260926-12db707.json` / `parity-run-20260926-12db707.log`. Hashes in [source identity](parity/source-identity.json). |
| 2026-09-26 | `ff250e8e91e7003a918c7943146fc64d0fb123bc` (code identical to CI source `31fb9dc`) | Distribution qualification (GAP-003) measured. See section 2 and [completion gaps](COMPLETION_GAPS.md). | `cargo package --locked` in a fresh isolated copy. Crate sha256 `db74b57f…3171a` reproduced identically across two independent runs. 52-file inventory byte-compared 52/52. Six notices present. Dependency tree retained. Private installs, packaged examples, force-reinstall upgrade, uninstall, and reinstall all clean on Rust 1.85.0 and stable 1.98.1. Exact-source CI inspected at step level (0 skips, 0 failures at `31fb9dc`). Raw artifacts in `docs/parity/distribution-20260926/`. Workflow changes were not made. |
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | Rendered parity measured: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors. See section 3 and [completion gaps](COMPLETION_GAPS.md). | Updated the parity coverage table and per-effect rows from the executed 205-record run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle. The five excluded upstream IDs are reported as excluded, not as passes. Installation, host, and platform qualification remain unverified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Full qualification unverified | Created the requested maintained compatibility report. Preserved historical evidence and open gaps. |

Run: `20260924-remaining-gap-documents`. Later audits and reviews update this report with source-bound results.
