# Sizewise

A Windows app that organizes folders, shows where disk space is being used and diagnoses what is slowing the PC, with Android and iOS companions focused on freeing space; always keeping the user in control of every change.

## Organization

**Organization**:
Moving the loose files at the root of a chosen folder into subfolders according to a criterion. Never deletes files and never touches existing subfolders.
_Avoid_: Tidying

**Category**:
A group of file types (Images, Documents, Videos, Music, Archives, Installers, Code, Other) that becomes a subfolder when organizing by type.
_Avoid_: Type, group

**Name conflict**:
When the destination already contains a file with the same name; resolved with a numeric suffix, e.g. `photo (1).jpg`. Nothing is ever overwritten.

## Scanning

**Scan**:
Measuring the space used by every file and folder of a Target.
_Avoid_: Analysis, crawl

**Target**:
The drive or folder a Scan measures. A Scan has exactly one Target.

**Fast scan**:
A Scan that reads the NTFS file system index in one pass; requires administrator rights.

**Standard scan**:
A Scan that walks folders one by one; requires no administrator rights.

**Large file**:
A file among the biggest results of a Scan.
_Avoid_: Heavy file, big file

**Protected folder**:
A system folder (e.g. `C:\Windows`, `Program Files`, `ProgramData`, a drive root) that appears in a Scan but accepts neither Organization nor Deletion.

## Operations

**Operation**:
A user-confirmed, reversible action that changes files or system configuration (move, rename, send to the Recycle Bin, disable a Startup item). Every Operation goes through a Preview and is recorded in the History. Ending a process is not an Operation, since it cannot be undone.

**Preview**:
The list of what an Operation will do, shown before it runs.

**History**:
The persistent record of executed Operations; survives closing the app.

**Undo**:
Reverting an Operation from the History, returning files to their previous state, including restoring them from the Recycle Bin.

**Deletion**:
Sending files the user selected themselves to the platform's Trash. The app never deletes permanently and never suggests what to delete, except Regenerable files during a Cleanup.
_Avoid_: Permanent delete, purge

## Performance

**Diagnosis**:
A measured fact about what is slowing the PC (e.g. a Startup item's impact, a process's CPU use, free disk space). The app shows Diagnoses; the user decides what to do.
_Avoid_: Health score, optimization

**Startup item**:
A program configured to launch automatically when the user signs in to Windows.
_Avoid_: Autorun, startup app

**Regenerable file**:
A file that Windows or a program recreates on its own when needed (temporary files, caches). Never a file the user created.
_Avoid_: Junk, garbage

**Cleanup**:
A Deletion of Regenerable files, listed by source; the user selects which sources to clear and nothing is pre-selected.
_Avoid_: Boost, purge

**Duplicate**:
A file whose content is identical to another file's, regardless of name or location.

**Similar photo**:
A photo that looks almost the same as another (e.g. a burst or retakes) without being a Duplicate.

## Mobile

**Trash**:
The platform's recoverable holding area for deleted items: the Windows Recycle Bin, "Recently Deleted" on iOS, the system trash on Android 11+.
_Avoid_: Bin (alone), recycle

**Unused app**:
An installed mobile app the user has not opened for 90 days or more.
_Avoid_: Dead app, bloatware

**Uninstall**:
Removing an installed program or app through the platform's own uninstaller, confirmed by the user. Distinct from Deletion: it is not reversible through Undo.
