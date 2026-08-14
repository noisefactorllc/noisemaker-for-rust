# {{NM_PROGRAM_NAME}}

Your program, exported from Noisedeck as a package that renders it **on the CPU**, in Rust. No GPU,
no WebGL, no runtime shader compiler: `engine/` is the whole renderer as a Cargo crate, and the
compiler turns what would normally be shader code into native machine code that walks the image one
pixel at a time.

That makes this the fastest CPU export here and the one with a build step. A GPU draws a frame in
milliseconds because it colors thousands of pixels at once; this colors them in sequence, but in
compiled native code rather than an interpreter.

## Run it

You need a **Rust toolchain, 1.85 or newer** — the crate is edition 2024 and declares 1.85 as its
minimum supported version, so an older `rustc` refuses it by name rather than failing somewhere
confusing. <https://rustup.rs> installs one. Unzip this folder, open a terminal in it, and start
small:

```sh
cargo run --release --manifest-path engine/Cargo.toml -- run program.dsl --width 64 --height 64 --output out.png
```

That writes a 64×64 `out.png` beside your program, which is enough to prove the export works. Then
scale up:

```sh
cargo run --release --manifest-path engine/Cargo.toml -- run program.dsl --width 512 --height 512 --output art.png
```

The `--` separates Cargo's arguments from the renderer's: everything after it goes to the program.
Paths are resolved from the directory you run in, not from `engine/`, so `program.dsl` and
`out.png` land here beside this README.

**The first build downloads crates.** The renderer depends on five published crates —
`clap`, `png`, `serde`, `serde_json` and `thiserror` — and Cargo fetches them, and their
dependencies, from crates.io the first time you build. `engine/Cargo.lock` ships with this export,
so the versions are pinned to exactly what this kit was built against rather than resolved fresh.
Once that build finishes, everything is on your disk: **rendering itself never touches the network**,
and every later run reuses the compiled binary. Expect the first `--release` build to take a couple
of minutes; the ones after it are instant.

Prefer a plain command to a Cargo invocation? Install the binary once and call it directly:

```sh
cargo install --path engine
noisemaker-rs run program.dsl --width 512 --height 512 --output art.png
```

Rendering time grows with the pixel count, and how far it grows depends entirely on what your
program does, so raise the size in steps rather than jumping to a poster.

Useful options: `--seed N` picks the deterministic seed, `--time N` the normalized time (some
effects animate), and `--input file.png` binds an image for programs that sample one.
`cargo run --release --manifest-path engine/Cargo.toml -- --help` lists the rest.

## What's inside

| Path | What it is |
| --- | --- |
| `program.dsl` | Your program's source, exactly as Noisedeck had it. |
| `engine/Cargo.toml` | The renderer's crate manifest. This is the file your command points at. |
| `engine/Cargo.lock` | The exact dependency versions this export was built against. |
| `engine/src/` | The engine: DSL parser, effect catalog, compiled shader IR, and the pixel kernels. |
| `noisedeck-export.json` | What was exported, when, against which engine build. |
| `LICENSES/` | Licenses for everything shipped here. |

`engine/` is self-contained: copy it somewhere else, point `--manifest-path` at it there, and the
same command works. Cargo writes its build output into `engine/target/`, which you can delete at any
time — the next build just takes longer.

## The engine

The port ships inside this export as source, so it builds and runs offline once its dependencies are
fetched. It is also a normal crate — add `engine/` as a path dependency and
`use noisemaker_cpu::{CpuRenderer, RenderOptions};` gives you the same renderer from your own Rust
code, which <https://github.com/noisefactorllc/noisemaker-for-rust> documents.

Noisedeck exported this program against Noisemaker `{{NM_ENGINE_VERSION}}`. The Rust port is a
second implementation of that engine rather than the same code, so expect small differences from
what the app showed you.

## Editing it

Replace `program.dsl` with anything the Noisemaker language accepts, as long as its effects are in
the supported set below, and run the same command again — the engine is already compiled, so only
the render repeats. To produce several variations, call `CpuRenderer` in a loop of your own rather
than paying process startup each time.

## Effects used by this program

{{NM_EFFECT_LIST}}

## What this port cannot render

Five effects from the upstream catalog: `synth/roll`, `synth/scope` and `synth/spectrum`, which react
to live audio, and `render/meshLoader` and `render/meshRender`, which need a mesh pipeline. Everything
else in the catalog renders here, and
`cargo run --release --manifest-path engine/Cargo.toml -- effects` lists exactly what the engine in
this folder carries.

To check an edited `program.dsl` against a different build of this port, put it back into Noisedeck
and open the export dialog with Rust selected: it marks any effect the port cannot render before you
export again.

## License

The Noisemaker engine and the Rust port are MIT licensed; see `LICENSES/`. The crates Cargo fetches
carry their own licenses, which `cargo tree` and each crate's page on crates.io state. Your program
and the imagery it renders are yours.
