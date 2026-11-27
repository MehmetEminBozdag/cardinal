import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useFolderSizes } from '../useFolderSizes';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: () => Promise.resolve('/Users/test/') }));

describe('size view native activity', () => {
  beforeEach(() => {
    mocks.invoke.mockReset().mockImplementation((command: string) => {
      if (command === 'get_size_status')
        return Promise.resolve({ phase: 'ready', rootPath: '/Users/test', version: 1 });
      return Promise.resolve(null);
    });
  });

  it('deactivates native scans immediately on tab switch and unmount', async () => {
    const hook = renderHook(({ active }) => useFolderSizes(active), {
      initialProps: { active: true },
    });
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('set_size_view_active', { active: true }),
    );
    hook.rerender({ active: false });
    expect(mocks.invoke).toHaveBeenCalledWith('set_size_view_active', { active: false });
    mocks.invoke.mockClear();
    hook.unmount();
    expect(mocks.invoke).toHaveBeenCalledWith('set_size_view_active', { active: false });
  });

  it('does not start a home scan before native tab activation completes', async () => {
    let activate!: () => void;
    mocks.invoke.mockImplementation((command: string, args?: { active?: boolean }) => {
      if (command === 'set_size_view_active' && args?.active)
        return new Promise<void>((resolve) => {
          activate = resolve;
        });
      if (command === 'get_size_status') return Promise.resolve({ phase: 'idle', version: 0 });
      return Promise.resolve(null);
    });
    const hook = renderHook(() => useFolderSizes(true));
    await waitFor(() => expect(activate).toBeDefined());
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'start_size_scan')).toBe(false);
    activate();
    await waitFor(() =>
      expect(mocks.invoke).toHaveBeenCalledWith('start_size_scan', { path: '/Users/test' }),
    );
    hook.unmount();
  });
});
