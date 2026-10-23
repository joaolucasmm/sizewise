# Changelog

## Unreleased

Phase 2a, Diagnoses: measured facts about what is slowing the PC, with no score and no "speed up" button.

- Persistent navigation: Disk space (the existing Scan, unchanged), Overview, Startup, Processes, Programs and Disks.
- **Overview**: free space per drive, enabled Startup items, the last startup duration, the top CPU and memory users and any disk reporting a health problem, each linking to its section.
- **Startup items** from the registry Run keys, the Startup folders, Store apps and logon scheduled tasks, with publisher, command, source and the same enabled/disabled state Task Manager shows. Items whose program file is gone are flagged as broken. View-only for now. Recent startup durations come from Windows' boot performance log, which needs administrator rights; per-item impact isn't shown because Windows doesn't expose it.
- **Processes**: live CPU, memory (private working set) and I/O per program, grouped by executable and expandable, with PC totals, refreshed every 1.5 s only while the screen is open. "Open file location" and a confirmed "End task", blocked for critical Windows processes and for Sizewise itself.
- **Installed programs** by size, searchable and sortable, with a "Measure" for programs that report no size, "Open install folder" and a confirmed shortcut to the program's own uninstaller. System components and updates are hidden.
- **Disks**: model, type, connection, size and health of each physical disk, with a backup warning when one reports a problem, and a warning plus "Scan this drive" for drives under 10% free.
- Data Windows only gives to administrators says so ("Needs administrator") instead of being left out silently.

## v0.1.0

First release: a read-only Standard scan of a drive or folder.

- Pick a whole drive (with total and free space) or any folder as the Target.
- Parallel Standard scan with live progress (current folder, files read, size so far, elapsed time) and Cancel.
- Folder tree sorted by size, with readable sizes, % of parent, a proportional bar and file/folder counts. Children load on demand and the list is virtualized, so full-drive results stay smooth.
- Top 100 Large files across the whole Target, with folder, size and modification date.
- Summary of the Target: total size, files, folders and Scan duration.
- Folders that can't be read are reported (with the list) instead of stopping the Scan; junctions and symbolic links are never followed.
- Cancelled Scans are clearly marked as incomplete.
- "Open in Explorer" (item selected) and "Copy path" on every item. Nothing in the app changes files.
- English and Brazilian Portuguese UI, following the Windows language, with a switcher that is remembered.
- Runs without administrator rights.
