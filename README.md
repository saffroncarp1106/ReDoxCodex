# CodexRustReWrite
A front-end rewrite for rust because 1.2g of ram for a basic ui window is ridiculous.


## Run

Double-click `Launch Codex Rust.cmd` to build and open this copy. Rust and the Visual Studio C++ build tools are required.

## Message formatting

Both Codex and Gemini layouts now use the shared Markdown/math renderer in `src/ui/markdown.rs`. Equations, fractions, powers, subscripts, logarithms, headings, links, lists, and tables are formatted. Inline math accepts `$...$` or `\(...\)`; display math accepts `$$...$$` or `\[...\]`. Code blocks retain literal source and indentation, including HTML and LaTeX code examples. Right-click an equation to copy its LaTeX. Unsupported equations remain readable as source with an explanation on hover.


## Verification

```powershell
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --locked
```
