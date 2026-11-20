import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { homeDir } from '@tauri-apps/api/path';
import { parentPath } from '../utils/folderSize';

export type SizeItemKind = 'dir' | 'file' | 'small';

export type SizeItem = {
  name: string;
  path: string;
  kind: SizeItemKind;
  size: number;
  fileCount: number;
  unreadableCount: number;
  children?: SizeItem[];
};

export type SizeListing = {
  path: string;
  rootPath: string;
  size: number;
  fileCount: number;
  unreadableCount: number;
  items: SizeItem[];
};

export type SizeStatus = {
  phase: 'idle' | 'scanning' | 'ready';
  rootPath: string | null;
  scanningPath: string | null;
  scannedFiles: number;
  scannedDirs: number;
  scannedBytes: number;
  totalSize: number;
  totalFiles: number;
  unreadableCount: number;
  scannedAt: number | null;
  lastScanMs: number | null;
  fromSnapshot: boolean;
  pendingUpdates: number;
  version: number;
  /** Size of ~/.Trash when it is inside the scanned tree. */
  trashSize: number | null;
};

export type VolumeSpace = {
  total: number;
  /** Free right now. */
  available: number;
  /** Finder's "available": free plus purgeable space macOS can reclaim. */
  availableForImportantUsage: number | null;
};

export type SizeSuggestion = {
  rule: string;
  level: 'safe' | 'review';
  path: string;
  name: string;
  kind: SizeItemKind;
  size: number;
};

export type SizeViewMode = 'list' | 'treemap' | 'suggestions';

const ACTIVE_POLL_MS = 700;
const IDLE_POLL_MS = 5_000;
const VOLUME_POLL_MS = 15_000;

/**
 * State for the folder size tab. Polls the native engine (cheap) and refetches
 * the listing whenever the tree version changes, so FSEvent-driven updates
 * show up without user action.
 */
export function useFolderSizes(active: boolean) {
  const [status, setStatus] = useState<SizeStatus | null>(null);
  const [home, setHome] = useState<string | null>(null);
  const [currentPath, setCurrentPath] = useState<string | null>(null);
  const [listing, setListing] = useState<SizeListing | null>(null);
  const [suggestions, setSuggestions] = useState<SizeSuggestion[]>([]);
  const [mode, setMode] = useState<SizeViewMode>('list');
  const [error, setError] = useState<string | null>(null);
  const [volume, setVolume] = useState<VolumeSpace | null>(null);
  const autoStarted = useRef(false);

  useEffect(() => {
    homeDir()
      .then((dir) => setHome(dir.replace(/\/$/, '')))
      .catch(() => setHome(null));
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const next = await invoke<SizeStatus>('get_size_status');
      setStatus((prev) =>
        prev &&
        prev.version === next.version &&
        prev.phase === next.phase &&
        prev.scannedFiles === next.scannedFiles &&
        prev.pendingUpdates === next.pendingUpdates
          ? prev
          : next,
      );
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    const timer = window.setInterval(
      () => void refreshStatus(),
      active ? ACTIVE_POLL_MS : IDLE_POLL_MS,
    );
    return () => window.clearInterval(timer);
  }, [active, refreshStatus]);

  const scan = useCallback(
    async (path: string) => {
      setError(null);
      try {
        await invoke<boolean>('start_size_scan', { path });
        const root = status?.rootPath;
        const insideTree = root != null && (path === root || path.startsWith(`${root}/`));
        if (!insideTree || root === path) setCurrentPath(path);
        void refreshStatus();
      } catch (cause) {
        setError(String(cause));
      }
    },
    [refreshStatus, status?.rootPath],
  );

  // First visit with nothing scanned yet: scan the home folder.
  useEffect(() => {
    if (!active || autoStarted.current || !home || !status) return;
    if (status.phase === 'idle') {
      autoStarted.current = true;
      void scan(home);
    } else {
      autoStarted.current = true;
    }
  }, [active, home, scan, status]);

  // Follow the root when it appears or changes.
  useEffect(() => {
    const root = status?.rootPath;
    if (!root) return;
    setCurrentPath((path) =>
      path && (path === root || path.startsWith(root === '/' ? '/' : `${root}/`)) ? path : root,
    );
  }, [status?.rootPath]);

  const version = status?.version ?? -1;
  const nested = mode === 'treemap';

  useEffect(() => {
    if (!active || !currentPath || status?.phase === 'idle') return;
    let cancelled = false;
    invoke<SizeListing | null>('get_size_listing', { path: currentPath, nested })
      .then((next) => {
        if (cancelled) return;
        if (next) {
          setListing(next);
        } else if (status?.rootPath && currentPath !== status.rootPath) {
          // The folder disappeared (trashed, renamed): step up until one exists.
          setCurrentPath(parentPath(currentPath));
        } else {
          setListing(null);
        }
      })
      .catch((cause) => !cancelled && setError(String(cause)));
    return () => {
      cancelled = true;
    };
  }, [active, currentPath, nested, version, status?.phase, status?.rootPath]);

  useEffect(() => {
    if (!active || mode !== 'suggestions' || status?.phase === 'idle') return;
    let cancelled = false;
    invoke<SizeSuggestion[]>('get_size_suggestions')
      .then((list) => !cancelled && setSuggestions(list))
      .catch((cause) => !cancelled && setError(String(cause)));
    return () => {
      cancelled = true;
    };
  }, [active, mode, version, status?.phase]);

  // Free space changes outside our control (other apps, emptying the Trash),
  // so refresh it periodically as well as whenever the tree changes.
  const rootForVolume = status?.rootPath ?? home;
  useEffect(() => {
    if (!active || !rootForVolume) return;
    let cancelled = false;
    const load = () =>
      invoke<VolumeSpace | null>('get_volume_space', { path: rootForVolume })
        .then((space) => !cancelled && setVolume(space))
        .catch(() => !cancelled && setVolume(null));
    void load();
    const timer = window.setInterval(() => void load(), VOLUME_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [active, rootForVolume, version]);

  const navigate = useCallback((path: string) => setCurrentPath(path), []);

  const goUp = useCallback(() => {
    const root = status?.rootPath;
    setCurrentPath((path) => (path && root && path !== root ? parentPath(path) : path));
  }, [status?.rootPath]);

  const rescanCurrent = useCallback(() => {
    if (currentPath) void scan(currentPath);
  }, [currentPath, scan]);

  const cancel = useCallback(() => {
    void invoke('cancel_size_scan');
  }, []);

  return {
    status,
    home,
    currentPath,
    listing,
    suggestions,
    volume,
    mode,
    setMode,
    error,
    scan,
    rescanCurrent,
    cancel,
    navigate,
    goUp,
  };
}
