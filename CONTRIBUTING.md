# Contributing to Noisemaker for Rust

Thanks for your interest in contributing.

## Getting set up

The declared minimum supported Rust version is 1.85. Install that compiler and
the current stable formatting and lint components:

```sh
rustup toolchain install 1.85.0 --profile minimal
rustup toolchain install stable --profile minimal --component rustfmt clippy
```

Run the same checks used for publication before submitting a change:

```sh
cargo +1.85.0 check --all-targets --all-features
cargo +stable fmt --all -- --check
cargo +stable clippy --all-targets --all-features -- -D warnings
cargo +stable test --all-targets --all-features
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'
PYTHONDONTWRITEBYTECODE=1 python3 scripts/generate_bundle.py --check
cargo +stable build --release --locked
cargo +stable package --locked
```

If a sibling JavaScript CPU checkout is available, run the focused cross-port
test explicitly:

```sh
NOISEMAKER_JS_CPU_DIR=../noisemaker-for-cpu \
  cargo +stable test --test parity_spine -- --ignored
```

The complete Rust test suite exercises all 205 eligible effects and all 456
non-null compile-time choices, so it can take several minutes. The JavaScript
CPU port is used only as an offline maintenance oracle by `scripts/parity.py`;
it must never become a production dependency.

Keep changes focused and include regression coverage for behavior changes. If
generated catalog or typed-IR files need to change, update their canonical
inputs and regenerate them instead of editing generated output by hand. Never
commit credentials, private source archives, built binaries, rendered media, or
Python cache files.

## Reporting issues

Open a GitHub issue with the operating system, Rust version, project commit,
and exact reproduction steps. Report suspected vulnerabilities privately using
[SECURITY.md](SECURITY.md).
