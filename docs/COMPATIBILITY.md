# noisemaker-for-rust: compatibility report

## 1. Source and authority revisions

Daily review: 2026-09-25. Current inspected source: [`25f1340c6b087d648bb5b4249b2f4d94be5d5c02`](https://github.com/noisefactorllc/noisemaker-for-rust/commit/25f1340c6b087d648bb5b4249b2f4d94be5d5c02).
Full rendered parity at this SHA: measured (section 3). Installation, host, and platform qualification remain unverified. No release approval follows from this review.
Current upstream discovery: `bbdeb56c4b75cf33379766c3e87b0f5a18bcbba8`. Published Noisemaker authority: `1.0.179`, source `fca611fd8f91424661d4e531d39313d24ea21134`, 210 effect IDs.
The observations below retain their original source and authority identities. They do not qualify later updates.
Current served kit: `0.1.16`, source `9a043c4ea38766d356fcd20c0d838149558cf0fd`. [Retrieved inventory and hashes](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/current-served-inventories.json). Artifact identity does not establish host qualification.

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
| Source-level checks | verified | The generated-bundle check exited 0. This is a reproducibility check, not a render or Cargo package installation test. |
| Actual host rendering | unverified | No new complete native or browser workflow qualified by this report. |
| Minimum and current host versions | unverified | Declared requirements are not a tested version matrix. |
| Supported operating systems and backends | unverified | This pass does not establish Windows, Linux, and macOS coverage. |
| Installed package and first useful result | unverified | Complete isolated installation was not qualified for this source. |
| Parameters, external inputs, state, and chains | unverified | Full current-authority combinations remain unmeasured. |
| Invalid input and recovery | unverified | Unit checks do not establish every installed public entry point. |
| Upgrade, removal, and resource cleanup | unverified | Prior defects and missing workflows remain in the gap register. |
| Accessibility of provided controls | unverified | Keyboard, focus, labels, and diagnostics need host observations where applicable. |
| Release readiness | blocked | Full parity, installation, host, and artifact evidence remain incomplete. |

## 3. Parity coverage

### Full render suite, 2026-09-25

One 205-record parity run executed the complete catalog through both CLIs against identified revisions. Port source: `25f1340c6b087d648bb5b4249b2f4d94be5d5c02`. JavaScript CPU oracle: noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` (immutable authority input, resolved by pinned checkout). CPU revision: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic. Parameters: size 8, time 0.25, seed 1, tolerance 0, per-command timeout 120 s. Result: 202 compared byte-exact (max delta 0), 3 unsupported with the stable overlay-interface reason (`filter/fibers`, `filter/scratches`, `filter/strayHair`), 0 errors. The five upstream effects excluded from this standalone CPU port remain excluded and are reported as excluded, not as passes. The harness default 30-second timeout first reported `filter3d/flow3d` as a timeout failure; the retained 120-second retry compares byte-exact (rust 66.841 s). The default-timeout failure is retained in this record, not discarded. Raw output, the machine-readable report, the retry record, and exact source hashes: [parity report](parity/parity-report-20260925.json), [raw run log](parity/parity-run-20260925.log), [flow3d retry](parity/parity-flow3d-120s-retry.json), [source identity and hashes](parity/source-identity.json).

Remaining coverage limits are unchanged by this run: parameter sweeps beyond the documented defaults, stateful sequences beyond the harness chain shape, installed CLI qualification (GAP-002), and platform matrices (GAP-003) remain unmeasured. No skip or tolerated difference counts as exact parity.

### Earlier measurements

Full parity requires complete applicable coverage with no skips or missing cases.
Historical NEAR, CHAOS, and tolerated differences do not count as strict equality.
The existing numerical contracts remain separate from exact comparison. This report does not change tolerances or goldens.
Unknown values mean `not measured`, never zero.

| Gate | Expected cases | Executed | Strict passes | Failures | Skips | Status |
|---|---|---|---|---|---|---|
| Current full render suite | 205 | 205 (202 compared, 3 unsupported with reasons) | 202 (byte-exact) | 0 | 0 | 2026-09-25 run at `25f1340` vs noisemaker-for-cpu `f2eb495d`, tolerance 0; see section 3 |

Earlier served compatibility inventory declares 205 effect IDs. Declaration does not establish execution or parity.
IDs absent from the served declaration: `render/meshLoader`, `render/meshRender`, `synth/roll`, `synth/scope`, `synth/spectrum`.
Missing effects remain visible toward the full-parity goal. Contract exclusions do not become successful tests.

Current served declaration: 205 effect IDs. This inventory is not evidence of execution. The declaration column below reflects kit `0.1.16`.

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
| `filter/fibers` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag; the Rust parity CLI path intentionally uses Initial semantics) |
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
| `filter/scratches` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag; the Rust parity CLI path intentionally uses Initial semantics) |
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
| `filter/strayHair` | yes | unsupported (the JavaScript CLI exposes Ready one-shot overlay generation but no Initial one-shot flag; the Rust parity CLI path intentionally uses Initial semantics) |
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

[Bounded test evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents/rust-tests.json). [Exact-source Actions](https://github.com/noisefactorllc/noisemaker-for-rust/actions?query=head_sha%3A2ef1cc4179f5163023c26e785f533d07cf699fb4).
[This run evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/evidence-20260924-remaining-gap-documents) retains commands, exit codes, source identities, and distribution metadata.
Official ecosystem reference: [Current Cargo Book, accessed 2026-09-24](https://doc.rust-lang.org/cargo/reference/publishing.html).
Source CI, export dispatch, artifact delivery, and rendered parity are separate evidence dimensions.
A successful dispatch or unit-test summary does not establish a full rendered gate.

## 5. Open compatibility limits

Next bounded check: Run cargo package with verification. Install the resulting crate in an isolated CARGO_HOME. Execute the documented noisemaker-rs PNG example on Rust 1.85 and the current supported toolchain. Resolve both ignored doctests without reducing the denominator. The rendered-parity half of the earlier check is executed (section 3); installation remains open.
See the stable entries in [completion gaps](COMPLETION_GAPS.md).

See [GAP-001 and the complete gap register](COMPLETION_GAPS.md#4-known-gaps) for evidence, dependencies, and acceptance criteria.

1. Reconcile the current authority and complete case inventory, including parameters, inputs, stateful frames, and host versions. Done for the documented harness denominator (section 3); broader sweeps remain open.
2. Run the existing actual-renderer suite without skip options. Record every missing, failed, refused, or timed-out case. Executed 2026-09-25 (section 3); parameter sweeps and stateful sequences beyond the harness shape remain open.
3. Verify installation, useful output, errors, recovery, upgrades, and removal with the actual distribution.
4. Inspect exact-source CI and retain artifact hashes. Keep unresolved qualification failed or unverified.

All eligible ports have equal priority. Full parity and zero skipped cases remain the goal.
Implementation corrections remain with the separate job. This report does not advance the parity checkpoint.

## 6. History

2026-09-25 daily review at `9a043c4ea38766d356fcd20c0d838149558cf0fd`: source freshness and bounded evidence reviewed. Open qualification limits retained. [Retained review evidence](/Users/alex/.codex/automations/noisemaker-port-completion-audit/review-20260925-053200/rust-ci-36082527571.log). No new closure claimed.

| Date | Source | Result | Change |
|---|---|---|---|
| 2026-09-25 | `25f1340c6b087d648bb5b4249b2f4d94be5d5c02` | Rendered parity measured: 202/205 compared byte-exact at tolerance 0, 3 unsupported with reasons, 0 errors; see section 3 and [completion gaps](COMPLETION_GAPS.md). | Updated the parity coverage table and per-effect rows from the executed 205-record run against the pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e` oracle. The five excluded upstream IDs are reported as excluded, not as passes. Installation, host, and platform qualification remain unverified. |
| 2026-09-24 | `2ef1cc4179f5163023c26e785f533d07cf699fb4` | Full qualification unverified | Created the requested maintained compatibility report. Preserved historical evidence and open gaps. |

Run: `20260924-remaining-gap-documents`. Later audits and reviews update this report with source-bound results.
