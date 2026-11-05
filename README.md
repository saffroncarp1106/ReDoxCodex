# CodexRustReWrite
A front-end rewrite for rust because 1.2g of ram for a basic ui window is ridiculous.


## Run

Double-click `Launch Codex Rust.cmd` to build and open this copy. Rust and the Visual Studio C++ build tools are required.

## Message formatting

Both Codex and Gemini layouts now use the shared Markdown/math renderer in `src/ui/markdown.rs`. Equations, fractions, powers, subscripts, logarithms, headings, links, lists, and tables are formatted. Inline math accepts `$...$` or `\(...\)`; display math accepts `$$...$$` or `\[...\]`. Code blocks retain literal source and indentation, including HTML and LaTeX code examples. Right-click an equation to copy its LaTeX. Unsupported equations remain readable as source with an explanation on hover.

The earlier renderer is preserved under `temp/superseded-renderer/`. Missing verification files from the downloaded archive have been restored under `tests/`.

## Verification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```

Native formatting preview (does not save changes to chats or preferences):

```powershell
New-Item -ItemType Directory -Path temp/screenshots -Force
.\target\debug\codex-rust.exe --screenshot temp/screenshots/formatting.png --format-preview --startup-preview=4.2
```
