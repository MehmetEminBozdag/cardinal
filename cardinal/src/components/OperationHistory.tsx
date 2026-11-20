import React, { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { formatBytes } from '../utils/folderSize';

export type LoggedItem = {
  path: string;
  trashedTo?: string;
  size?: number;
  admin: boolean;
  ok: boolean;
  error?: string;
};

export type OperationLogEntry = {
  time: number;
  source: 'contextMenu' | 'appUninstall' | 'orphans';
  app?: string;
  items: LoggedItem[];
  receipts?: string[];
};

const LOGGED_EVENT = 'file-operation-logged';
const LIMIT = 50;

function baseName(path: string): string {
  return path.split('/').filter(Boolean).pop() ?? path;
}

/**
 * Persistent history of Move to Trash operations (all tabs), read from the
 * native operation log. Refreshes whenever a new operation is logged.
 */
export function OperationHistory(): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const [entries, setEntries] = useState<OperationLogEntry[] | null>(null);

  const load = useCallback(async () => {
    try {
      setEntries(await invoke<OperationLogEntry[]>('get_operation_log', { limit: LIMIT }));
    } catch {
      setEntries([]);
    }
  }, []);

  useEffect(() => {
    void load();
    let unlisten: (() => void) | undefined;
    let disposed = false;
    listen(LOGGED_EVENT, () => void load())
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [load]);

  const revealLog = useCallback(async () => {
    try {
      const path = await invoke<string | null>('get_operation_log_path');
      if (path) await invoke('open_in_finder', { path });
    } catch {
      // The log file does not exist yet.
    }
  }, []);

  const locale = i18n?.language;

  return (
    <section className="operation-history" aria-label={t('workspace.operations.history.title')}>
      <h4>{t('workspace.operations.history.title')}</h4>
      {entries && entries.length === 0 ? (
        <p className="workspace-note">{t('workspace.operations.history.empty')}</p>
      ) : null}
      {(entries ?? []).map((entry, index) => {
        const failed = entry.items.filter((i) => !i.ok).length;
        const size = entry.items.reduce((sum, i) => sum + (i.ok ? (i.size ?? 0) : 0), 0);
        const admin = entry.items.some((i) => i.admin);
        return (
          <details key={`${entry.time}-${index}`} className="operation-history__entry">
            <summary>
              <span className="operation-history__time">
                {new Date(entry.time * 1000).toLocaleString(locale)}
              </span>
              <span>
                {t(`workspace.operations.history.sources.${entry.source}`)}
                {entry.app ? ` · ${entry.app}` : ''}
              </span>
              <span className="operation-history__meta">
                {t('workspace.operations.history.items', { count: entry.items.length })}
                {size > 0 ? ` · ${formatBytes(size, locale)}` : ''}
                {failed > 0
                  ? ` · ${t('workspace.operations.history.failed', { count: failed })}`
                  : ''}
                {admin ? ' · 🔑' : ''}
              </span>
            </summary>
            <ul>
              {entry.items.map((item) => (
                <li key={item.path} className={item.ok ? undefined : 'is-failed'}>
                  <span className="operation-history__path" title={item.path}>
                    {item.path}
                  </span>
                  {item.size ? (
                    <span className="operation-history__size">
                      {formatBytes(item.size, locale)}
                    </span>
                  ) : null}
                  {item.trashedTo ? (
                    <span className="operation-history__dest">
                      {t('workspace.operations.history.toTrash', {
                        name: baseName(item.trashedTo),
                      })}
                    </span>
                  ) : null}
                  {item.error ? (
                    <span className="operation-history__error">{item.error}</span>
                  ) : null}
                </li>
              ))}
              {entry.receipts?.length ? (
                <li>
                  {t('workspace.operations.history.receipts', { count: entry.receipts.length })}
                </li>
              ) : null}
            </ul>
          </details>
        );
      })}
      <button type="button" className="workspace-secondary-action" onClick={() => void revealLog()}>
        {t('workspace.operations.history.reveal')}
      </button>
    </section>
  );
}
