import { afterEach, describe, expect, it, vi } from 'vitest';

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

const { confirmNative } = await import('../nativeConfirm');

const options = { message: 'Remove?', detail: 'Details', confirmLabel: 'OK', cancelLabel: 'No' };

describe('confirmNative', () => {
  afterEach(() => {
    invokeMock.mockReset();
    vi.restoreAllMocks();
  });

  it('uses the native dialog result', async () => {
    invokeMock.mockResolvedValue(false);
    const confirmSpy = vi.spyOn(window, 'confirm');
    await expect(confirmNative(options)).resolves.toBe(false);
    expect(invokeMock).toHaveBeenCalledWith(
      'confirm_dialog',
      expect.objectContaining({ message: 'Remove?' }),
    );
    expect(confirmSpy).not.toHaveBeenCalled();
  });

  it('falls back to window.confirm when the command is unavailable', async () => {
    invokeMock.mockRejectedValue(new Error('no command'));
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    await expect(confirmNative(options)).resolves.toBe(true);
    expect(confirmSpy).toHaveBeenCalledWith('Remove?\n\nDetails');
  });
});
