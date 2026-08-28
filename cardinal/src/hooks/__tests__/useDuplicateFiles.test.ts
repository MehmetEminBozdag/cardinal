import { act, renderHook } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDuplicateFiles } from '../useDuplicateFiles';
import type { SlabIndex } from '../../types/slab';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('useDuplicateFiles', () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it('scans the current result set only when requested', async () => {
    vi.mocked(invoke).mockResolvedValue({
      scannedFiles: 2,
      skippedFiles: 0,
      limited: false,
      cancelled: false,
      groups: [],
    });
    const results = [11, 22] as SlabIndex[];
    const { result } = renderHook(() => useDuplicateFiles(results));

    expect(invoke).not.toHaveBeenCalled();
    await act(() => result.current.scan());

    expect(invoke).toHaveBeenCalledWith('find_duplicates', { results: [11, 22] });
    expect(result.current.state).toBe('ready');
    expect(result.current.scannedFiles).toBe(2);
  });

  it('rejects overly broad scans before invoking the backend', async () => {
    const results = Array.from({ length: 20_001 }, (_, index) => index as SlabIndex);
    const { result } = renderHook(() => useDuplicateFiles(results));

    await act(() => result.current.scan());

    expect(invoke).not.toHaveBeenCalled();
    expect(result.current.state).toBe('error');
    expect(result.current.error).toBe('tooMany');
  });

  it('cancels and clears stale duplicate results when the result version changes', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      scannedFiles: 2,
      skippedFiles: 0,
      limited: false,
      cancelled: false,
      groups: [{ size: 10, files: [] }],
    });
    const { result, rerender } = renderHook(
      ({ version }) => useDuplicateFiles([11, 22] as SlabIndex[], version),
      { initialProps: { version: 1 } },
    );
    await act(() => result.current.scan());
    expect(result.current.state).toBe('ready');

    rerender({ version: 2 });

    expect(invoke).toHaveBeenLastCalledWith('cancel_duplicate_scan');
    expect(result.current.state).toBe('idle');
    expect(result.current.groups).toEqual([]);
  });
});
