// A single confirmation dialog for actions Sizewise can't undo (End task,
// Uninstall). Screens await `askConfirm`; `ConfirmDialog` renders it.

import { reactive } from "vue";

export interface ConfirmRequest {
  title: string;
  /** Paragraphs shown in order. */
  message: string[];
  confirmLabel: string;
}

export const confirmState = reactive({
  request: null as ConfirmRequest | null,
  resolve: null as ((confirmed: boolean) => void) | null,
});

export function askConfirm(request: ConfirmRequest): Promise<boolean> {
  confirmState.resolve?.(false);
  return new Promise((resolve) => {
    confirmState.request = request;
    confirmState.resolve = resolve;
  });
}

export function answerConfirm(confirmed: boolean): void {
  const resolve = confirmState.resolve;
  confirmState.request = null;
  confirmState.resolve = null;
  resolve?.(confirmed);
}
