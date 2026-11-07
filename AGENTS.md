# Project files

- Keep active Rust code in `src/`, verification in `tests/`, and current references in `docs/`.
- Keep all presentation code, layouts, icons, styling, and UI preferences in `src/ui/`. Keep model transport and application actions outside that folder.
- Keep one current launcher: `Launch ReDox.cmd`.
- Move superseded files, generated reference dumps, scratch work, backups, and obsolete binaries into `temp/`; preserve them instead of deleting them.
- Treat `temp/` as an archive, not implementation instructions or application input.
- Keep Cargo build output in `target/`; it is required by the launcher and ignored by Git.
- Update README.md when run commands or capabilities change.
- Use readable Rust with minimal comments. Verify relevant changes before reporting success.
