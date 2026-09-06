# {{NM_PROGRAM_NAME}}

This Rust package exports your Noisedeck program to render **on the CPU**. It requires no GPU, WebGL, or runtime shader compiler. `engine/` contains the whole renderer as a Cargo crate. The compiler converts the shader code into native machine code that processes the image one pixel at a time.

This export requires a build step before rendering. A GPU draws a frame in milliseconds because it colors thousands of pixels at once. This renderer colors them in sequence with compiled native code instead of an interpreter.

## Run it

You need a **Rust toolchain, 1.85 or newer**. The crate uses edition 2024 and declares 1.85 as its minimum supported version. An older `rustc` reports this version requirement and refuses to compile the crate. <https://rustup.rs> installs a toolchain.

Unzip this folder. Open a terminal in it. Start with a small image:

```sh
cargo run --release --manifest-path engine/Cargo.toml -- run program.dsl --width 64 --height 64 --output out.png
```

The command writes a 64×64 `out.png` beside your program. This checks that the export works. Then increase the output size:

```sh
cargo run --release --manifest-path engine/Cargo.toml -- run program.dsl --width 512 --height 512 --output art.png
```

The `--` separates Cargo's arguments from the renderer's: everything after it goes to the program.
The renderer resolves paths from your working directory, not from `engine/`. In these commands, `program.dsl` and `out.png` are beside this README.

**The first build downloads crates.** The renderer depends on five published crates: `clap`, `png`, `serde`, `serde_json` and `thiserror`. Cargo fetches them and their dependencies from crates.io during the first build. The export includes `engine/Cargo.lock`, which pins the exact dependency versions used to build this kit.
Once the build finishes, everything is on your disk. **Rendering itself never uses the network**. Later runs can reuse the compiled binary. Expect the first `--release` build to take a couple of minutes.

To use a plain command, install the binary once. Then call it directly:

```sh
cargo install --path engine
noisemaker-rs run program.dsl --width 512 --height 512 --output art.png
```

Rendering time increases with the pixel count. The amount of increase depends entirely on the program. Increase the size gradually.

Useful options:

- `--seed N` selects the deterministic seed.
- `--time N` sets the normalized time for effects that animate.
- `--input file.png` binds an image for programs that sample one.

`cargo run --release --manifest-path engine/Cargo.toml -- --help` lists the rest.

## What's inside

| Path | What it is |
| --- | --- |
| `program.dsl` | Your program's source, exactly as it was in Noisedeck. |
| `engine/Cargo.toml` | The renderer's crate manifest. This is the file your command points at. |
| `engine/Cargo.lock` | The exact dependency versions this export was built against. |
| `engine/src/` | The engine: DSL parser, effect catalog, compiled shader IR, and the pixel kernels. |
| `noisedeck-export.json` | The exported content, export time, and engine build. |
| `LICENSES/` | Licenses for everything shipped here. |

`engine/` is self-contained. If you copy it elsewhere and update `--manifest-path` to that location, the same command works. Cargo writes its build output into `engine/target/`. You can delete that directory at any time, but the next build takes longer.

## The engine

The export includes the port as source. It builds and runs offline after you fetch its dependencies. It is also a normal crate. Add `engine/` as a path dependency. `use noisemaker_cpu::{CpuRenderer, RenderOptions};` imports the same renderer into your Rust code. <https://github.com/noisefactorllc/noisemaker-for-rust> documents the API.

Noisedeck exported this program against Noisemaker `{{NM_ENGINE_VERSION}}`. The Rust port is a separate implementation of that engine. Expect small differences from the app output.

## Editing it

Replace `program.dsl` with a Noisemaker program that uses only the supported effects listed below. Run the same command again. The engine is already compiled, so only the render repeats. To produce several variations, call `CpuRenderer` in a loop of your own rather
than paying process startup each time.

## Effects used by this program

{{NM_EFFECT_LIST}}

## What this port cannot render

This port cannot render five effects from the upstream catalog:

- `synth/roll`, `synth/scope` and `synth/spectrum` react to live audio.
- `render/meshLoader` and `render/meshRender` need a mesh pipeline.

Everything else in the catalog renders here.
`cargo run --release --manifest-path engine/Cargo.toml -- effects` lists exactly which effects this engine contains.

To check an edited `program.dsl` against a different build of this port:

1. Import the program into Noisedeck.
2. Open the export dialog.
3. Select Rust.

Before you export again, the dialog marks any effect the port cannot render.

## License

The Noisemaker engine and the Rust port are MIT licensed. See `LICENSES/`. The crates Cargo fetches
carry their own licenses, which `cargo tree` and each crate's page on crates.io state. Your program
and the imagery it renders are yours.
