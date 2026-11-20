import React, { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import type { Leftover, LeftoverKind, useInstalledApps } from '../hooks/useInstalledApps';
import { defaultSelection } from '../hooks/useInstalledApps';
import { formatBytes, tildePath } from '../utils/folderSize';
import { confirmNative } from '../utils/nativeConfirm';

type InstalledApps = ReturnType<typeof useInstalledApps>;

type AppsPanelProps = {
  apps: InstalledApps;
  filterQuery: string;
  home: string | null;
};

const KIND_ORDER: LeftoverKind[] = [
  'appBundle',
  'support',
  'cache',
  'preferences',
  'container',
  'savedState',
  'logs',
  'webData',
  'launchItem',
  'helper',
  'receipt',
  'recentDocuments',
  'other',
];

export function AppsPanel({ apps, filterQuery, home }: AppsPanelProps): React.JSX.Element {
  const { t, i18n } = useTranslation();
  const locale = i18n.language;
  const bytes = useCallback((value: number) => formatBytes(value, locale), [locale]);
  const { plan, selection } = apps;

  const visibleApps = useMemo(() => {
    const query = filterQuery.trim().toLowerCase();
    const list = apps.apps ?? [];
    return query
      ? list.filter(
          (a) => a.name.toLowerCase().includes(query) || a.bundleId.toLowerCase().includes(query),
        )
      : list;
  }, [apps.apps, filterQuery]);

  const groups = useMemo(() => {
    const byKind = new Map<LeftoverKind, Leftover[]>();
    for (const item of plan?.items ?? []) {
      byKind.set(item.kind, [...(byKind.get(item.kind) ?? []), item]);
    }
    return KIND_ORDER.filter((k) => byKind.has(k)).map((k) => [k, byKind.get(k)!] as const);
  }, [plan]);

  const chosen = (plan?.items ?? []).filter((i) => selection.has(i.path));
  const chosenSize = chosen.reduce((sum, i) => sum + i.size, 0);
  const adminCount = chosen.filter((i) => i.needsAdmin).length;
  const leftoverCount = chosen.filter((i) => i.kind !== 'appBundle').length;

  const confirmAndRemove = useCallback(() => {
    if (!plan || chosen.length === 0) return;
    // Defer so a native menu or focus change cannot swallow the dialog.
    window.setTimeout(async () => {
      const lines = chosen
        .slice(0, 12)
        .map((i) => `• ${tildePath(i.path, home)} — ${bytes(i.size)}`)
        .join('\n');
      const more =
        chosen.length > 12 ? `\n${t('apps.confirm.more', { count: chosen.length - 12 })}` : '';
      const admin = adminCount > 0 ? `\n\n${t('apps.confirm.admin', { count: adminCount })}` : '';
      const confirmed = await confirmNative({
        message: t('apps.confirm.title', {
          name: plan.name,
          count: chosen.length,
          size: bytes(chosenSize),
        }),
        detail: `${lines}${more}${admin}\n\n${t('apps.confirm.trashNote')}`,
        confirmLabel: t('dialog.moveToTrash'),
        cancelLabel: t('dialog.cancel'),
        destructive: true,
      });
      if (confirmed) void apps.uninstall();
    }, 0);
  }, [adminCount, apps, bytes, chosen, chosenSize, home, plan, t]);

  const report = apps.report;
  const orphanItems = (apps.orphans?.groups ?? []).flatMap((g) => g.items);
  const orphanChosen = orphanItems.filter((i) => selection.has(i.path));
  const orphanChosenSize = orphanChosen.reduce((sum, i) => sum + i.size, 0);
  const orphanAdmin = orphanChosen.filter((i) => i.needsAdmin).length;

  const confirmAndRemoveOrphans = useCallback(() => {
    if (orphanChosen.length === 0) return;
    window.setTimeout(async () => {
      const lines = orphanChosen
        .slice(0, 12)
        .map((i) => `• ${tildePath(i.path, home)} — ${bytes(i.size)}`)
        .join('\n');
      const more =
        orphanChosen.length > 12
          ? `\n${t('apps.confirm.more', { count: orphanChosen.length - 12 })}`
          : '';
      const admin = orphanAdmin > 0 ? `\n\n${t('apps.confirm.admin', { count: orphanAdmin })}` : '';
      const confirmed = await confirmNative({
        message: t('apps.orphans.confirmTitle', {
          count: orphanChosen.length,
          size: bytes(orphanChosenSize),
        }),
        detail: `${lines}${more}${admin}\n\n${t('apps.confirm.trashNote')}`,
        confirmLabel: t('dialog.moveToTrash'),
        cancelLabel: t('dialog.cancel'),
        destructive: true,
      });
      if (confirmed) void apps.removeOrphans();
    }, 0);
  }, [apps, bytes, home, orphanAdmin, orphanChosen, orphanChosenSize, t]);

  const renderItem = (item: Leftover, tag: React.ReactNode) => (
    <label key={item.path} className={`apps-item is-${item.confidence}`}>
      <input
        type="checkbox"
        checked={selection.has(item.path)}
        onChange={() => apps.toggle(item.path)}
      />
      <span className="apps-item__path" title={item.path}>
        {tildePath(item.path, home)}
      </span>
      {tag}
      {item.needsAdmin ? (
        <span className="apps-tag is-admin" title={t('apps.adminHint')}>
          🔑
        </span>
      ) : null}
      <span className="sizes-num">{bytes(item.size)}</span>
      <button
        type="button"
        className="apps-reveal"
        title={t('apps.reveal')}
        aria-label={t('apps.reveal')}
        onClick={(event) => {
          event.preventDefault();
          void invoke('open_in_finder', { path: item.path });
        }}
      >
        ↗
      </button>
    </label>
  );

  return (
    <div className="apps-panel">
      <div className="apps-list" role="listbox" aria-label={t('apps.listLabel')}>
        <div className="apps-list__header">
          <span>
            {apps.apps
              ? t('apps.count', {
                  count: apps.apps.length,
                  formatted: apps.apps.length.toLocaleString(locale),
                })
              : t('apps.loading')}
          </span>
          <button type="button" onClick={() => void apps.refresh()} disabled={apps.loading}>
            ↻
          </button>
        </div>
        <button
          type="button"
          role="option"
          aria-selected={apps.view === 'orphans'}
          className={`apps-row apps-row--orphans${apps.view === 'orphans' ? ' is-active' : ''}`}
          onClick={apps.showOrphans}
        >
          <span className="apps-row__name">{t('apps.orphans.title')}</span>
          <span className="apps-row__size">{apps.orphans ? bytes(apps.orphans.total) : '…'}</span>
          <span className="apps-row__meta">
            {apps.orphans
              ? t('apps.orphans.subtitle', { count: apps.orphans.groups.length })
              : t('apps.orphans.scanning')}
          </span>
        </button>
        {visibleApps.map((app) => {
          const active = plan?.path === app.path;
          return (
            <button
              key={app.path}
              type="button"
              role="option"
              aria-selected={active}
              className={`apps-row${active ? ' is-active' : ''}`}
              onClick={() => void apps.select(app.path)}
            >
              <span className="apps-row__name">
                {app.name}
                {app.running ? <span className="apps-badge">{t('apps.running')}</span> : null}
              </span>
              <span className="apps-row__size">{bytes(app.appSize + app.leftoverSize)}</span>
              <span className="apps-row__meta">
                {app.version ? `${app.version} · ` : ''}
                {app.leftoverSize > 0
                  ? t('apps.leftoverEstimate', { size: bytes(app.leftoverSize) })
                  : app.bundleId}
              </span>
            </button>
          );
        })}
      </div>

      <div className="apps-detail">
        {apps.error ? (
          <div className="apps-message is-error">
            {apps.error.includes('app_running') ? t('apps.errors.running') : apps.error}
          </div>
        ) : null}
        {report ? (
          <div className={`apps-message${report.failed.length ? ' is-error' : ' is-ok'}`}>
            {t('apps.report.moved', { count: report.moved.length })}
            {report.receiptsForgotten.length
              ? ` ${t('apps.report.receipts', { count: report.receiptsForgotten.length })}`
              : ''}
            {report.adminCancelled ? ` ${t('apps.report.adminCancelled')}` : ''}
            {report.failed.length ? (
              <ul>
                {report.failed.map((f) => (
                  <li key={f.path}>{tildePath(f.path, home)}</li>
                ))}
              </ul>
            ) : null}
            <div className="sizes-muted">{t('apps.report.emptyTrash')}</div>
          </div>
        ) : null}

        {apps.view === 'orphans' ? (
          apps.orphansLoading && !apps.orphans ? (
            <div className="sizes-empty">{t('apps.orphans.scanning')}</div>
          ) : !apps.orphans || apps.orphans.groups.length === 0 ? (
            <div className="sizes-empty">{t('apps.orphans.empty')}</div>
          ) : (
            <>
              <div className="apps-detail__header">
                <div>
                  <h3>{t('apps.orphans.title')}</h3>
                  <div className="sizes-muted">{t('apps.orphans.note')}</div>
                </div>
              </div>
              <div className="apps-items">
                {apps.orphans.groups.map((group) => (
                  <section key={group.key} className="apps-group">
                    <h4 className="apps-group__title">
                      <span>{group.label}</span>
                      <span className="sizes-num">{bytes(group.size)}</span>
                      <button
                        type="button"
                        className="apps-group__select"
                        onClick={() =>
                          apps.setSelection(
                            (prev) => new Set([...prev, ...group.items.map((i) => i.path)]),
                          )
                        }
                      >
                        {t('apps.orphans.selectGroup')}
                      </button>
                    </h4>
                    {group.items.map((item) =>
                      renderItem(
                        item,
                        <span className="apps-tag is-possible">
                          {t(`apps.kinds.${item.kind}`)}
                        </span>,
                      ),
                    )}
                  </section>
                ))}
              </div>
              <div className="apps-footer">
                <span>
                  {t('apps.orphans.selected', {
                    count: orphanChosen.length,
                    size: bytes(orphanChosenSize),
                  })}
                  {orphanAdmin > 0 ? ` · 🔑 ${t('apps.adminCount', { count: orphanAdmin })}` : ''}
                </span>
                <button
                  type="button"
                  className="apps-remove"
                  disabled={orphanChosen.length === 0 || apps.removing}
                  onClick={confirmAndRemoveOrphans}
                >
                  {apps.removing
                    ? t('apps.removing')
                    : t('apps.remove', { size: bytes(orphanChosenSize) })}
                </button>
              </div>
            </>
          )
        ) : apps.analyzing ? (
          <div className="sizes-empty">{t('apps.analyzing')}</div>
        ) : !plan ? (
          report ? null : (
            <div className="sizes-empty">{t('apps.pick')}</div>
          )
        ) : (
          <>
            <div className="apps-detail__header">
              <div>
                <h3>{plan.name}</h3>
                <div className="sizes-muted">
                  {plan.bundleId}
                  {plan.version ? ` · ${plan.version}` : ''}
                </div>
              </div>
              <div className="apps-detail__actions">
                <button type="button" onClick={() => apps.setSelection(defaultSelection(plan))}>
                  {t('apps.selectRecommended')}
                </button>
                <button
                  type="button"
                  onClick={() => apps.setSelection(new Set(plan.items.map((i) => i.path)))}
                >
                  {t('apps.selectAll')}
                </button>
              </div>
            </div>
            {plan.running ? (
              <div className="apps-message is-error">{t('apps.errors.running')}</div>
            ) : null}
            <div className="apps-items">
              {groups.map(([kind, items]) => (
                <section key={kind} className="apps-group">
                  <h4>{t(`apps.kinds.${kind}`)}</h4>
                  {items.map((item) =>
                    renderItem(
                      item,
                      <span className={`apps-tag is-${item.confidence}`}>
                        {t(`apps.confidence.${item.confidence}`)}
                      </span>,
                    ),
                  )}
                </section>
              ))}
              {plan.receipts.length ? (
                <section className="apps-group">
                  <h4>{t('apps.receiptsTitle')}</h4>
                  {plan.receipts.map((pkg) => (
                    <div key={pkg} className="apps-item is-certain">
                      <span className="apps-item__path">{pkg}</span>
                      <span className="apps-tag is-admin">🔑</span>
                    </div>
                  ))}
                </section>
              ) : null}
            </div>
            <div className="apps-footer">
              <span>
                {t('apps.selected', {
                  count: chosen.length,
                  leftovers: leftoverCount,
                  size: bytes(chosenSize),
                })}
                {adminCount > 0 ? ` · 🔑 ${t('apps.adminCount', { count: adminCount })}` : ''}
              </span>
              <button
                type="button"
                className="apps-remove"
                disabled={chosen.length === 0 || apps.removing || plan.running}
                onClick={confirmAndRemove}
              >
                {apps.removing ? t('apps.removing') : t('apps.remove', { size: bytes(chosenSize) })}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
