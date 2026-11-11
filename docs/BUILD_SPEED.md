# Build speed measurements

The benchmark builds the actual redox executable, including linking, with Rust
1.88.0 on Windows. Each configuration gets a new target directory. Downloaded
registry sources are already available; this measures empty Cargo build caches,
not network download time or an empty operating-system file cache.

Run from the project root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/measure-build.ps1 -Label unique-name
```

The script refuses to reuse a target directory. It saves the manifest, lockfile,
toolchain, raw logs and wall-clock results under `temp/build-speed/<label>/`.
Build artifacts and Cargo timing HTML stay under `target/speed-<label>/`.
Each run measures an initial build and then an unchanged rebuild, sequentially.
This is a single-machine comparison; repeat in alternating order to estimate
variance. It does not measure application frame rate or model inference speed.

## Changes under comparison

- Remove reqwest's unused `stream` feature. The app uses `Response::chunk`, which
  is available without that feature in the pinned reqwest source. Keep JSON and
  Rustls support and verify streamed responses using mock transport tests.
- Use line tables for development debug information instead of full symbols.
  Backtrace file/line information remains; inspecting local variables in a native
  debugger requires restoring full debug info, for example with
  `CARGO_PROFILE_DEV_DEBUG=2`. That override changes the build cache key.
- Keep release optimization, incremental compilation defaults, supported image
  formats, accessibility and rendering features intact.

## Results (2026-11-10)

| Measurement | Original | Modified | Reduction |
| --- | ---: | ---: | ---: |
| Full debug build, empty target | 177.083 s | 81.373 s | 54.0% |
| Unchanged debug rebuild | 1.326 s | 0.825 s | 37.8% |
| Application PDB size | 200,347,648 bytes | 87,715,840 bytes | 56.2% |

Completed runs: `baseline-20261110a` and `optimized-20261110b`.
The first optimized attempt (`optimized-20261110a`) was interrupted and excluded;
its partial artifacts and logs are preserved. The completed optimized build used
a different empty directory. Both completed builds exited successfully.

These are single samples. The later run may benefit from operating-system file
caching and different system load. The unchanged result is below two seconds and
especially sensitive to noise. Both edits were measured together; the numbers
do not isolate their individual effects. Incremental source-edit rebuild time,
release build speed, and application runtime performance were not measured.

## Validation

- Both full executable builds and unchanged rebuilds passed with `--locked`.
- `cargo check --locked --all-targets --features design-export` passed using the
  optimized target directory (35.15 seconds), with the same seven unused-code
  warnings seen before these edits. This check is not a build-speed comparison.
- Mock Ollama/project-tool tests: 17 passed, 2 live-model tests ignored. The desktop
  input test was excluded. This covers fragmented streaming, retries, cancellation,
  context rollover and continued tool execution without the reqwest stream feature.
- The lockfile removed futures-sink, tokio-util and wasm-streams; existing package
  versions were retained. The remaining dependency graph still supplies futures-util.

Test command (using the completed optimized cache):

```powershell
cargo test --locked --lib ollama:: --target-dir target/speed-optimized-20261110b -- --skip folderless_ollama_desktop_input_uses_shared_approval
```
