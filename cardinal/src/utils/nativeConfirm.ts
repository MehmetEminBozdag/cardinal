import { invoke } from '@tauri-apps/api/core';

export type NativeConfirmOptions = {
  message: string;
  detail?: string;
  confirmLabel: string;
  cancelLabel: string;
  destructive?: boolean;
};

/**
 * Asks for confirmation with a native macOS alert.
 *
 * The WKWebView used by Tauri does not implement `window.confirm` (it returns
 * false without showing anything), so the native `confirm_dialog` command is
 * used. `window.confirm` remains the fallback outside the desktop shell.
 */
export async function confirmNative(options: NativeConfirmOptions): Promise<boolean> {
  try {
    const result = await invoke<boolean>('confirm_dialog', {
      message: options.message,
      detail: options.detail ?? null,
      confirmLabel: options.confirmLabel,
      cancelLabel: options.cancelLabel,
      destructive: options.destructive ?? false,
    });
    if (typeof result === 'boolean') return result;
  } catch {
    // Fall through to the browser dialog.
  }
  return window.confirm([options.message, options.detail].filter(Boolean).join('\n\n'));
}

/** Native replacement for `window.alert` (also ignored by WKWebView). */
export async function alertNative(
  message: string,
  okLabel: string,
  detail?: string,
): Promise<void> {
  try {
    await invoke('message_dialog', { message, detail: detail ?? null, okLabel });
    return;
  } catch {
    // Fall through to the browser dialog.
  }
  window.alert([message, detail].filter(Boolean).join('\n\n'));
}
