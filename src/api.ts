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

/** Every Large file the Scan kept (the Top 100), largest first. */
export const getTopFiles = () => invoke<TreeNode[]>("get_top_files");

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

// Diagnoses

/** Why a Diagnosis has no value. */
export interface Unavailable {
  reason: "needsAdministrator" | "failed";
  message: string;
}

/** A value, or the reason it is missing; exactly one of the two is set. */
export interface Availability<T> {
  value: T | null;
  unavailable: Unavailable | null;
}

export type Scope = "currentUser" | "allUsers";

export type StartupSource =
  | { kind: "registryRun"; scope: Scope; wow64: boolean }
  | { kind: "startupFolder"; scope: Scope }
  | { kind: "storeApp" }
  | { kind: "scheduledTask"; path: string };

export interface StartupItem {
  name: string;
  publisher: string | null;
  command: string;
  target: string | null;
  source: StartupSource;
  enabled: boolean;
  /** Its program file no longer exists. */
  broken: boolean;
}

export interface StartupDuration {
  /** Milliseconds since the Unix epoch. */
  recordedAt: number;
  bootMs: number;
  mainPathMs: number;
  postBootMs: number;
}

export type Protection = "critical" | "ownProcess";

export interface ProcessInfo {
  pid: number;
  name: string;
  path: string | null;
  /** Share of the whole CPU, 0–100; null on the first sample. */
  cpuPercent: number | null;
  ramBytes: number | null;
  readBytesPerSec: number | null;
  writeBytesPerSec: number | null;
  protection: Protection | null;
}

export interface ProcessGroup {
  key: string;
  name: string;
  path: string | null;
  processes: ProcessInfo[];
  cpuPercent: number | null;
  /** Null when no process of the group could be read. */
  ramBytes: number | null;
  readBytesPerSec: number | null;
  writeBytesPerSec: number | null;
}

export interface SystemTotals {
  cpuPercent: number | null;
  ramUsedBytes: number;
  ramTotalBytes: number;
  readBytesPerSec: number | null;
  writeBytesPerSec: number | null;
}

export interface ProcessSnapshot {
  groups: ProcessGroup[];
  totals: SystemTotals;
}

export interface EndTaskFailure {
  reason: "critical" | "ownProcess" | "accessDenied" | "alreadyExited" | "failed";
  message: string;
}

export interface Program {
  id: string;
  name: string;
  publisher: string | null;
  version: string | null;
  /** `YYYY-MM-DD`. */
  installDate: string | null;
  sizeBytes: number | null;
  installLocation: string | null;
  scope: Scope;
}

export interface UninstallFailure {
  reason: "cancelled" | "uninstallerMissing" | "notFound" | "failed";
  message: string;
}

export interface UninstallFinished {
  id: string;
  exitCode: number | null;
}

export interface PhysicalDisk {
  id: string;
  model: string;
  mediaType: "ssd" | "hdd" | "scm" | "unspecified";
  busType: string;
  sizeBytes: number;
  health: "healthy" | "warning" | "unhealthy" | "unknown";
  hasProblem: boolean;
}

export interface Volume extends Drive {
  /** Less than 10% free. */
  lowSpace: boolean;
}

export interface Overview {
  volumes: Volume[];
  enabledStartupItems: number;
  lastStartup: Availability<StartupDuration | null>;
  disks: Availability<PhysicalDisk[]>;
}

export const getOverview = () => invoke<Overview>("get_overview");

export const listStartupItems = () => invoke<StartupItem[]>("list_startup_items");

export const getStartupDurations = () => invoke<Availability<StartupDuration[]>>("get_startup_durations");

/** Ends a process; rejects with an `EndTaskFailure`. */
export const endProcess = (pid: number) => invoke<void>("end_process", { pid });

export const listInstalledPrograms = () => invoke<Program[]>("list_installed_programs");

/** Size of the program's install folder, in bytes. */
export const measureProgram = (id: string) => invoke<number>("measure_program", { id });

/** Starts the program's own uninstaller; rejects with an `UninstallFailure`. */
export const startUninstall = (id: string) => invoke<void>("start_uninstall", { id });

export const listPhysicalDisks = () => invoke<Availability<PhysicalDisk[]>>("list_physical_disks");

export const listVolumes = () => invoke<Volume[]>("list_volumes");

export const revealPath = (path: string) => invoke<void>("reveal_path", { path });

/** Opens a folder in Explorer; refuses anything that isn't a folder. */
export const openFolder = (path: string) => invoke<void>("open_folder", { path });

export const onUninstallFinished = (listener: (finished: UninstallFinished) => void) =>
  listen<UninstallFinished>("uninstall-finished", (e) => listener(e.payload));

// Process sampling runs only while a screen that shows it is open. Screens
// hold a share of it; the short delay before stopping lets one screen hand
// over to the next without restarting the sampler.
let samplingUsers = 0;
let stopTimer = 0;

/** Starts receiving process snapshots; returns a function that stops. */
export async function watchProcesses(listener: (snapshot: ProcessSnapshot) => void): Promise<UnlistenFn> {
  const unlisten = await listen<ProcessSnapshot>("process-snapshot", (e) => listener(e.payload));
  samplingUsers++;
  window.clearTimeout(stopTimer);
  await invoke<void>("start_process_sampling");
  return () => {
    unlisten();
    samplingUsers--;
    if (samplingUsers === 0) {
      stopTimer = window.setTimeout(() => void invoke<void>("stop_process_sampling"), 300);
    }
  };
}
