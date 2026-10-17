// Typed wrappers around the Tauri commands and events of the Rust side.
// The UI holds no scanning logic: everything here is a request or a notification.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Drive {
  root: string;
  letter: string;
  label: string;
  fileSystem: string;
  totalBytes: number;
  freeBytes: number;
}

export interface TreeNode {
  id: number;
  name: string;
  kind: "folder" | "file";
  size: number;
  files: number;
  folders: number;
  /** Milliseconds since the Unix epoch. */
  modified: number | null;
  childCount: number;
  path: string;
}

export interface ChildrenPage {
  items: TreeNode[];
  total: number;
}

export interface Summary {
  scanId: number;
  target: string;
  root: TreeNode;
  totalSize: number;
  totalFiles: number;
  totalFolders: number;
  durationMs: number;
  state: "completed" | "cancelled";
  unreadableCount: number;
}

export interface Progress {
  scanId: number;
  currentFolder: string;
  files: number;
  bytes: number;
  elapsedMs: number;
}

export interface ScanFailure {
  scanId: number;
  target: string;
  reason: "notFound" | "notAFolder" | "unreadable" | "internal";
  message: string;
}

export const systemLocale = () => invoke<string | null>("system_locale");

export const listDrives = () => invoke<Drive[]>("list_drives");

export const startScan = (target: string) => invoke<number>("start_scan", { target });

export const cancelScan = () => invoke<void>("cancel_scan");

export const getSummary = (scanId: number) => invoke<Summary>("get_summary", { scanId });

export const getChildren = (id: number, offset: number, limit: number) =>
  invoke<ChildrenPage>("get_children", { id, offset, limit });

export const getTopFiles = (limit: number) => invoke<TreeNode[]>("get_top_files", { limit });

export const getUnreadable = () => invoke<string[]>("get_unreadable");

export const revealInExplorer = (id: number) => invoke<void>("reveal_in_explorer", { id });

export interface ScanListeners {
  progress: (progress: Progress) => void;
  completed: (summary: Summary) => void;
  cancelled: (summary: Summary) => void;
  failed: (failure: ScanFailure) => void;
}

/** Subscribes to every Scan event; returns a function that unsubscribes. */
export async function onScanEvents(listeners: ScanListeners): Promise<UnlistenFn> {
  const unlisten = await Promise.all([
    listen<Progress>("scan-progress", (e) => listeners.progress(e.payload)),
    listen<Summary>("scan-completed", (e) => listeners.completed(e.payload)),
    listen<Summary>("scan-cancelled", (e) => listeners.cancelled(e.payload)),
    listen<ScanFailure>("scan-failed", (e) => listeners.failed(e.payload)),
  ]);
  return () => unlisten.forEach((fn) => fn());
}
