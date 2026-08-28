import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SlabIndex } from '../types/slab';

export type DuplicateStorage = 'apfsClone' | 'physicalCopy' | 'hardLink' | 'unknown';

export type DuplicateGroup = {
  size: number;
  files: Array<{ path: string; storage: DuplicateStorage }>;
};

type DuplicateScanResponse = {
  scannedFiles: number;
  skippedFiles: number;
  limited: boolean;
  cancelled: boolean;
  groups: DuplicateGroup[];
};

const MAX_DUPLICATE_CANDIDATES = 20_000;

export type DuplicateScanError = 'tooMany' | 'tooLarge' | 'failed';

export function useDuplicateFiles(results: SlabIndex[], resultsVersion = 0) {
  const [state, setState] = useState<'idle' | 'scanning' | 'ready' | 'error'>('idle');
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [scannedFiles, setScannedFiles] = useState(0);
  const [skippedFiles, setSkippedFiles] = useState(0);
  const [limited, setLimited] = useState(false);
  const [error, setError] = useState<DuplicateScanError | null>(null);
  const requestId = useRef(0);
  const previousResultsVersion = useRef(resultsVersion);

  useEffect(() => {
    if (previousResultsVersion.current === resultsVersion) return;
    previousResultsVersion.current = resultsVersion;
    requestId.current += 1;
    void invoke('cancel_duplicate_scan');
    setState('idle');
    setGroups([]);
    setScannedFiles(0);
    setSkippedFiles(0);
    setLimited(false);
    setError(null);
  }, [resultsVersion]);

  const scan = useCallback(async () => {
    const currentRequestId = requestId.current + 1;
    requestId.current = currentRequestId;
    setState('scanning');
    setError(null);
    if (results.length > MAX_DUPLICATE_CANDIDATES) {
      setGroups([]);
      setError('tooMany');
      setState('error');
      return;
    }
    try {
      const response = await invoke<DuplicateScanResponse>('find_duplicates', { results });
      if (requestId.current !== currentRequestId) return;
      if (response.cancelled) {
        setState('idle');
        return;
      }
      setGroups(response.groups);
      setScannedFiles(response.scannedFiles);
      setSkippedFiles(response.skippedFiles);
      setLimited(response.limited);
      setState('ready');
    } catch (cause) {
      if (requestId.current !== currentRequestId) return;
      setGroups([]);
      setError(String(cause).includes('duplicate_input_too_large') ? 'tooLarge' : 'failed');
      setState('error');
    }
  }, [results]);

  return { state, groups, scannedFiles, skippedFiles, limited, error, scan };
}
