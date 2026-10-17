# Changelog

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
