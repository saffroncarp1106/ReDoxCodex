# CodexRustReWrite
A front-end rewrite with rust because 1.2g of ram for a basic ui window is ridiculous.


## Run

### Windows

Double-click `Launch Codex Rust.cmd` to build and open this copy. Rust and the Visual Studio C++ build tools are required.

### macOS

Install Rust using rustup and the Xcode command line tools (`xcode-select --install`), then run from the repository:

```sh
cargo run --locked
```

The repository selects Rust 1.88.0 automatically through rustup. Codex CLI is required: the app looks for `codex` on `PATH`, then in `Codex.app` or `ChatGPT.app` under `/Applications` and `~/Applications`. You can also select the CLI in Settings > Advanced.

## Message formatting

Both Codex and Gemini layouts now use the shared Markdown/math renderer in `src/ui/markdown.rs`. Equations, fractions, powers, subscripts, logarithms, headings, links, lists, and tables are formatted. Inline math accepts `$...$` or `\(...\)`; display math accepts `$$...$$` or `\[...\]`. Code blocks retain literal source and indentation, including HTML and LaTeX code examples. Right-click an equation to copy its LaTeX. Unsupported equations remain readable as source with an explanation on hover.


## Verification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```


## v0.1.1 missing-file repair

The formatting preview is now bundled at `src/ui/previews/formatting.md`. Normal app builds no longer depend on a file under `tests/`. The referenced UI regression tests are included in `tests/ui/`; include that folder when distributing source for `cargo test` or `cargo fmt`.

<<<<<<< HEAD
Build with `cargo build --locked`, or double-click `Launch Codex Rust.cmd`. The source archive must include `src/`, `tests/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, and the launcher. Generated `target/` and `temp/` directories are unnecessary for a source download.
=======
Build with `cargo build --locked`, or double-click `Launch Codex Rust.cmd`. The source archive must include `src/`, `tests/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, and the launcher. Generated `target/` and `temp/` directories are unnecessary for a source download.
>>>>>>> d660e872fe0f67647af2f8c07d727969a5d4ecbb
