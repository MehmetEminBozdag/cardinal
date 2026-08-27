import { act, renderHook } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDuplicateFiles } from '../useDuplicateFiles';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('useDuplicateFiles', () => {
  beforeEach(() => vi.mocked(invoke).mockReset());

  it('scans the current result set only when requested', async () => {
    vi.mocked(invoke).mockResolvedValue({ scannedFiles: 2, groups: [] });
    const { result } = renderHook(() => useDuplicateFiles([11, 22]));

    expect(invoke).not.toHaveBeenCalled();
    await act(() => result.current.scan());

    expect(invoke).toHaveBeenCalledWith('find_duplicates', { results: [11, 22] });
    expect(result.current.state).toBe('ready');
    expect(result.current.scannedFiles).toBe(2);
  });
});
