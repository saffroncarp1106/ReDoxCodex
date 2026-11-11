# Build and task execution review

Reviewed 2026-11-10. Findings below are recommendations unless marked implemented.
Existing uncommitted application changes are preserved. Timings are observations
from a cached dependency graph, not clean-build benchmarks or promised savings.

## First pass

1. **Separate Studio runtime and editor dependencies.** `../ReCreation/src/lib.rs`
   unconditionally declares editor, originals, and simulation modules. Its MCP
   feature is disabled here, but editor code and embedded originals still compile.
   Extract a runtime library or gate editor-only modules after tracing dependencies.
   Studio checking took 14.54 seconds in the first pass.
2. **Use debug builds for edit/run cycles.** `Launch ReDox.cmd` defaults to release
   and `Cargo.toml` enables release ThinLTO. The existing `--debug` option avoids
   release optimization work. Preserve the optimized distribution profile.
3. **Share the application between executables.** `src/main.rs` and
   `src/export_designs.rs` both declare `mod app`. All-target builds check the
   application separately for these targets. Consider a shared app library with
   thin executable entry points; retain the transport/UI directory boundary.
4. **Support long-running worker commands.** `src/ollama_tools.rs::run_command`
   times out at 30 seconds. The all-target check took 37.03 seconds. Use command
   sessions with polling, cancellation, and terminal status instead of relying on
   a fixed short timeout for build tasks.
5. **Make project instructions explicit in the worker prompt.** The instruction
   to read AGENTS.md appears in the list_files tool description, but not the
   main worker prompt. Include applicable root/nested instructions before edits;
   preserve user scope and access permissions. State focused verification and
   preservation of target/ explicitly in repository workflow guidance.
6. **Reconcile README launch and verification instructions.** The updater section
   claims the launcher uses existing binaries, although it rebuilds. Include an
   explicit design-export feature check in the verification matrix.

## Second pass: task lifecycle

7. **Do not mark every answer as completed work.** In `src/ollama.rs`, any complete
   response without tool calls sets the checkpoint status to Complete. A question,
   plan, or report of a blocker can therefore become a completed task. Distinguish
   response termination from task completion and preserve unfinished work. Verify
   using mock replies containing a clarification, a plan, and a successful result.
8. **Drain command output after the display limit.** In
   `src/ollama_tools.rs::read_output`, `take(64 * 1024)` stops reading the pipe at
   the limit, then drops it. A verbose compiler can encounter a broken output pipe
   instead of completing normally. Continue draining while retaining only bounded
   output; label truncation and preserve exit status. Verify with a child producing
   more than 64 KiB on both streams and then exiting successfully.
9. **Avoid synchronous full checkpoint rewrites on the async worker.**
   `src/ollama_state.rs::record` calls synchronous `save` before and after actions.
   Each save joins and serializes the accumulated history, repeats it in Markdown
   and JSON, and replaces the file. Long transcripts therefore increase allocation
   and disk work per action. Consider an append journal plus atomic snapshots, or
   awaited blocking I/O with bounded snapshots. Preserve pre-action durability.
10. **Avoid cloning images just to estimate context size.**
    `src/ollama_state.rs::prompt_size` clones each JSON message before removing
    images. Base64 images can be large and this runs repeatedly in the task loop.
    Count borrowed non-image fields and apply the image allowance without cloning
    payloads. Verify equivalent estimates for text, tools, Unicode, and images.

## Third pass: dependency and tool scope

11. **Implemented: remove the unused reqwest stream feature.** The source
    uses `Response::chunk`, not `bytes_stream` or `Body::wrap_stream`. In the pinned
    reqwest 0.12.28 source, chunk is not feature-gated; the other APIs are.
    `cargo tree -e features -i tokio-util` shows reqwest as its only parent.
    Removing stream should eliminate this dependency on the current Windows graph.
    Keep JSON and Rustls support. Validate all targets and the mock streaming tests;
    removing the feature does not mean removing streamed Ollama responses.
12. **Make winresource a Windows host build dependency.** It is currently in the
    unconditional build-dependencies table. The environment test in build.rs is a
    runtime branch, so non-Windows hosts still compile the build dependency.
    For native-platform builds, use a Windows-targeted build dependency and compile
    time gating in build.rs. Define cross-compilation support separately: a build
    script runs on the host, whereas CARGO_CFG_WINDOWS describes the target.
13. **Separate the presence utility from the application's dependency graph.**
    src/presence.rs only uses discord-rich-presence and std, but its package owns
    all GUI, HTTP, image, and Studio dependencies. A small workspace package would
    make standalone presence development cheaper on a cold build. Keep the one
    application launcher and update its documented cargo command if moved.
14. **Add ranged reads and targeted edits for workers.** The worker tools only
    read or replace complete files (128 KiB cap). app.rs alone is about 56 KiB,
    exceeding the conservative input byte budget of the supported 32k context.
    A complete read forces compaction and source excerpt loss; a complete rewrite
    can exceed the output allowance. Add line ranges/search and patch operations
    with stale-content checks, backups, and the existing approvals. This reduces
    repeated model work and makes small edits to large Rust modules practical.
15. **Expose directory truncation and pagination.** list_files takes the first
    512 directory entries before sorting, silently omitting the rest. Repeated
    calls have no cursor and cannot reliably discover omitted files. Sort a stable
    listing, return a truncation indicator and page token, and test >512 entries.

## Fourth pass: build boundaries and continuation

16. **Decouple headless transport tests from eframe.** `src/run.rs` imports
    eframe::egui solely for the Context supplied to Running::start and repaint
    notifications. The root package also has unconditional UI dependencies.
    Extract a headless core package and pass a repaint callback/event adapter from
    the UI. Merely replacing the Context type without separating or gating package
    dependencies will not remove GUI build work. Validate core transport tests
    without a renderer and then integration behavior in the app.
17. **Implemented: benchmark reduced development debug information.** The original
    dev/test profiles used full debug information. Compare
    debug = "line-tables-only" or debug = 1 with the current profile, preserving
    incremental compilation. This trades debugger detail for potentially less
    codegen/link work; measure cold, one-file edit, and no-change builds separately.
    Profile changes invalidate caches once. This is a candidate, not a measured win.
18. **Specify the worker shell and syntax.** The tool description says shell
    command but does not disclose that Windows uses cmd.exe /D /S /C and Unix
    uses /bin/sh -c. Put the actual shell in the tool description/system context
    so the model does not assume PowerShell syntax on Windows. Test executable
    paths with spaces and embedded quotes as well as ordinary commands.
19. **Cancel command descendants, not just the shell.** The process wrapper owns
    a cmd.exe or sh child with kill_on_drop; there is no process-tree ownership.
    Stopping/timing out a shell is not proof cargo/rustc descendants stopped.
    Use Windows job objects or Unix process groups and reap after cancellation.
    Verify with a harmless child/grandchild fixture that records termination.
    This is a source-level risk; no process-tree cancellation experiment was run.
20. **Keep the original objective separate from the latest message.** Journal::open
    takes the last user message as task, then nests the old task/context into a
    summary. Repeated Continue turns grow that summary until compaction/excerpting.
    Keep structured objective, current steering, pending steps and outcomes; update
    the objective only on an actual task change. Verify multiple resumes and
    clarification turns, including a failed summary request.
21. **Deliver the worker's final conclusion when output is capped.**
    `src/delegation.rs` accumulates a bounded output prefix and returns its first
    24k characters. Long progress streams can exclude the final result, changed
    files, and verification. Preserve a bounded tail or a separate final result,
    plus a reference to the full task log. Test long progress followed by a unique
    final conclusion and a retry that replaces streamed content.
22. **Avoid duplicate context during summarization.** Conversation::compact
    concatenates the journal context and the message history. Both contain the
    same assistant output and tool results, increasing summary input and causing
    more excerpt loss. Keep durable action records, but construct model summary
    input once per event with stable IDs. Verify results and pending actions survive.

## Verification record

- First pass: `cargo check --locked --all-targets --features design-export --timings`
  passed in 37.03 seconds, with unused-code warnings in design-export paths.
- Follow-up implementation and benchmark results: [Build speed](BUILD_SPEED.md).
  Findings 11 and 17 are implemented; other proposals remain recommendations.

## References

- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Cargo feature unification](https://doc.rust-lang.org/cargo/reference/features.html)
