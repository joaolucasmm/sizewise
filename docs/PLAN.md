# Plan — Sizewise

A Windows app to organize folders and find what is using disk space. Vocabulary in [GLOSSARY.md](../GLOSSARY.md); structural decisions in [docs/adr](adr/).

## Context

- Personal use, open source on GitHub (MIT license), portfolio showcase aimed at international roles.
- Stack: Tauri 2 + Vue 3 + TypeScript (UI) + Rust (core) — [ADR 0001](adr/0001-tauri-vue-rust.md).
- UI in English and Brazilian Portuguese from day one (i18n); Category folder names follow the selected language.
- All project artifacts (code, docs, issues, commits) are written in English.
- Distribution: Tauri-generated installer on GitHub Releases, unsigned (the README explains the SmartScreen warning).

## Safety principles — [ADR 0002](adr/0002-no-automatic-deletion.md)

- Every Operation goes through a Preview and is recorded in the History (persistent across restarts).
- Deletion is manual only (multi-select allowed, nothing pre-selected) and only to the Recycle Bin.
- Undo also restores from the Recycle Bin; it warns if the file changed or the Recycle Bin was emptied.
- Protected folders (`C:\Windows`, `Program Files`, `ProgramData`, drive roots) are shown but accept no actions.
- Name conflict → `(1)` suffix; never overwrite.

## v1 scope

**Scan**
- One Target at a time (drive or folder).
- Parallel Standard scan (no admin); Fast scan via the MFT when elevated + NTFS.
- App starts without admin; a "Fast scan (requires administrator)" button relaunches it elevated.
- Live progress (current folder, files, size) + Cancel.
- Results: tree sorted by size (virtualized list) + top 100 Large files.
- Actions: open in Explorer; manual Deletion.
- The last Scan is not kept between sessions.

**Organization**
- Manual, with Preview; only loose files at the root of the chosen folder.
- v1 criterion: by Category (Images, Documents, Videos, Music, Archives, Installers, Code, Other); subfolders created inside the folder itself.

**Quality**
- Automated tests in the Rust core (Organization, Name conflict, Deletion, Undo) on real temporary folders.
- GitHub Actions: build + tests on every push; installer generated on tag.

## Milestones

1. **M1** — Tauri + Vue + TS skeleton, EN/PT-BR i18n, CI.
2. **M2** — Standard scan: tree, top 100, progress, cancel.
3. **M3** — Manual Deletion + History + Undo + Protected folders.
4. **M4** — Organization by Category with Preview.
5. **M5** — Fast scan (MFT) + timing comparison in the README.
6. **M6** — Treemap.

## After v1

- Organization by date (Year/Month) and custom rules (extension / name fragment).
- Watching folders and organizing automatically.
- Scanning several drives at once.
- **Phase 2 — optimizations:** temp files and cache cleanup; startup programs; installed programs by size; duplicate files.

## Not in the product

- "Free up RAM", "registry cleaning" — no real gain, real risk.
- Permanent or automatic Deletion.

## Environment prerequisites

- Install Rust (rustup) and Tauri's Windows dependencies (WebView2, MSVC Build Tools). Node 23 is already present.
