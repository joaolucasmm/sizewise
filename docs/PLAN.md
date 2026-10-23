# Plan — Sizewise

A Windows app to organize folders, find what is using disk space and diagnose what is slowing the PC, followed by native Android and iOS apps focused on freeing space. Vocabulary in [GLOSSARY.md](../GLOSSARY.md); structural decisions in [docs/adr](adr/).

## Context

- Personal use, open source on GitHub (MIT license), portfolio showcase aimed at international roles.
- Stack: Tauri 2 + Vue 3 + TypeScript (UI) + Rust (core) — [ADR 0001](adr/0001-tauri-vue-rust.md).
- UI in English and Brazilian Portuguese from day one (i18n); Category folder names follow the selected language.
- All project artifacts (code, docs, issues, commits) are written in English.
- Distribution: Tauri-generated installer on GitHub Releases, unsigned (the README explains the SmartScreen warning).

## Safety principles — [ADR 0002](adr/0002-no-automatic-deletion.md)

- Every Operation goes through a Preview and is recorded in the History (persistent across restarts).
- Deletion is manual only (multi-select allowed, nothing pre-selected) and only to the Recycle Bin. Single narrow exception: Cleanup may list Regenerable files ([ADR 0003](adr/0003-cleanup-of-regenerable-files.md)), still to the Recycle Bin.
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
## Phase 2 — Performance (desktop, after v0.1.0)

Posture: the app shows **Diagnoses** (measured facts about what is slowing the PC) and the user decides. No health score, no "speed up your PC" claims — the pitch is "see what's slowing your PC and fix it yourself".

Focus: faster startup, less lag in use, free space. Hardware health is view-only.

1. **Startup items** — sources: registry Run keys, Startup folders, Store apps (toggleable); logon scheduled tasks (view-only). Disabled with the same mechanism Task Manager uses (marks as disabled, deletes nothing). Disabling is an Operation: Preview, History, Undo.
2. **Resource monitor** — live snapshot of processes by CPU, RAM and disk, refreshed every 1–2 s; no history, nothing in the background. "Open file location" and "End task" (with confirmation, blocked for critical Windows processes). Ending a process is not an Operation.
3. **Installed programs by size** — shortcut to each program's official uninstaller.
4. **Cleanup** of Regenerable files — [ADR 0003](adr/0003-cleanup-of-regenerable-files.md). Listed by source, nothing pre-selected, Preview, sent to the Recycle Bin, History + Undo. First sources (no admin): user `%TEMP%`, Chrome/Edge/Firefox caches (only while the browser is closed), Explorer thumbnail cache, crash dumps and error reports. Later, with admin mode: `C:\Windows\Temp`, Windows Update leftovers. Files in use are skipped and listed. When done: "X GB moved to the Recycle Bin; space is freed when you empty it" + "Open Recycle Bin" button; the app never empties it.
5. **Duplicates** — the user marks which copies to keep; unmarked copies go to the Recycle Bin. A group with no copy marked to keep is left untouched, so at least one copy always survives.
6. **Disk health and low space** — SMART status per drive and a warning below 10% free, shown only inside the app (no notifications, no background service).

Excluded: disabling Windows services — breaks features for negligible gain.

## Mobile (after desktop v1 / M6) — [ADR 0004](adr/0004-native-mobile-apps.md)

- Native apps: Kotlin + Jetpack Compose (Android), SwiftUI (iOS), in this repository with their own folders; shared glossary, ADRs and issues, no shared code. CI builds only what changed.
- Positioning: freeing space, never "boosting" — the OS sandbox prevents real performance tuning, and stores reject such claims. Built and installed locally first; store release is possible later.
- **Android first** (author's Redmi 9 Pro — check it runs Android 11+, required for the system Trash and the clear-cache dialog). **iOS later** on a borrowed work Mac + iPhone XR (confirm the employer allows it; free provisioning expires every 7 days).
- Deletion always goes to the platform's Trash (iOS "Recently Deleted", Android 11+ system trash), confirmed by the OS dialog.

**Mobile v1**
- Storage overview.
- Large videos and media.
- Exact Duplicate photos.
- Screenshots.
- Android: all installed apps sorted by time since last use (most idle first), each showing that time; apps idle 90+ days tagged **Unused**; Uninstall via the system dialog. Needs the one-time "Usage access" permission; non-removable system apps are excluded.
- Android: button opening the system dialog that clears all apps' caches (Android 11+).

**Mobile later**
- Similar photos (on-device image similarity).
- iOS: duplicate contacts.
- Home-screen widgets showing free space.

## Not in the product

- "Free up RAM", "registry cleaning", "boost" — no real gain, real risk.
- Health scores or made-up performance numbers.
- Permanent or automatic Deletion; emptying the Trash on the user's behalf.
- Disabling Windows services.

## Environment prerequisites

- Install Rust (rustup) and Tauri's Windows dependencies (WebView2, MSVC Build Tools). Node 23 is already present.
