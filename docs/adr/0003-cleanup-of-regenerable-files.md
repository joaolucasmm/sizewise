# Cleanup lists Regenerable files, still only to the Recycle Bin

Amends ADR 0002. A Cleanup of temporary files and caches requires the app to list files the user did not pick themselves, which ADR 0002 forbade. We allow one narrow exception: the app may list **Regenerable files** (files Windows or a program recreates on its own), grouped by source, with nothing pre-selected and a Preview before running. Everything else in ADR 0002 stands: Cleanup goes to the Recycle Bin, is recorded in the History and can be Undone. The trade-off is that a Cleanup frees no space until the user empties the Recycle Bin themselves; we accepted that to keep the "never delete permanently" guarantee intact.

## Considered Options

- **Keep ADR 0002 strict** and only link to Windows Storage Sense: safest, but leaves out a feature users expect.
- **Permanent deletion for Regenerable files only**: frees space immediately, but breaks the one guarantee every Deletion in the app shares.
