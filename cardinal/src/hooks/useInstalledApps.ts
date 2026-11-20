import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type InstalledApp = {
  name: string;
  path: string;
  bundleId: string;
  version: string | null;
  appSize: number;
  leftoverSize: number;
  running: boolean;
};

export type LeftoverKind =
  | 'appBundle'
  | 'support'
  | 'cache'
  | 'preferences'
  | 'container'
  | 'savedState'
  | 'logs'
  | 'webData'
  | 'launchItem'
  | 'helper'
  | 'receipt'
  | 'recentDocuments'
  | 'other';

export type Confidence = 'certain' | 'likely' | 'possible';

export type Leftover = {
  path: string;
  kind: LeftoverKind;
  confidence: Confidence;
  size: number;
  needsAdmin: boolean;
  launchItem: boolean;
};

export type AppPlan = {
  name: string;
  path: string;
  bundleId: string;
  version: string | null;
  running: boolean;
  items: Leftover[];
  receipts: string[];
};

export type UninstallReport = {
  moved: string[];
  failed: Array<{ path: string; reason: string }>;
  receiptsForgotten: string[];
  adminCancelled: boolean;
};

export type OrphanGroup = {
  key: string;
  label: string;
  size: number;
  items: Leftover[];
};

export type OrphanReport = {
  groups: OrphanGroup[];
  total: number;
};

/** Certain and likely items are preselected; possible ones need an explicit choice. */
export function defaultSelection(plan: AppPlan): Set<string> {
  return new Set(plan.items.filter((i) => i.confidence !== 'possible').map((i) => i.path));
}

export function useInstalledApps(active: boolean) {
  const [apps, setApps] = useState<InstalledApp[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [plan, setPlan] = useState<AppPlan | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [selection, setSelection] = useState<Set<string>>(new Set());
  const [removing, setRemoving] = useState(false);
  const [report, setReport] = useState<UninstallReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<'app' | 'orphans'>('app');
  const [orphans, setOrphans] = useState<OrphanReport | null>(null);
  const [orphansLoading, setOrphansLoading] = useState(false);
  const requestId = useRef(0);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setApps(await invoke<InstalledApp[]>('list_installed_apps'));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (active && apps === null && !loading) void refresh();
  }, [active, apps, loading, refresh]);

  const loadOrphans = useCallback(async () => {
    setOrphansLoading(true);
    try {
      setOrphans(await invoke<OrphanReport>('list_orphans'));
    } catch (cause) {
      setError(String(cause));
    } finally {
      setOrphansLoading(false);
    }
  }, []);

  useEffect(() => {
    if (active && orphans === null && !orphansLoading) void loadOrphans();
  }, [active, orphans, orphansLoading, loadOrphans]);

  /** Shows leftovers of apps that are no longer installed. Nothing is preselected. */
  const showOrphans = useCallback(() => {
    requestId.current += 1;
    setView('orphans');
    setPlan(null);
    setAnalyzing(false);
    setReport(null);
    setError(null);
    setSelection(new Set());
    void loadOrphans();
  }, [loadOrphans]);

  const select = useCallback(async (path: string) => {
    const id = ++requestId.current;
    setView('app');
    setAnalyzing(true);
    setReport(null);
    setError(null);
    setPlan(null);
    try {
      const next = await invoke<AppPlan>('analyze_app', { path });
      if (id !== requestId.current) return;
      setPlan(next);
      setSelection(defaultSelection(next));
    } catch (cause) {
      if (id === requestId.current) setError(String(cause));
    } finally {
      if (id === requestId.current) setAnalyzing(false);
    }
  }, []);

  const toggle = useCallback((path: string) => {
    setSelection((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const uninstall = useCallback(async () => {
    if (!plan) return;
    setRemoving(true);
    setError(null);
    try {
      const result = await invoke<UninstallReport>('uninstall_app', {
        path: plan.path,
        items: plan.items.filter((i) => selection.has(i.path)).map((i) => i.path),
        // Receipts are forgotten only when the app itself is removed.
        receipts: selection.has(plan.path) ? plan.receipts : [],
      });
      setReport(result);
      setPlan(null);
      setSelection(new Set());
      void refresh();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setRemoving(false);
    }
  }, [plan, refresh, selection]);

  const removeOrphans = useCallback(async () => {
    if (selection.size === 0) return;
    setRemoving(true);
    setError(null);
    try {
      const result = await invoke<UninstallReport>('remove_orphans', { items: [...selection] });
      setReport(result);
      setSelection(new Set());
      void loadOrphans();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setRemoving(false);
    }
  }, [loadOrphans, selection]);

  return {
    view,
    orphans,
    orphansLoading,
    showOrphans,
    removeOrphans,
    apps,
    loading,
    refresh,
    plan,
    analyzing,
    select,
    selection,
    setSelection,
    toggle,
    removing,
    uninstall,
    report,
    error,
  };
}
