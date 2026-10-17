// Item actions shared by the tree and the Top 100. These are the only actions
// in the app, and neither changes any file.

import { reactive } from "vue";
import { revealInExplorer, type TreeNode } from "./api";
import { i18n } from "./i18n";

export const menu = reactive({
  open: false,
  x: 0,
  y: 0,
  node: null as TreeNode | null,
});

export const toast = reactive({ message: "", kind: "info" as "info" | "error", timer: 0 });

export function showToast(message: string, kind: "info" | "error" = "info"): void {
  window.clearTimeout(toast.timer);
  toast.message = message;
  toast.kind = kind;
  toast.timer = window.setTimeout(() => (toast.message = ""), kind === "error" ? 6000 : 2000);
}

export function openMenu(event: MouseEvent, node: TreeNode): void {
  event.preventDefault();
  menu.node = node;
  menu.x = event.clientX;
  menu.y = event.clientY;
  menu.open = true;
}

export function closeMenu(): void {
  menu.open = false;
}

const t = (key: string, values: Record<string, unknown>) => i18n.global.t(key, values);

export async function reveal(node: TreeNode): Promise<void> {
  closeMenu();
  try {
    await revealInExplorer(node.id);
  } catch (error) {
    showToast(t("actions.revealFailed", { message: String(error) }), "error");
  }
}

export async function copyPath(node: TreeNode): Promise<void> {
  closeMenu();
  try {
    await navigator.clipboard.writeText(node.path);
    showToast(t("actions.copied", {}));
  } catch (error) {
    showToast(t("actions.copyFailed", { message: String(error) }), "error");
  }
}
