# Installed developer workflow evidence — 2026-09-26 (GAP-002)

Executed on host: AMD EPYC 7713, x86_64, Linux 6.8.0-134-generic (see
[host.txt](host.txt)). Toolchains: Rust 1.85.0 (MSRV, `Cargo.toml:5`) and stable
1.98.1 (rustc 1.98.1 (48a229cea 2026-09-01)). Source under test: git `HEAD`
`81953750ac7268ed6b46497ee3e3ea620ebb41c1` (the audited base `31fb9dc` tree).
All commands were run with `CARGO_HOME`/`RUSTUP_HOME` isolated in a private
scratch root; no global installation and no user-project changes were made.

Oracle: pinned noisemaker-for-cpu `f2eb495d70abcb74e3632e7a652a4f83e4f3b11e`
(immutable `git checkout` of that SHA; the same oracle revision as the GAP-001
rendered-parity evidence). JavaScript is an offline test oracle only.

## 0. Version matrix (required check: test minimum and current supported versions)

| Step | Rust 1.85.0 | stable 1.98.1 |
|---|---|---|
| `cargo check --workspace --all-targets --locked` | exit 0 ([check-1.85.0.log](check-1.85.0.log)) | exit 0 ([check-stable.log](check-stable.log)) |
| `cargo test --locked --release` | exit 0, all suites pass ([test-1.85.0.log](test-1.85.0.log)) | exit 0, all suites pass ([test-stable.log](test-stable.log)) |

## 1. Package, private install, uninstall (both toolchains)

- `git archive HEAD | tar -x` into an isolated copy, then
  `cargo +stable package --locked`: exit 0, artifact
  `noisemaker-for-rust-0.1.0.crate` ([package.log](package.log)).
- sha256 (crate):
  `eae2d3831a4c14a911562e6a52d607206c475be75bc7799f2f0efedb6a936ca5`
  (see [hashes.md](hashes.md)).
- `cargo +<T> install --locked --root <private root> --path <unpacked crate>`:
  exit 0 for 1.85.0 ([install-1.85.0.log](install-1.85.0.log),
  [reinstall-1.85.0.log](reinstall-1.85.0.log)) and stable
  ([install-stable.log](install-stable.log),
  [reinstall-stable.log](reinstall-stable.log)).
  Installed binary reports `noisemaker-rs 0.1.0` on both toolchains.
- `cargo +<T> uninstall --root <private root> noisemaker-for-rust`: exit 0,
  private bin directory is empty afterwards ([uninstall-1.85.0.log](uninstall-1.85.0.log),
  [uninstall-stable.log](uninstall-stable.log)). Cleanup result: removed.

## 2. Meaningful output (installed binary, both toolchains)

- `noisemaker-rs effects`: 205 sorted `ID<TAB>KIND` records
  ([effects-1.85.0.log](effects-1.85.0.log), [effects-stable.log](effects-stable.log)).
- `generate synth/gradient --width 64 --height 64 --seed 1`: exit 0 →
  [gen-1.85.0.png](gen-1.85.0.png) / [gen-stable.png](gen-stable.png).
- DSL chain via `run -`
  (`search synth,classicNoisedeck,filter; gradient().kaleido(sides: 8).vignette().write(o0)`):
  exit 0 → [dsl-1.85.0.png](dsl-1.85.0.png) / [dsl-stable.png](dsl-stable.png).
- PNG input: a 32x32 gradient ([input-32x32.png](input-32x32.png)) rendered and
  `apply filter/blur` at input dimensions: exit 0 →
  [applied-1.85.0.png](applied-1.85.0.png) / [applied-stable.png](applied-stable.png).
- sha256 of every retained output is in [hashes.md](hashes.md).

## 3. Error diagnostics and recovery (installed binary, both toolchains)

- Invalid parameter value `--param colorCount=99`: exit 1 with
  `noisemaker-rs: <cli>:1:47: Parameter "colorCount" must be at most 4`
  ([recovery-1.85.0.log](recovery-1.85.0.log), [recovery-stable.log](recovery-stable.log)).
- Unknown parameter `--param bogus=1`: exit 1 with
  `noisemaker-rs: Unknown parameter "bogus" for synth/gradient`.
- Recovery: a corrected run `--param colorCount=3` exits 0 and writes a valid
  PNG → [recovered-1.85.0.png](recovered-1.85.0.png) /
  [recovered-stable.png](recovered-stable.png) (identical bytes on both
  toolchains; sha256 in [hashes.md](hashes.md)).

## 4. Read-only output destination (file preservation)

With the scratch directory mode 0555, `generate ... --output <dir>/pre.png`
fails with `noisemaker-rs: Permission denied (os error 13)` (exit 1) and the
pre-existing destination file (`PRE-EXISTING` bytes) is unchanged
([readonly-1.85.0.log](readonly-1.85.0.log), [readonly-stable.log](readonly-stable.log)).

## 5. Cancellation and file preservation

A long render (`synth/noise` 1024x1024, `octaves=8`) was started with the
destination pre-seeded with `KEEP-ME` and SIGTERM'd after 3 s. Result recorded
in [cancel-1.85.0.log](cancel-1.85.0.log) / [cancel-stable.log](cancel-stable.log):
the pre-existing destination is still `KEEP-ME`
([cancel-dest-preserved-1.85.0.png](cancel-dest-preserved-1.85.0.png),
[cancel-dest-preserved-stable.png](cancel-dest-preserved-stable.png)) and no
temporary or partial files remain. This matches README.md:102-104 (destinations
are created through exclusive unpredictable sibling temporary files and renamed
only after successful encoding).

## 6. Immutable-oracle pixel comparison (installed binary)

- stable installed binary, full catalog:
  `python3 scripts/parity.py --rust <installed> --js <pinned oracle> --timeout 120`
  ([parity-stable.log](parity-stable.log), report
  [parity-report-stable.json](parity-report-stable.json)): catalog=205,
  compared=202, byte-exact=202 at tolerance 0, unsupported=3 (the stable
  overlay-interface exclusions `filter/fibers`, `filter/scratches`,
  `filter/strayHair`), errors=0, failed=0 — identical denominator and result to
  the GAP-001 run.
- 1.85.0 installed binary, focused:
  `--only synth/gradient --only classicNoisedeck/bitEffects`
  ([parity-1.85.log](parity-1.85.log), report
  [parity-report-1.85.json](parity-report-1.85.json)): 2 compared, 2 byte-exact
  at tolerance 0, 0 errors.

## 7. Host, GPU, licensing, and input requirements

- Host: single Linux x86_64 host, 6 vCPU, identified in [host.txt](host.txt).
- GPU: none used; the renderer is CPU-only by design (README.md:14-19). No GPU
  path exists in the CLI.
- Licensing: package is MIT ([LICENSE](../../../LICENSE)); the packaged crate
  carries the MIT license file (verified by `cargo package` listing
  [package.log](package.log)). Dependency licenses are fetched from crates.io
  per `Cargo.lock`; the package build succeeded with `--locked`.
- Input requirements: the CLI accepts catalog effect IDs/DSL programs and
  optional PNG inputs via `--input`/`--texture`; external-texture effects
  require a PNG binding (README.md:93-94, 116). Both the generator path and the
  PNG-input path were exercised above.

## 8. Unavailable platforms (kept explicit)

- macOS and Windows were not tested on this run; no darwin/windows toolchain
  was exercised. The measured matrix is Linux x86_64 only.
- GPU rendering paths: not applicable/not measured (CPU-only port).
- Platform matrix qualification remains GAP-003 scope.

## 9. Limitations

- `animate` was not exercised (requires `ffmpeg`; the CLI's documented
  behavior without ffmpeg is to fail with installation guidance, which was not
  separately re-verified here).
- The 1.85.0 oracle comparison is focused (2 records); the full-catalog
  byte-exact comparison was executed with the stable installed binary.
- Rendering is deterministic and single-threaded; timings are not qualification
  results.
