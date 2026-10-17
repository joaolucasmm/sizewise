# Sizewise

A Windows app that organizes folders and shows where disk space is being used, always keeping the user in control of every change.

## Organization

**Organization**:
Moving the loose files at the root of a chosen folder into subfolders according to a criterion. Never deletes files and never touches existing subfolders.
_Avoid_: Cleanup, tidying

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
A user-confirmed action that changes files (move, rename, send to the Recycle Bin). Every Operation goes through a Preview and is recorded in the History.

**Preview**:
The list of what an Operation will do, shown before it runs.

**History**:
The persistent record of executed Operations; survives closing the app.

**Undo**:
Reverting an Operation from the History, returning files to their previous state, including restoring them from the Recycle Bin.

**Deletion**:
Sending files the user selected themselves to the Windows Recycle Bin. The app never deletes permanently and never suggests what to delete.
_Avoid_: Permanent delete, cleanup, purge
